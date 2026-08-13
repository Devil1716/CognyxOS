use crate::backend::{LocalMacBackend, MacOSExecutionBackend, RemoteMacBackend};
use async_trait::async_trait;
use cognyx_execution::{ExecutionRuntime, RuntimeKind, RuntimeStatus};
use std::sync::Arc;

pub struct MacOSRuntime {
    pub id: String,
    pub name: String,
    pub status: RuntimeStatus,
    pub backend: Arc<dyn MacOSExecutionBackend>,
}

impl MacOSRuntime {
    pub fn new_local(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            status: RuntimeStatus::Created,
            backend: Arc::new(LocalMacBackend::new()),
        }
    }

    pub fn new_remote(
        id: impl Into<String>,
        name: impl Into<String>,
        remote_host: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            status: RuntimeStatus::Created,
            backend: Arc::new(RemoteMacBackend::new(remote_host)),
        }
    }
}

#[async_trait]
impl ExecutionRuntime for MacOSRuntime {
    fn runtime_id(&self) -> &str {
        &self.id
    }

    fn runtime_type(&self) -> RuntimeKind {
        RuntimeKind::MacOsVm
    }

    fn status(&self) -> RuntimeStatus {
        self.status.clone()
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "macos.zsh".to_string(),
            "macos.xcode".to_string(),
            "macos.applescript".to_string(),
        ]
    }

    fn location(&self) -> String {
        self.backend.backend_type().to_string()
    }

    fn security_level(&self) -> u32 {
        4 // Isolated runtime
    }

    fn available_tools(&self) -> Vec<String> {
        vec![
            "zsh".to_string(),
            "xcodebuild".to_string(),
            "osascript".to_string(),
        ]
    }

    async fn start(&mut self) -> Result<(), String> {
        self.backend.start_mac().await?;
        self.status = RuntimeStatus::Running;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.backend.stop_mac().await?;
        self.status = RuntimeStatus::Stopped;
        Ok(())
    }

    async fn pause(&mut self) -> Result<(), String> {
        self.status = RuntimeStatus::Paused;
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), String> {
        self.status = RuntimeStatus::Running;
        Ok(())
    }

    async fn execute_command(&self, command: &str, _args: &[&str]) -> Result<String, String> {
        self.backend.execute(command).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_remote_mac_runtime() {
        let mut runtime =
            MacOSRuntime::new_remote("mac-1", "Mac Studio Worker", "mac-host.local:50051");
        assert_eq!(runtime.runtime_type(), RuntimeKind::MacOsVm);

        runtime.start().await.unwrap();
        assert_eq!(runtime.status(), RuntimeStatus::Running);

        let out = runtime
            .execute_command("xcodebuild -version", &[])
            .await
            .unwrap();
        assert!(out.contains("Mac Studio Worker") || out.contains("xcodebuild"));

        runtime.stop().await.unwrap();
        assert_eq!(runtime.status(), RuntimeStatus::Stopped);
    }
}
