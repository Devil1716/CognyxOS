//! Configuration module for cognyxd

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Failed to parse config: {0}")]
    Parse(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub host: HostConfig,
    
    #[serde(default)]
    pub network: NetworkConfig,
    
    #[serde(default)]
    pub storage: StorageConfig,
    
    #[serde(default)]
    pub virtualization: VirtualizationConfig,
    
    #[serde(default)]
    pub security: SecurityConfig,
    
    #[serde(default)]
    pub logging: LoggingConfig,
    
    #[serde(default)]
    pub api: ApiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: HostConfig::default(),
            network: NetworkConfig::default(),
            storage: StorageConfig::default(),
            virtualization: VirtualizationConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
            api: ApiConfig::default(),
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    #[serde(default = "default_machine_id")]
    pub machine_id: String,
    
    #[serde(default)]
    pub mode: String,
}

fn default_machine_id() -> String {
    "auto".to_string()
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            machine_id: default_machine_id(),
            mode: "infrastructure".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_bridge_name")]
    pub bridge_name: String,
    
    #[serde(default = "default_nat_subnet")]
    pub nat_subnet: String,
    
    #[serde(default = "default_dns_servers")]
    pub dns_servers: Vec<String>,
}

fn default_bridge_name() -> String {
    "cognyx-br0".to_string()
}

fn default_nat_subnet() -> String {
    "10.100.0.0/16".to_string()
}

fn default_dns_servers() -> Vec<String> {
    vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()]
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bridge_name: default_bridge_name(),
            nat_subnet: default_nat_subnet(),
            dns_servers: default_dns_servers(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_pool_path")]
    pub pool_path: String,
    
    #[serde(default)]
    pub default_driver: String,
    
    #[serde(default)]
    pub encryption: EncryptionConfig,
}

fn default_pool_path() -> String {
    "/var/lib/cognyx/storage".to_string()
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            pool_path: default_pool_path(),
            default_driver: "dir".to_string(),
            encryption: EncryptionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    #[serde(default)]
    pub enabled: bool,
    
    #[serde(default)]
    pub keyring: String,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            keyring: "system".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualizationConfig {
    #[serde(default)]
    pub hypervisor: String,
    
    #[serde(default = "default_memory")]
    pub default_memory: u64,
    
    #[serde(default = "default_vcpus")]
    pub default_vcpus: u32,
    
    #[serde(default)]
    pub gpu_passthrough: GpuPassthroughConfig,
}

fn default_memory() -> u64 {
    2048
}

fn default_vcpus() -> u32 {
    2
}

impl Default for VirtualizationConfig {
    fn default() -> Self {
        Self {
            hypervisor: "qemu".to_string(),
            default_memory: default_memory(),
            default_vcpus: default_vcpus(),
            gpu_passthrough: GpuPassthroughConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuPassthroughConfig {
    #[serde(default)]
    pub enabled: bool,
    
    #[serde(default)]
    pub iommu_group: String,
}

impl Default for GpuPassthroughConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            iommu_group: "auto".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_true")]
    pub iommu: bool,
    
    #[serde(default = "default_true")]
    pub cpu_mitigations: bool,
    
    #[serde(default)]
    pub audit: bool,
    
    #[serde(default = "default_max_vms")]
    pub max_vms: u32,
}

fn default_true() -> bool {
    true
}

fn default_max_vms() -> u32 {
    50
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            iommu: default_true(),
            cpu_mitigations: default_true(),
            audit: true,
            max_vms: default_max_vms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default)]
    pub level: String,
    
    #[serde(default)]
    pub format: String,
    
    #[serde(default)]
    pub output: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
            output: "journal".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_socket_path")]
    pub socket_path: String,
    
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    
    #[serde(default = "default_max_payload")]
    pub max_payload_size: u64,
}

fn default_socket_path() -> String {
    "/run/cognyx/cognyxd.sock".to_string()
}

fn default_timeout() -> u64 {
    30
}

fn default_max_payload() -> u64 {
    100
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            socket_path: default_socket_path(),
            timeout: default_timeout(),
            max_payload_size: default_max_payload(),
        }
    }
}
