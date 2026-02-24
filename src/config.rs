use serde::{Deserialize, Serialize};

/// Policy de pull d'images Docker, similaire à k3s
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImagePullPolicy {
    /// Télécharge toujours l'image depuis le registry (comme k3s Always)
    Always,
    /// Télécharge seulement si l'image n'est pas présente localement (comme k3s IfNotPresent)
    IfNotPresent,
}

impl Default for ImagePullPolicy {
    fn default() -> Self {
        ImagePullPolicy::IfNotPresent
    }
}

impl ImagePullPolicy {
    /// Parse une policy depuis une string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "always" => Ok(ImagePullPolicy::Always),
            "ifnotpresent" | "if_not_present" => Ok(ImagePullPolicy::IfNotPresent),
            _ => Err(format!("Unknown image pull policy: {}", s)),
        }
    }
}

/// Configuration globale de l'application
#[derive(Debug, Clone)]
pub struct Config {
    pub image_pull_policy: ImagePullPolicy,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            image_pull_policy: ImagePullPolicy::default(),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    /// Charge la configuration depuis les variables d'environnement
    pub fn from_env() -> Self {
        let policy = std::env::var("DOCKEROPS_IMAGE_PULL_POLICY")
            .ok()
            .and_then(|s| ImagePullPolicy::from_str(&s).ok())
            .unwrap_or_default();

        Self {
            image_pull_policy: policy,
        }
    }
}

