use anyhow::Result;
use std::path::Path;
use std::process::Command;
use crate::docker::client::DockerClient;

/// Service pour gérer les stacks Docker Swarm
pub struct StackService {
    client: DockerClient,
}

impl StackService {
    /// Crée un nouveau service de stacks
    pub fn new(client: DockerClient) -> Self {
        Self { client }
    }

    /// Déploie un stack Docker Swarm
    pub async fn deploy_stack(&self, stack_name: &str, compose_path: &Path, secrets_env_vars: &[(String, String)]) -> Result<()> {
        println!("    Deploying stack '{}' with docker stack deploy", stack_name);

        // Pour l'instant, on utilise encore la commande docker car bollard ne supporte pas directement
        // docker stack deploy. On pourrait utiliser docker-compose ou implémenter manuellement.
        // Pour cette version, on garde la commande shell mais on pourrait améliorer plus tard.
        
        let mut command = Command::new("docker");
        command.args(&["stack", "deploy", "--detach=false", "-c", compose_path.to_str().unwrap(), stack_name]);
        
        // Add secrets as environment variables
        for (env_name, env_value) in secrets_env_vars {
            command.env(env_name, env_value);
            println!("    Added environment variable: {} (secret)", env_name);
        }
        
        let output = command.output()?;
        
        if output.status.success() {
            println!("    Successfully deployed stack '{}'", stack_name);
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("    Error deploying stack '{}': {}", stack_name, error);
            Err(anyhow::anyhow!("Failed to deploy stack: {}", error))
        }
    }

    /// Arrête un stack Docker Swarm
    pub async fn stop_stack(&self, stack_name: &str) -> Result<()> {
        println!("    Stopping stack '{}' with docker stack rm", stack_name);
        
        let output = Command::new("docker")
            .args(&["stack", "rm", stack_name])
            .output()?;
        
        if output.status.success() {
            println!("    Successfully stopped stack '{}'", stack_name);
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("    Warning: Error stopping stack '{}': {}", stack_name, error);
            // Don't return error here as the stack might not exist
        }
        
        Ok(())
    }
}

