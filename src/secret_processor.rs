use anyhow::Result;
use std::path::Path;
use std::fs;
use crate::models::{SecretDefinition, NfsConfig};

/// Processeur pour la configuration NFS et la déclaration des secrets (Docker Swarm).
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

    /// Lit secrets.yaml (déclaration uniquement), génère entrypoint-secrets.sh, retourne les définitions.
    /// Ne lit aucune valeur de secret (plus de NFS). Les secrets sont créés avec docker secret create.
    pub fn process_secrets(stack_dir: &Path) -> Result<Option<Vec<SecretDefinition>>> {
        let secrets_file_path = stack_dir.join("secrets.yaml");
        if !secrets_file_path.exists() {
            return Ok(None);
        }

        let secrets_content = fs::read_to_string(&secrets_file_path)?;
        let definitions: Vec<SecretDefinition> = serde_yaml::from_str(&secrets_content)?;
        if definitions.is_empty() {
            return Ok(None);
        }

        // Generate entrypoint script: export ENV=$(cat /run/secrets/SECRET) for each, then exec "$@"
        let mut script = String::from("#!/bin/sh\nset -e\n");
        for def in &definitions {
            script.push_str(&format!(
                "export {}=$(cat /run/secrets/{} 2>/dev/null || true)\n",
                def.env, def.secret
            ));
        }
        script.push_str("exec \"$@\"\n");

        let script_path = stack_dir.join("entrypoint-secrets.sh");
        fs::write(&script_path, script)?;

        Ok(Some(definitions))
    }
}
