use anyhow::Result;
use std::fs;
use std::path::Path;
use serde_yaml::Value;
use crate::models::VolumeDefinition;
use crate::models::NfsConfig;

/// Processeur pour les fichiers docker-compose
pub struct ComposeProcessor;

impl ComposeProcessor {
    /// Extrait les images depuis un contenu YAML de docker-compose
    pub fn extract_images(content: &str) -> Result<Vec<String>> {
        let yaml_value: Value = serde_yaml::from_str(content)?;
        let mut images_found = Vec::new();
        Self::extract_images_from_yaml(&yaml_value, &mut images_found);
        Ok(images_found)
    }

    /// Extrait récursivement les images depuis une structure YAML
    fn extract_images_from_yaml(value: &Value, images: &mut Vec<String>) {
        match value {
            Value::Mapping(mapping) => {
                for (key, val) in mapping {
                    if let Some(key_str) = key.as_str() {
                        if key_str == "image" {
                            if let Some(image_name) = val.as_str() {
                                if !image_name.is_empty() {
                                    images.push(image_name.to_string());
                                }
                            }
                        } else {
                            // Recursively search in nested structures
                            Self::extract_images_from_yaml(val, images);
                        }
                    } else {
                        // Recursively search in nested structures
                        Self::extract_images_from_yaml(val, images);
                    }
                }
            }
            Value::Sequence(sequence) => {
                for item in sequence {
                    Self::extract_images_from_yaml(item, images);
                }
            }
            _ => {
                // For other types (String, Number, etc.), do nothing
            }
        }
    }

    /// Traite les volumes dans le contenu docker-compose
    pub fn process_volumes(
        compose_content: &str,
        volumes_definitions: &[VolumeDefinition],
        nfs_config: &NfsConfig,
    ) -> Result<String> {
        // Parse the compose content to find volume references
        let mut yaml_value: serde_yaml::Value = serde_yaml::from_str(compose_content)?;

        // Process services section
        if let Some(services) = yaml_value.get_mut("services") {
            if let Some(services_mapping) = services.as_mapping_mut() {
                for (_service_name, service) in services_mapping {
                    if let Some(volumes) = service.get_mut("volumes") {
                        Self::process_service_volumes(volumes, volumes_definitions, nfs_config)?;
                    }
                }
            }
        }

        // Add volumes section to docker-compose if it doesn't exist
        Self::add_volumes_section(&mut yaml_value, volumes_definitions)?;

        // Convert back to string
        let modified_content = serde_yaml::to_string(&yaml_value)?;
        Ok(modified_content)
    }

    /// Traite les volumes d'un service spécifique
    fn process_service_volumes(
        volumes: &mut serde_yaml::Value,
        volumes_definitions: &[VolumeDefinition],
        nfs_config: &NfsConfig,
    ) -> Result<()> {
        use crate::models::VolumeType;

        match volumes {
            serde_yaml::Value::Sequence(seq) => {
                for volume in seq.iter_mut() {
                    if let Some(volume_str) = volume.as_str() {
                        // Check if this is a volume reference (format: volume_id:container_path)
                        if volume_str.contains(':') {
                            let parts: Vec<&str> = volume_str.split(':').collect();

                            if parts.len() >= 2 && parts.len() <= 3 {
                                let volume_id = parts[0];
                                let container_path = parts[1];
                                let options = if parts.len() == 3 { parts[2] } else { "" };

                                // Find the volume definition
                                if let Some(volume_def) = volumes_definitions.iter().find(|v| v.id == volume_id) {
                                    match volume_def.r#type {
                                        VolumeType::Volume => {
                                            // For Docker volumes, use the path as volume name
                                            let volume_path = if !options.is_empty() {
                                                format!("{}:{}:{}", volume_def.path, container_path, options)
                                            } else {
                                                format!("{}:{}", volume_def.path, container_path)
                                            };
                                            *volume = serde_yaml::Value::String(volume_path);
                                        }
                                        VolumeType::Binding => {
                                            // For bindings, replace with NFS path
                                            let full_nfs_path = Path::new(&nfs_config.path).join(&volume_def.path);

                                            // Create the NFS directory if it doesn't exist
                                            if !full_nfs_path.exists() {
                                                fs::create_dir_all(&full_nfs_path)?;
                                            }

                                            let nfs_path = if !options.is_empty() {
                                                format!("{}:{}:{}", full_nfs_path.display(), container_path, options)
                                            } else {
                                                format!("{}:{}", full_nfs_path.display(), container_path)
                                            };
                                            *volume = serde_yaml::Value::String(nfs_path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Ajoute la section volumes au docker-compose si elle n'existe pas
    fn add_volumes_section(
        yaml_value: &mut serde_yaml::Value,
        volumes_definitions: &[VolumeDefinition],
    ) -> Result<()> {
        use crate::models::VolumeType;

        // Create volumes section if it doesn't exist
        if yaml_value.get("volumes").is_none() {
            yaml_value["volumes"] = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        }

        let volumes_section = yaml_value.get_mut("volumes").unwrap();

        // Add each volume definition to the volumes section
        for volume_def in volumes_definitions {
            match volume_def.r#type {
                VolumeType::Volume => {
                    // Create volume configuration
                    let mut volume_config = serde_yaml::Mapping::new();
                    volume_config.insert(
                        serde_yaml::Value::String("driver".to_string()),
                        serde_yaml::Value::String("local".to_string())
                    );
                    volumes_section[&volume_def.id] = serde_yaml::Value::Mapping(volume_config);
                }
                VolumeType::Binding => {
                    // Bindings don't need to be in the volumes section
                }
            }
        }

        Ok(())
    }
}

