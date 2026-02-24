use anyhow::Result;
use std::path::Path;
use std::fs;
use std::process::Command;
use crate::models::{VolumeDefinition, VolumeType, NfsConfig};

/// Processeur pour gérer les volumes (NFS, bindings, etc.)
pub struct VolumeProcessor;

impl VolumeProcessor {
    /// Traite la configuration des volumes depuis un fichier volumes.yaml
    pub fn load_volumes_config(repo_path: &str) -> Result<Option<Vec<VolumeDefinition>>> {
        let volumes_file_path = Path::new(repo_path).join("volumes.yaml");
        if !volumes_file_path.exists() {
            return Ok(None);
        }

        let volumes_content = fs::read_to_string(&volumes_file_path)?;
        let volumes_definitions: Vec<VolumeDefinition> = serde_yaml::from_str(&volumes_content)?;

        Ok(Some(volumes_definitions))
    }

    /// Traite tous les volumes définis
    pub async fn process_volumes(
        volumes_definitions: &mut Vec<VolumeDefinition>,
        nfs_config: Option<&NfsConfig>,
        repo_path: &str,
    ) -> Result<()> {
        for volume_def in volumes_definitions {
            match volume_def.r#type {
                VolumeType::Volume => {
                    // For Docker volumes, we just need to ensure they exist
                    // This is handled by Docker itself when deploying
                }
                VolumeType::Binding => {
                    if let Some(nfs_config) = nfs_config {
                        Self::process_binding_volume(volume_def, nfs_config, repo_path).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Traite un volume de type binding (copie vers NFS)
    async fn process_binding_volume(
        volume_def: &mut VolumeDefinition,
        nfs_config: &NfsConfig,
        repo_path: &str,
    ) -> Result<()> {
        let local_path = Path::new(repo_path).join(&volume_def.path);

        if !local_path.exists() {
            return Ok(());
        }

        // Create NFS destination path
        let nfs_dest_path = Path::new(&nfs_config.path).join(&volume_def.path);

        // Remove existing file or directory on NFS if it exists
        if nfs_dest_path.exists() {
            let metadata = fs::metadata(&nfs_dest_path)?;
            if metadata.is_dir() {
                fs::remove_dir_all(&nfs_dest_path)?;
            } else {
                fs::remove_file(&nfs_dest_path)?;
            }
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = nfs_dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Copy recursively
        if local_path.is_dir() {
            Self::copy_directory_recursive(&local_path, &nfs_dest_path).await?;
        } else {
            if let Some(parent) = nfs_dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&local_path, &nfs_dest_path)?;
        }

        // Fix permissions for Docker compatibility
        Self::fix_permissions_recursive(&nfs_dest_path).await?;

        // Update the volume definition path to point to NFS
        volume_def.path = nfs_dest_path.to_string_lossy().to_string();

        Ok(())
    }

    /// Copie récursivement un répertoire
    async fn copy_directory_recursive(src: &Path, dst: &Path) -> Result<()> {
        if !src.is_dir() {
            return Err(anyhow::anyhow!("Source is not a directory: {}", src.display()));
        }

        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if file_type.is_dir() {
                Box::pin(Self::copy_directory_recursive(&src_path, &dst_path)).await?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    /// Corrige les permissions pour la compatibilité Docker
    async fn fix_permissions_recursive(path: &Path) -> Result<()> {
        // Use chmod command to set appropriate permissions
        let output = Command::new("chmod")
            .args(&["-R", "755", path.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: Failed to set directory permissions: {}", error);
        }

        // For files, set 644 permissions
        let output = Command::new("find")
            .args(&[path.to_str().unwrap(), "-type", "f", "-exec", "chmod", "644", "{}", ";"])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: Failed to set file permissions: {}", error);
        }

        // Change ownership to a more Docker-friendly user/group
        let current_user = std::env::var("SUDO_USER")
            .ok()
            .or_else(|| std::env::var("USER").ok())
            .unwrap_or_else(|| "1000".to_string());

        let output = Command::new("chown")
            .args(&["-R", &format!("{}:{}", current_user, current_user), path.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: Failed to change ownership: {}", error);
        }

        Ok(())
    }
}

