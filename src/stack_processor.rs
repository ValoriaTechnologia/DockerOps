use anyhow::Result;
use std::path::Path;
use std::fs;
use std::sync::Arc;
use md5;
use crate::database::Database;
use crate::models::{Stack, StackDefinition};
use crate::compose_processor::ComposeProcessor;
use crate::volume_processor::VolumeProcessor;
use crate::secret_processor::SecretProcessor;
use crate::docker::{ImageService, StackService};

/// Processeur pour gérer les stacks Docker
pub struct StackProcessor {
    db: Arc<Database>,
    image_service: Arc<ImageService>,
    stack_service: Arc<StackService>,
}

impl StackProcessor {
    /// Crée un nouveau processeur de stacks
    pub fn new(db: Arc<Database>, image_service: Arc<ImageService>, stack_service: Arc<StackService>) -> Self {
        Self {
            db,
            image_service,
            stack_service,
        }
    }

    /// Traite et déploie les stacks depuis un répertoire
    pub async fn process_and_deploy_stacks(
        &self,
        repo_path: &str,
        repository_url: &str,
        is_reconcile: bool,
        force: bool,
    ) -> Result<()> {
        // Reset image reference counts at the beginning
        self.db.reset_image_reference_counts().await?;

        // Look for stacks.yaml file
        let stacks_file_path = Path::new(repo_path).join("stacks.yaml");
        if !stacks_file_path.exists() {
            return Err(anyhow::anyhow!("stacks.yaml not found in repository"));
        }

        // Read and parse stacks.yaml
        let stacks_content = fs::read_to_string(&stacks_file_path)?;
        let stacks_definitions: Vec<StackDefinition> = serde_yaml::from_str(&stacks_content)?;

        // Process volumes configuration
        let mut volumes_definitions = VolumeProcessor::load_volumes_config(repo_path)?;
        
        // Load NFS config if volumes exist
        let nfs_config = if volumes_definitions.is_some() {
            Some(SecretProcessor::load_nfs_config(repo_path)?)
        } else {
            None
        };

        // Process volumes if they exist
        if let Some(ref mut volumes_defs) = volumes_definitions {
            if let Some(ref nfs_config) = nfs_config {
                VolumeProcessor::process_volumes(volumes_defs, Some(nfs_config), repo_path).await?;
            }
        }

        // Process each stack
        for stack_def in &stacks_definitions {
            self.process_stack(
                stack_def,
                repo_path,
                repository_url,
                is_reconcile,
                force,
                volumes_definitions.as_deref(),
                nfs_config.as_ref(),
            ).await?;
        }

        Ok(())
    }

    /// Traite un stack individuel
    async fn process_stack(
        &self,
        stack_def: &StackDefinition,
        repo_path: &str,
        repository_url: &str,
        is_reconcile: bool,
        force: bool,
        volumes_definitions: Option<&[crate::models::VolumeDefinition]>,
        nfs_config: Option<&crate::models::NfsConfig>,
    ) -> Result<()> {
        // Look for the stack directory
        let stack_dir = Path::new(repo_path).join(&stack_def.name);
        if !stack_dir.exists() || !stack_dir.is_dir() {
            eprintln!("Warning: Stack directory '{}' not found", stack_def.name);
            return Ok(());
        }

        // Look for docker-compose file in the stack directory
        let compose_files = vec![
            stack_dir.join("docker-compose.yml"),
            stack_dir.join("docker-compose.yaml"),
            stack_dir.join("compose.yml"),
            stack_dir.join("compose.yaml"),
        ];

        let compose_file_path = compose_files.iter()
            .find(|f| f.exists())
            .ok_or_else(|| anyhow::anyhow!("No docker-compose file found in stack directory '{}'", stack_def.name))?;

        let mut compose_content = fs::read_to_string(compose_file_path)?;

        // Process volumes in compose file if volumes definitions exist
        if let Some(volumes_defs) = volumes_definitions {
            if let Some(nfs_config) = nfs_config {
                compose_content = ComposeProcessor::process_volumes(&compose_content, volumes_defs, nfs_config)?;
            }
        }

        // Process secrets: read secrets.yaml (declaration only), generate entrypoint-secrets.sh, inject into compose
        if let Some(secret_defs) = SecretProcessor::process_secrets(&stack_dir)? {
            let entrypoint_volume = "./entrypoint-secrets.sh:/run/entrypoint-secrets.sh:ro";
            compose_content = ComposeProcessor::process_secrets(&compose_content, &secret_defs, entrypoint_volume)?;
        }

        // Write the modified compose content back to the file
        fs::write(compose_file_path, &compose_content)?;

        let compose_hash = Self::calculate_md5(&compose_content);

        // Calculate relative path for database
        let relative_compose_path = compose_file_path
            .strip_prefix(repo_path)
            .unwrap_or(compose_file_path)
            .to_string_lossy()
            .replace('\\', "/")
            .to_string();

        // Check if stack exists in database
        if let Some(existing_stack) = self.db.get_stack_by_name(&stack_def.name, repository_url).await? {
            let has_changed = existing_stack.hash != compose_hash;
            let should_deploy = has_changed || force;

            if should_deploy {
                if is_reconcile {
                    // For reconcile, stop the existing stack first
                    self.stack_service.stop_stack(&stack_def.name).await?;
                }

                // Update stack in database
                self.db.update_stack_hash(&stack_def.name, repository_url, &compose_hash).await?;

                // Deploy the updated stack
                self.deploy_stack(&stack_def.name, compose_file_path).await?;
                self.db.update_stack_status(&stack_def.name, repository_url, "deployed").await?;
            }
        } else {
            // New stack
            let stack = Stack::new(
                stack_def.name.clone(),
                repository_url.to_string(),
                relative_compose_path.clone(),
                compose_hash.clone(),
            );
            self.db.create_stack(&stack).await?;

            // Deploy the new stack
            self.deploy_stack(&stack_def.name, compose_file_path).await?;
            self.db.update_stack_status(&stack_def.name, repository_url, "deployed").await?;
        }

        // Process compose file for image extraction
        self.process_yaml_file(&compose_content, &relative_compose_path).await?;

        Ok(())
    }

    /// Déploie un stack
    async fn deploy_stack(&self, stack_name: &str, compose_path: &Path) -> Result<()> {
        // Read compose file to extract images
        let compose_content = fs::read_to_string(compose_path)?;

        // Extract and pull images before deployment
        let images_found = ComposeProcessor::extract_images(&compose_content)?;

        if !images_found.is_empty() {
            for image_name in &images_found {
                self.image_service.pull_image(image_name).await?;
            }
        }

        // Deploy the stack using Docker client (no secret values; secrets are Swarm-native)
        self.stack_service.deploy_stack(stack_name, compose_path).await?;

        Ok(())
    }

    /// Traite un fichier YAML pour extraire les images et mettre à jour la base de données
    async fn process_yaml_file(&self, content: &str, file_path: &str) -> Result<()> {

        // Extract images from YAML structure
        let images_found = ComposeProcessor::extract_images(content)?;

        // Update database with found images
        for image_name in &images_found {
            self.update_image_reference(image_name).await?;
        }

        if !images_found.is_empty() {
            println!("  Found {} images in {}: {:?}", images_found.len(), file_path, images_found);
        }

        Ok(())
    }

    /// Met à jour la référence d'une image dans la base de données
    async fn update_image_reference(&self, image_name: &str) -> Result<()> {
        use crate::models::Image;

        // Try to get existing image
        if let Some(existing_image) = self.db.get_image_by_name(image_name).await? {
            // Increment reference count
            let new_count = existing_image.reference_count + 1;
            self.db.update_image_reference_count(image_name, new_count).await?;
        } else {
            // Create new image with reference count 1
            let new_image = Image::new(image_name.to_string(), 1);
            self.db.create_image(&new_image).await?;
        }

        Ok(())
    }

    /// Calcule le hash MD5 d'un contenu
    fn calculate_md5(content: &str) -> String {
        let result = md5::compute(content.as_bytes());
        format!("{:x}", result)
    }
}

