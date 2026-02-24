use anyhow::Result;
use octocrab::Octocrab;
use std::sync::Arc;

/// Client GitHub utilisant octocrab pour l'API GitHub
pub struct GitHubClient {
    octocrab: Arc<Octocrab>,
}

impl GitHubClient {
    /// Crée un nouveau client GitHub
    pub fn new() -> Result<Self> {
        let octocrab = if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            Octocrab::builder()
                .personal_token(token)
                .build()?
        } else {
            Octocrab::builder().build()?
        };

        Ok(Self {
            octocrab: Arc::new(octocrab),
        })
    }

    /// Crée un nouveau client GitHub avec un token spécifique
    pub fn with_token(token: String) -> Result<Self> {
        let octocrab = Octocrab::builder()
            .personal_token(token)
            .build()?;

        Ok(Self {
            octocrab: Arc::new(octocrab),
        })
    }

    /// Retourne une référence au client octocrab interne
    pub fn octocrab(&self) -> &Octocrab {
        &self.octocrab
    }
}

impl Default for GitHubClient {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback si octocrab ne peut pas être créé
            panic!("Failed to create GitHub client. Please set GITHUB_TOKEN environment variable if needed.")
        })
    }
}

