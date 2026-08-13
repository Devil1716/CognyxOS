use crate::traits::{ExecutionRuntime, RuntimeKind, RuntimeStatus};
use async_trait::async_trait;

pub struct LinuxRuntime {
    pub id: String,
    pub name: String,
    pub status: RuntimeStatus,
}

impl LinuxRuntime {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            status: RuntimeStatus::Running,
        }
    }
}

#[async_trait]
impl ExecutionRuntime for LinuxRuntime {
    fn runtime_id(&self) -> &str {
        &self.id
    }

    fn runtime_type(&self) -> RuntimeKind {
        RuntimeKind::NativeLinux
    }

    fn status(&self) -> RuntimeStatus {
        self.status.clone()
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "bash".to_string(),
            "process.spawn".to_string(),
            "filesystem.read".to_string(),
            "filesystem.write".to_string(),
            "network.access".to_string(),
        ]
    }

    fn location(&self) -> String {
        "local".to_string()
    }

    fn security_level(&self) -> u32 {
        1 // Host level
    }

    fn available_tools(&self) -> Vec<String> {
        vec![
            "bash".to_string(),
            "git".to_string(),
            "cargo".to_string(),
            "python".to_string(),
        ]
    }

    async fn start(&mut self) -> Result<(), String> {
        self.status = RuntimeStatus::Running;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
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
        Ok(format!("Executed '{}' on Native Linux", command))
    }
}
