use anyhow::Result;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use crate::database::Database;
use crate::config::Config;
use crate::github::{GitHubClient, RepositoryService};
use crate::docker::{DockerClient, ImageService, StackService};
use crate::stack_processor::StackProcessor;

pub struct Commands {
    db: Arc<Database>,
    repo_service: RepositoryService,
    image_service: Arc<ImageService>,
    stack_service: Arc<StackService>,
    config: Config,
}

impl Commands {
    pub async fn new(db: Database) -> Result<Self> {
        let config = Config::from_env();
        let github_client = GitHubClient::new()?;
        let docker_client = DockerClient::new().await?;
        let repo_service = RepositoryService::new(github_client);
        let image_service = Arc::new(ImageService::new(docker_client, config.image_pull_policy));
        
        // Create a new Docker client for stack service
        let docker_client_for_stacks = DockerClient::new().await?;
        let stack_service = Arc::new(StackService::new(docker_client_for_stacks));
        
        Ok(Self {
            db: Arc::new(db),
            repo_service,
            image_service,
            stack_service,
            config,
        })
    }

    /// Returns true if the repo was already in cache (caller may ignore).
    pub async fn watch_or_skip_if_cached(&self, github_url: &str) -> Result<bool> {
        if let Some(cached_repo) = self.db.get_repository_from_cache(github_url).await? {
            println!("Repository '{}' is already being watched (last watch: {}), skipping.", github_url, cached_repo.last_watch);
            return Ok(true);
        }
        self.watch(github_url).await?;
        Ok(false)
    }

    pub async fn watch(&self, github_url: &str) -> Result<()> {
        println!("Watching GitHub repository: {}", github_url);
        
        // Check if repository is already in cache
        if let Some(cached_repo) = self.db.get_repository_from_cache(github_url).await? {
            return Err(anyhow::anyhow!("Repository '{}' is already being watched (last watch: {})", 
                github_url, cached_repo.last_watch));
        }
        
        // Clone the repository
        let repo_path = self.repo_service.clone_repository(github_url).await?;
        println!("Repository cloned to: {}", repo_path);
        
        // Process stacks and deploy them
        let stack_processor = StackProcessor::new(
            Arc::clone(&self.db),
            Arc::clone(&self.image_service),
            Arc::clone(&self.stack_service),
        );
        stack_processor.process_and_deploy_stacks(&repo_path, github_url, false, false).await?;
        
        // Process images: pull according to policy, remove unused
        self.process_images().await?;
        
        // Add repository to cache
        self.db.add_repository_to_cache(github_url).await?;
        println!("Repository added to cache");
        
        // Clean up cloned repository
        if let Err(e) = fs::remove_dir_all(&repo_path) {
            println!("Warning: Could not clean up repository directory: {}", e);
        }
        
        Ok(())
    }

    pub async fn reconcile(&self, force: bool) -> Result<()> {
        println!("Reconciling database...");
        
        // Check if there are any repositories in cache
        let repositories = self.db.get_all_repositories().await?;
        if repositories.is_empty() {
            return Err(anyhow::anyhow!("No repositories found in cache. Please run 'watch' command first."));
        }
        
        println!("Found {} repositories in cache:", repositories.len());
        for repo in &repositories {
            println!("  - {} (last watch: {})", repo.url, repo.last_watch);
        }
        
        // Get all stacks and display them
        let stacks = self.db.get_all_stacks().await?;
        println!("\nFound {} stacks in database:", stacks.len());
        
        for stack in &stacks {
            println!("  - {} (status: {}, hash: {})", stack.name, stack.status, stack.hash);
        }
        
        // Get all images and display them
        let images = self.db.get_all_images().await?;
        println!("\nFound {} images in database:", images.len());
        
        for image in &images {
            println!("  - {} (referenced {} times)", image.name, image.reference_count);
        }
        
        // Now reconcile each repository
        println!("\nStarting reconciliation process...");
        if force {
            println!("⚠️  Force mode enabled - will redeploy all stacks regardless of changes");
        }
        for repo in &repositories {
            println!("Reconciling repository: {}", repo.url);
            
            // Clone the repository
            let repo_path = self.repo_service.clone_repository(&repo.url).await?;
            println!("Repository cloned to: {}", repo_path);
            
            // Process stacks and deploy them (with is_reconcile=true and force flag)
            let stack_processor = StackProcessor::new(
                Arc::clone(&self.db),
                Arc::clone(&self.image_service),
                Arc::clone(&self.stack_service),
            );
            stack_processor.process_and_deploy_stacks(&repo_path, &repo.url, true, force).await?;
            
            // Process images: pull according to policy, remove unused
            self.process_images().await?;
            
            // Clean up cloned repository
            if let Err(e) = fs::remove_dir_all(&repo_path) {
                println!("Warning: Could not clean up repository directory: {}", e);
            }
        }
        
        println!("Reconciliation completed!");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        println!("Stopping DockerOps and cleaning up all resources...");
        
        // Get all stacks from database
        let stacks = self.db.get_all_stacks().await?;
        println!("Found {} stacks to remove", stacks.len());
        
        // Remove all stacks
        for stack in &stacks {
            println!("Removing stack: {}", stack.name);
            self.stack_service.stop_stack(&stack.name).await?;
        }
        
        // Get all images from database
        let images = self.db.get_all_images().await?;
        println!("Found {} images to remove", images.len());
        
        // Remove all images
        for image in &images {
            println!("Removing image: {}", image.name);
            self.image_service.remove_image(&image.name).await?;
        }
        
        // Clean up database
        println!("Cleaning up database...");
        self.db.delete_all_stacks().await?;
        self.db.reset_image_reference_counts().await?;
        self.db.delete_images_with_zero_count().await?;
        self.db.clear_repository_cache().await?;
        
        // Verify cache is cleared
        let repositories = self.db.get_all_repositories().await?;
        if !repositories.is_empty() {
            println!("Warning: Repository cache still contains {} entries, forcing cleanup...", repositories.len());
            self.db.clear_repository_cache().await?;
            
            // Verify again after forced cleanup
            let repositories_after = self.db.get_all_repositories().await?;
            if !repositories_after.is_empty() {
                println!("❌ Cache cleanup failed! Still contains {} entries", repositories_after.len());
                for repo in &repositories_after {
                    println!("  - {}", repo.url);
                }
            } else {
                println!("✅ Cache successfully cleared");
            }
        }
        
        println!("All stacks and images have been removed.");
        println!("Database connection will be closed.");
        Ok(())
    }

    pub fn show_version() {
        println!("DockerOps CLI v{}", env!("CARGO_PKG_VERSION"));
        println!("A Docker Swarm stack manager for GitHub repositories");
        println!("Repository: https://github.com/TomBedinoVT/DockerOps");
    }

    /// Daemon mode: optionally seed repos from DOCKEROPS_REPOS, then reconcile in a loop every interval_secs.
    pub async fn run_daemon(&self, repo_urls: &[String], interval_secs: u64) -> Result<()> {
        for url in repo_urls {
            let url = url.trim();
            if url.is_empty() {
                continue;
            }
            if let Err(e) = self.watch_or_skip_if_cached(url).await {
                eprintln!("Warning: failed to watch '{}': {}", url, e);
            }
        }

        let duration = Duration::from_secs(interval_secs);
        loop {
            sleep(duration).await;
            println!("[daemon] Running reconcile (interval {}s)...", interval_secs);
            if let Err(e) = self.reconcile(false).await {
                eprintln!("[daemon] Reconcile error: {}", e);
            }
        }
    }

    pub async fn debug_cache(&self) -> Result<()> {
        println!("Debug: Checking repository cache...");
        
        let repositories = self.db.get_all_repositories().await?;
        println!("Found {} repositories in cache:", repositories.len());
        
        for repo in &repositories {
            println!("  - {} (last watch: {})", repo.url, repo.last_watch);
        }
        
        Ok(())
    }

    async fn process_images(&self) -> Result<()> {
        // Get all images from database
        let images = self.db.get_all_images().await?;
        println!("  Found {} images in database", images.len());
        
        for image in &images {
            if image.reference_count == 0 {
                // Remove unused images
                println!("  Removing unused image: {}", image.name);
                self.image_service.remove_image(&image.name).await?;
            } else {
                // Pull image according to policy (Always or IfNotPresent)
                println!("  Processing image: {} (referenced {} times)", image.name, image.reference_count);
                self.image_service.pull_image(&image.name).await?;
            }
        }
        
        // Remove images with zero count from database
        self.db.delete_images_with_zero_count().await?;
        
        Ok(())
    }
}
