use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("Container not found: {0}")]
    NotFound(String),
    #[error("Execution error: {0}")]
    ExecutionFailed(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerSpec {
    pub container_id: String,
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
    pub env: Vec<(String, String)>,
    pub volume_mounts: Vec<String>,
    pub gpu_access: bool,
}

#[async_trait]
pub trait ContainerBackend: Send + Sync {
    async fn create_container(&self, spec: ContainerSpec) -> Result<String, ContainerError>;
    async fn start_container(&self, id: &str) -> Result<(), ContainerError>;
    async fn stop_container(&self, id: &str) -> Result<(), ContainerError>;
    async fn pause_container(&self, id: &str) -> Result<(), ContainerError>;
    async fn resume_container(&self, id: &str) -> Result<(), ContainerError>;
    async fn delete_container(&self, id: &str) -> Result<(), ContainerError>;
    async fn exec_command(&self, id: &str, cmd: &[&str]) -> Result<String, ContainerError>;
    async fn get_logs(&self, id: &str) -> Result<Vec<String>, ContainerError>;
}

#[derive(Default)]
pub struct MockContainerBackend {
    containers: dashmap::DashMap<String, (ContainerSpec, String)>,
}

impl MockContainerBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ContainerBackend for MockContainerBackend {
    async fn create_container(&self, spec: ContainerSpec) -> Result<String, ContainerError> {
        let id = spec.container_id.clone();
        self.containers
            .insert(id.clone(), (spec, "created".to_string()));
        Ok(id)
    }

    async fn start_container(&self, id: &str) -> Result<(), ContainerError> {
        let mut entry = self
            .containers
            .get_mut(id)
            .ok_or_else(|| ContainerError::NotFound(id.to_string()))?;
        entry.1 = "running".to_string();
        Ok(())
    }

    async fn stop_container(&self, id: &str) -> Result<(), ContainerError> {
        let mut entry = self
            .containers
            .get_mut(id)
            .ok_or_else(|| ContainerError::NotFound(id.to_string()))?;
        entry.1 = "stopped".to_string();
        Ok(())
    }

    async fn pause_container(&self, id: &str) -> Result<(), ContainerError> {
        let mut entry = self
            .containers
            .get_mut(id)
            .ok_or_else(|| ContainerError::NotFound(id.to_string()))?;
        entry.1 = "paused".to_string();
        Ok(())
    }

    async fn resume_container(&self, id: &str) -> Result<(), ContainerError> {
        let mut entry = self
            .containers
            .get_mut(id)
            .ok_or_else(|| ContainerError::NotFound(id.to_string()))?;
        entry.1 = "running".to_string();
        Ok(())
    }

    async fn delete_container(&self, id: &str) -> Result<(), ContainerError> {
        self.containers
            .remove(id)
            .ok_or_else(|| ContainerError::NotFound(id.to_string()))?;
        Ok(())
    }

    async fn exec_command(&self, id: &str, cmd: &[&str]) -> Result<String, ContainerError> {
        if !self.containers.contains_key(id) {
            return Err(ContainerError::NotFound(id.to_string()));
        }
        Ok(format!("Executed '{:?}' in container {}", cmd, id))
    }

    async fn get_logs(&self, id: &str) -> Result<Vec<String>, ContainerError> {
        if !self.containers.contains_key(id) {
            return Err(ContainerError::NotFound(id.to_string()));
        }
        Ok(vec![format!("[MockContainer] Container {} started", id)])
    }
}
