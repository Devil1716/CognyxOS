use crate::model::{Worker, WorkerError, WorkerHealth, WorkerResult, WorkerStatus};
use async_trait::async_trait;
use cognyx_execution::{ExecutionRuntime, RuntimeKind, RuntimeStatus};

/// Adapter: a remote worker appears to RuntimeRegistry as an ExecutionRuntime.
pub struct RemoteWorkerRuntime {
    pub worker: Worker,
    status: RuntimeStatus,
}

impl RemoteWorkerRuntime {
    pub fn new(worker: Worker) -> Self {
        Self {
            worker,
            status: RuntimeStatus::Running,
        }
    }

    fn kind_from_os(os: &[String]) -> RuntimeKind {
        match os.first().map(|s| s.as_str()) {
            Some("windows") => RuntimeKind::WindowsVm,
            Some("macos") => RuntimeKind::MacOsVm,
            Some("container") => RuntimeKind::Container,
            _ => RuntimeKind::RemoteWorker,
        }
    }
}

#[async_trait]
impl ExecutionRuntime for RemoteWorkerRuntime {
    fn runtime_id(&self) -> &str {
        &self.worker.identity.worker_id
    }
    fn runtime_type(&self) -> RuntimeKind {
        Self::kind_from_os(&self.worker.capabilities.os)
    }
    fn status(&self) -> RuntimeStatus {
        if self.worker.health == WorkerHealth::Disconnected
            || self.worker.status == WorkerStatus::Offline
        {
            RuntimeStatus::Failed("disconnected".into())
        } else {
            self.status.clone()
        }
    }
    fn capabilities(&self) -> Vec<String> {
        let mut caps = vec!["filesystem.read".into(), "filesystem.write".into()];
        caps.extend(self.worker.capabilities.applications.clone());
        caps
    }
    fn location(&self) -> String {
        format!("worker:{}", self.worker.identity.worker_id)
    }
    fn security_level(&self) -> u32 {
        3
    }
    fn available_tools(&self) -> Vec<String> {
        self.worker.capabilities.applications.clone()
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
        if self.worker.health == WorkerHealth::Disconnected {
            return Err("worker disconnected".into());
        }
        Ok(format!(
            "remote {} ran '{}'",
            self.worker.identity.worker_id, command
        ))
    }
}

#[allow(dead_code)]
pub fn map_exec_err(err: String) -> WorkerError {
    WorkerError::Network(err)
}

#[allow(dead_code)]
pub type RemoteExecResult = WorkerResult<String>;
