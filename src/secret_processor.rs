use anyhow::Result;
use std::path::Path;
use std::fs;
use crate::models::{SecretDefinition, NfsConfig};

/// Processeur pour gérer les secrets
pub struct SecretProcessor;

impl SecretProcessor {
    /// Charge la configuration NFS depuis nfs.yaml
    pub fn load_nfs_config(repo_path: &str) -> Result<NfsConfig> {
        let nfs_file_path = Path::new(repo_path).join("nfs.yaml");
        if !nfs_file_path.exists() {
            return Err(anyhow::anyhow!("nfs.yaml not found at: {}", nfs_file_path.display()));
        }

        let nfs_content = fs::read_to_string(&nfs_file_path)?;
        let config = serde_yaml::from_str::<NfsConfig>(&nfs_content)?;

        Ok(config)
    }

    /// Traite les secrets depuis un fichier secrets.yaml
    pub fn process_secrets(stack_dir: &Path, repo_path: &str) -> Result<Vec<(String, String)>> {
        // Read secrets.yaml file if it exists
        let secrets_file_path = stack_dir.join("secrets.yaml");
        if !secrets_file_path.exists() {
            return Ok(Vec::new());
        }

        let secrets_content = fs::read_to_string(&secrets_file_path)?;
        let secrets_definitions: Vec<SecretDefinition> = serde_yaml::from_str(&secrets_content)?;

        // Read NFS configuration to get the secrets path
        let nfs_config = Self::load_nfs_config(repo_path)?;
        let secrets_base_path = Path::new(&nfs_config.path).join("secret");

        let mut env_vars = Vec::new();

        // Process each secret definition
        for secret_def in &secrets_definitions {
            // Read secret value from NFS secrets directory
            let secret_path = secrets_base_path.join(&secret_def.id);
            if !secret_path.exists() {
                return Err(anyhow::anyhow!("Secret file not found: {}", secret_path.display()));
            }

            let secret_value = fs::read_to_string(&secret_path)?;
            let secret_value = secret_value.trim(); // Remove trailing whitespace/newlines

            // Add to environment variables list
            env_vars.push((secret_def.env.clone(), secret_value.to_string()));
        }

        Ok(env_vars)
    }
}

