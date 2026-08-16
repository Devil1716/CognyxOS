use crate::automation::WindowsAppAutomation;
use crate::vm_manager::WindowsVmManager;
use async_trait::async_trait;
use cognyx_execution::{ExecutionRuntime, RuntimeKind, RuntimeStatus};

pub struct WindowsRuntime {
    pub id: String,
    pub name: String,
    pub status: RuntimeStatus,
    pub vm_manager: WindowsVmManager,
    pub automation: WindowsAppAutomation,
}

impl WindowsRuntime {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let id_str = id.into();
        Self {
            id: id_str.clone(),
            name: name.into(),
            status: RuntimeStatus::Created,
            vm_manager: WindowsVmManager::default(),
            automation: WindowsAppAutomation::new(&id_str),
        }
    }

    /// Native Windows host identity for RuntimeRegistry. Does not start a VM.
    /// `execute_command` still goes through the existing automation adapter
    /// (formatted string — not used by CapabilityGateway after VAL-001).
    pub fn host() -> Self {
        let id = cognyx_execution::native_host_runtime_id();
        Self {
            id: id.to_string(),
            name: cognyx_execution::native_host_runtime_name().to_string(),
            status: RuntimeStatus::Running,
            vm_manager: WindowsVmManager::default(),
            automation: WindowsAppAutomation::new(id),
        }
    }
}

#[async_trait]
impl ExecutionRuntime for WindowsRuntime {
    fn runtime_id(&self) -> &str {
        &self.id
    }

    fn runtime_type(&self) -> RuntimeKind {
        RuntimeKind::WindowsVm
    }

    fn status(&self) -> RuntimeStatus {
        self.status.clone()
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "win32.powershell".to_string(),
            "win32.cmd".to_string(),
            "win32.filesystem".to_string(),
            "win32.automation".to_string(),
        ]
    }

    fn location(&self) -> String {
        if self.id == cognyx_execution::native_host_runtime_id() {
            "local-host".to_string()
        } else {
            "local-kvm".to_string()
        }
    }

    fn security_level(&self) -> u32 {
        if self.id == cognyx_execution::native_host_runtime_id() {
            1
        } else {
            4 // Full hardware isolated VM
        }
    }

    fn available_tools(&self) -> Vec<String> {
        vec!["powershell.exe".to_string(), "cmd.exe".to_string()]
    }

    async fn start(&mut self) -> Result<(), String> {
        self.vm_manager.start_windows_vm(&self.id).await?;
        self.status = RuntimeStatus::Running;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.vm_manager.stop_windows_vm(&self.id).await?;
        self.status = RuntimeStatus::Stopped;
        Ok(())
    }

    async fn pause(&mut self) -> Result<(), String> {
        self.vm_manager.pause_windows_vm(&self.id).await?;
        self.status = RuntimeStatus::Paused;
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), String> {
        self.vm_manager.resume_windows_vm(&self.id).await?;
        self.status = RuntimeStatus::Running;
        Ok(())
    }

    async fn execute_command(&self, command: &str, _args: &[&str]) -> Result<String, String> {
        self.automation.execute_powershell(command).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_windows_runtime_execution() {
        let mut runtime = WindowsRuntime::new("win-10", "Windows 11 Enterprise");
        assert_eq!(runtime.runtime_type(), RuntimeKind::WindowsVm);

        runtime.start().await.unwrap();
        assert_eq!(runtime.status(), RuntimeStatus::Running);

        let res = runtime.execute_command("Get-Process", &[]).await.unwrap();
        assert!(res.contains("Get-Process"));

        runtime.stop().await.unwrap();
        assert_eq!(runtime.status(), RuntimeStatus::Stopped);
    }

    #[test]
    fn native_host_identity_is_windows_not_linux() {
        let host = WindowsRuntime::host();
        assert!(!host.runtime_id().to_lowercase().contains("linux"));
        assert!(host.runtime_id().to_lowercase().contains("windows"));
        assert_eq!(host.status(), RuntimeStatus::Running);
    }
}
