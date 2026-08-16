use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerIdentity {
    pub worker_id: String,
    pub public_key: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerCapabilities {
    pub os: Vec<String>,
    pub applications: Vec<String>,
    pub gpu: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkerResources {
    pub cpu: u32,
    pub ram_mb: u32,
    pub gpu_count: u32,
    pub disk_mb: u32,
    pub network_mbps: u32,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkerHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkerStatus {
    Registered,
    Online,
    Busy,
    Offline,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerPolicy {
    pub allowed_principals: HashSet<String>,
    pub allow_destructive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Worker {
    pub identity: WorkerIdentity,
    pub capabilities: WorkerCapabilities,
    pub resources: WorkerResources,
    pub health: WorkerHealth,
    pub status: WorkerStatus,
    pub policy: WorkerPolicy,
    pub last_heartbeat_secs: u64,
    pub tls_required: bool,
}

impl Worker {
    pub fn new(id: impl Into<String>, os: &str) -> Self {
        let worker_id = id.into();
        Self {
            identity: WorkerIdentity {
                public_key: format!("pub-{worker_id}"),
                token: format!("tok-{worker_id}"),
                worker_id,
            },
            capabilities: WorkerCapabilities {
                os: vec![os.to_string()],
                applications: vec![],
                gpu: false,
            },
            resources: WorkerResources {
                cpu: 4,
                ram_mb: 8192,
                gpu_count: 0,
                disk_mb: 102400,
                network_mbps: 1000,
                latency_ms: 5,
            },
            health: WorkerHealth::Healthy,
            status: WorkerStatus::Registered,
            policy: WorkerPolicy {
                allowed_principals: HashSet::from(["kernel".into()]),
                allow_destructive: false,
            },
            last_heartbeat_secs: now_secs(),
            tls_required: true,
        }
    }
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum WorkerError {
    #[error("unauthenticated worker")]
    AuthenticationFailure,
    #[error("unauthorized: {0}")]
    AuthorizationFailure(String),
    #[error("worker unavailable: {0}")]
    Unavailable(String),
    #[error("network failure: {0}")]
    Network(String),
    #[error("duplicate destructive execution blocked: {0}")]
    DuplicateDestructive(String),
    #[error("non-transferable state: {0}")]
    NonTransferable(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("transfer integrity: {0}")]
    Integrity(String),
}

pub type WorkerResult<T> = Result<T, WorkerError>;
