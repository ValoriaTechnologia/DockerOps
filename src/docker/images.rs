use anyhow::Result;
use bollard::query_parameters::{CreateImageOptions, ListImagesOptions, RemoveImageOptions};
use bollard::models::ImageSummary;
use futures::StreamExt;
use crate::docker::client::DockerClient;
use crate::config::ImagePullPolicy;

/// Service pour gérer les images Docker
pub struct ImageService {
    client: DockerClient,
    pull_policy: ImagePullPolicy,
}

impl ImageService {
    /// Crée un nouveau service d'images
    pub fn new(client: DockerClient, pull_policy: ImagePullPolicy) -> Self {
        Self {
            client,
            pull_policy,
        }
    }

    /// Vérifie si une image existe localement
    pub async fn image_exists(&self, image_name: &str) -> Result<bool> {
        let images: Vec<ImageSummary> = self.client.docker().list_images(None::<ListImagesOptions>).await?;
        
        Ok(images.iter().any(|img| {
            img.repo_tags.iter().any(|tag| tag == image_name)
        }))
    }

    /// Récupère le SHA d'une image locale
    pub async fn get_local_image_sha(&self, image_name: &str) -> Result<Option<String>> {
        match self.client.docker().inspect_image(image_name).await {
            Ok(image) => {
                // L'ID de l'image est le SHA
                Ok(image.id.map(|id| id.trim_start_matches("sha256:").to_string()))
            }
            Err(_) => Ok(None),
        }
    }

    /// Pull une image Docker selon la policy configurée
    pub async fn pull_image(&self, image_name: &str) -> Result<()> {
        match self.pull_policy {
            ImagePullPolicy::Always => {
                println!("    Pulling image: {} (policy: Always)", image_name);
                self.force_pull_image(image_name).await
            }
            ImagePullPolicy::IfNotPresent => {
                if self.image_exists(image_name).await? {
                    println!("    Image {} already exists locally (policy: IfNotPresent), skipping pull", image_name);
                    Ok(())
                } else {
                    println!("    Pulling image: {} (policy: IfNotPresent, not found locally)", image_name);
                    self.force_pull_image(image_name).await
                }
            }
        }
    }

    /// Force le pull d'une image (toujours télécharger)
    async fn force_pull_image(&self, image_name: &str) -> Result<()> {
        let options = CreateImageOptions {
            from_image: Some(image_name.to_string()),
            ..Default::default()
        };

        let mut stream = self.client.docker().create_image(Some(options), None, None);
        
        while let Some(result) = stream.next().await {
            match result {
                Ok(_) => {
                    // Progress information is available here if needed
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to pull image {}: {}", image_name, e));
                }
            }
        }

        println!("    Successfully pulled image: {}", image_name);
        Ok(())
    }

    /// Supprime une image Docker
    pub async fn remove_image(&self, image_name: &str) -> Result<()> {
        println!("    Removing image: {}", image_name);
        
        match self.client.docker().remove_image(image_name, None::<RemoveImageOptions>, None).await {
            Ok(_) => {
                println!("    Successfully removed image: {}", image_name);
                Ok(())
            }
            Err(e) => {
                println!("    Warning: Error removing image {}: {}", image_name, e);
                // Ne pas retourner d'erreur car l'image pourrait ne pas exister
                Ok(())
            }
        }
    }
}

