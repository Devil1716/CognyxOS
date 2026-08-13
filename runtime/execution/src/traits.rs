use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RuntimeKind {
    NativeLinux,
    WindowsVm,
    MacOsVm,
    Container,
    RemoteWorker,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RuntimeStatus {
    Created,
    Starting,
    Running,
    Paused,
    Stopping,
    Stopped,
    Failed(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub runtime_id: String,
    pub name: String,
    pub kind: RuntimeKind,
    pub status: RuntimeStatus,
    pub capabilities: Vec<String>,
    pub location: String,
    pub security_level: u32,
    pub available_tools: Vec<String>,
}

#[async_trait]
pub trait ExecutionRuntime: Send + Sync {
    fn runtime_id(&self) -> &str;
    fn runtime_type(&self) -> RuntimeKind;
    fn status(&self) -> RuntimeStatus;
    fn capabilities(&self) -> Vec<String>;
    fn location(&self) -> String;
    fn security_level(&self) -> u32;
    fn available_tools(&self) -> Vec<String>;
    fn can_perform(&self, capability: &str) -> bool {
        self.capabilities()
            .iter()
            .any(|c| c == capability || c == "*")
    }

    async fn start(&mut self) -> Result<(), String>;
    async fn stop(&mut self) -> Result<(), String>;
    async fn pause(&mut self) -> Result<(), String>;
    async fn resume(&mut self) -> Result<(), String>;
    async fn execute_command(&self, command: &str, args: &[&str]) -> Result<String, String>;
}
