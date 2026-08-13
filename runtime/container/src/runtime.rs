use crate::backend::{ContainerBackend, MockContainerBackend};
use async_trait::async_trait;
use cognyx_execution::{ExecutionRuntime, RuntimeKind, RuntimeStatus};
use std::sync::Arc;

pub struct ContainerRuntime {
    pub id: String,
    pub name: String,
    pub status: RuntimeStatus,
    pub backend: Arc<dyn ContainerBackend>,
}

impl ContainerRuntime {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            status: RuntimeStatus::Created,
            backend: Arc::new(MockContainerBackend::new()),
        }
    }
}

#[async_trait]
impl ExecutionRuntime for ContainerRuntime {
    fn runtime_id(&self) -> &str {
        &self.id
    }

    fn runtime_type(&self) -> RuntimeKind {
        RuntimeKind::Container
    }

    fn status(&self) -> RuntimeStatus {
        self.status.clone()
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "container.exec".to_string(),
            "container.logs".to_string(),
            "container.isolate".to_string(),
        ]
    }

    fn location(&self) -> String {
        "local".to_string()
    }

    fn security_level(&self) -> u32 {
        3 // Container isolated
    }

    fn available_tools(&self) -> Vec<String> {
        vec!["sh".to_string(), "env".to_string(), "cat".to_string()]
    }

    async fn start(&mut self) -> Result<(), String> {
        self.backend
            .start_container(&self.id)
            .await
            .map_err(|e| e.to_string())?;
        self.status = RuntimeStatus::Running;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.backend
            .stop_container(&self.id)
            .await
            .map_err(|e| e.to_string())?;
        self.status = RuntimeStatus::Stopped;
        Ok(())
    }

    async fn pause(&mut self) -> Result<(), String> {
        self.backend
            .pause_container(&self.id)
            .await
            .map_err(|e| e.to_string())?;
        self.status = RuntimeStatus::Paused;
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), String> {
        self.backend
            .resume_container(&self.id)
            .await
            .map_err(|e| e.to_string())?;
        self.status = RuntimeStatus::Running;
        Ok(())
    }

    async fn execute_command(&self, command: &str, args: &[&str]) -> Result<String, String> {
        let mut full_cmd = vec![command];
        full_cmd.extend_from_slice(args);
        self.backend
            .exec_command(&self.id, &full_cmd)
            .await
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_container_runtime_lifecycle() {
        let mut runtime = ContainerRuntime::new("c-101", "Alpine Web Server");
        assert_eq!(runtime.runtime_type(), RuntimeKind::Container);

        // Pre-create in mock backend
        let spec = crate::backend::ContainerSpec {
            container_id: "c-101".to_string(),
            name: "Alpine Web Server".to_string(),
            image: "alpine:latest".to_string(),
            command: vec!["sleep".to_string(), "3600".to_string()],
            env: vec![],
            volume_mounts: vec![],
            gpu_access: false,
        };
        runtime.backend.create_container(spec).await.unwrap();

        runtime.start().await.unwrap();
        assert_eq!(runtime.status(), RuntimeStatus::Running);

        let out = runtime.execute_command("echo", &["hello"]).await.unwrap();
        assert!(out.contains("echo"));

        runtime.stop().await.unwrap();
        assert_eq!(runtime.status(), RuntimeStatus::Stopped);
    }
}
