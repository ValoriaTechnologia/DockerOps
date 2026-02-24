use anyhow::Result;
use git2::{FetchOptions, RemoteCallbacks, build::RepoBuilder};
use std::path::Path;
use crate::github::client::GitHubClient;

/// Service pour gérer les opérations sur les repositories GitHub
pub struct RepositoryService {
    client: GitHubClient,
}

impl RepositoryService {
    /// Crée un nouveau service de repository
    pub fn new(client: GitHubClient) -> Self {
        Self { client }
    }

    /// Clone un repository GitHub dans un répertoire temporaire
    pub async fn clone_repository(&self, github_url: &str) -> Result<String> {
        // Convert GitHub URL to clone URL if needed
        let clone_url = if github_url.starts_with("https://github.com/") {
            github_url.to_string()
        } else if github_url.starts_with("github.com/") {
            format!("https://{}", github_url)
        } else {
            github_url.to_string()
        };

        // Create temporary directory for cloning
        let temp_dir = if cfg!(windows) {
            format!("{}\\temp_repo_{}", std::env::var("TEMP").unwrap_or_else(|_| "C:\\temp".to_string()), chrono::Utc::now().timestamp())
        } else {
            format!("/tmp/temp_repo_{}", chrono::Utc::now().timestamp())
        };
        let repo_path = Path::new(&temp_dir);

        println!("Cloning repository from: {}", clone_url);

        // Setup authentication if token is available
        let mut callbacks = RemoteCallbacks::new();

        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            println!("Using GitHub token for authentication");
            let token_clone = token.clone();
            callbacks.credentials(move |_url, username_from_url, _allowed_types| {
                git2::Cred::userpass_plaintext(username_from_url.unwrap_or("git"), &token_clone)
            });
        } else {
            println!("No GitHub token found. Trying to clone without authentication...");
            println!("If this fails, set the GITHUB_TOKEN environment variable");
        }

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let mut builder = RepoBuilder::new();
        builder.fetch_options(fetch_options);

        let _repo = builder.clone(&clone_url, repo_path)
            .map_err(|e| anyhow::anyhow!("Failed to clone repository: {}", e))?;

        Ok(temp_dir)
    }

    /// Vérifie si un repository existe et est accessible via l'API GitHub
    pub async fn check_repository(&self, github_url: &str) -> Result<bool> {
        // Parse GitHub URL to extract owner and repo
        let (owner, repo) = self.parse_github_url(github_url)?;
        
        // Try to get repository info via API
        match self.client.octocrab().repos(owner, repo).get().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Parse une URL GitHub pour extraire owner et repo
    fn parse_github_url(&self, url: &str) -> Result<(String, String)> {
        // Remove https:// or http://
        let url = url.trim_start_matches("https://").trim_start_matches("http://");
        // Remove github.com/
        let url = url.trim_start_matches("github.com/");
        // Remove .git suffix if present
        let url = url.trim_end_matches(".git");
        
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() >= 2 {
            Ok((parts[0].to_string(), parts[1].to_string()))
        } else {
            Err(anyhow::anyhow!("Invalid GitHub URL format: {}", url))
        }
    }
}

