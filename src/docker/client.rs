use anyhow::Result;
use bollard::Docker;

/// Client Docker utilisant bollard
pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
    /// Crée un nouveau client Docker
    pub async fn new() -> Result<Self> {
        let docker = Docker::connect_with_socket_defaults()
            .map_err(|e| anyhow::anyhow!("Failed to connect to Docker daemon: {}", e))?;

        Ok(Self { docker })
    }

    /// Retourne une référence au client Docker interne
    pub fn docker(&self) -> &Docker {
        &self.docker
    }
}

