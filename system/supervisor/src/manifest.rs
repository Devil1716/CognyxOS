use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RestartPolicy {
    Always,
    OnFailure,
    Never,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub interval_secs: u64,
    pub timeout_secs: u64,
    pub max_retries: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval_secs: 5,
            timeout_secs: 2,
            max_retries: 3,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceManifest {
    pub name: String,
    pub description: String,
    pub binary_path: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub dependencies: Vec<String>,
    pub priority: u32,
    pub restart_policy: RestartPolicy,
    pub health_check: HealthCheckConfig,
}

impl ServiceManifest {
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }
}
