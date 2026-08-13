use crate::types::*;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VirtualizationError {
    #[error("VM not found: {0}")]
    NotFound(String),
    #[error("Invalid VM state transition: {0}")]
    InvalidState(String),
    #[error("QEMU execution error: {0}")]
    QemuError(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[async_trait]
pub trait VirtualizationBackend: Send + Sync {
    async fn create_vm(&self, config: VirtualMachineConfig) -> Result<String, VirtualizationError>;
    async fn start_vm(&self, vm_id: &str) -> Result<(), VirtualizationError>;
    async fn stop_vm(&self, vm_id: &str) -> Result<(), VirtualizationError>;
    async fn pause_vm(&self, vm_id: &str) -> Result<(), VirtualizationError>;
    async fn resume_vm(&self, vm_id: &str) -> Result<(), VirtualizationError>;
    async fn restart_vm(&self, vm_id: &str) -> Result<(), VirtualizationError>;
    async fn shutdown_vm(&self, vm_id: &str) -> Result<(), VirtualizationError>;
    async fn force_stop_vm(&self, vm_id: &str) -> Result<(), VirtualizationError>;
    async fn delete_vm(&self, vm_id: &str) -> Result<(), VirtualizationError>;
    async fn snapshot_vm(
        &self,
        vm_id: &str,
        name: &str,
    ) -> Result<VirtualMachineSnapshot, VirtualizationError>;
    async fn restore_snapshot(
        &self,
        vm_id: &str,
        snapshot_id: &str,
    ) -> Result<(), VirtualizationError>;
    async fn clone_vm(&self, vm_id: &str, new_name: &str) -> Result<String, VirtualizationError>;
    async fn inspect_vm(&self, vm_id: &str) -> Result<VirtualMachineConfig, VirtualizationError>;
    async fn get_state(&self, vm_id: &str) -> Result<VirtualMachineState, VirtualizationError>;
    async fn attach_device(
        &self,
        vm_id: &str,
        device: VirtualMachineDevice,
    ) -> Result<(), VirtualizationError>;
    async fn detach_device(&self, vm_id: &str, device_id: &str) -> Result<(), VirtualizationError>;
    async fn resize_resources(
        &self,
        vm_id: &str,
        new_resources: VirtualMachineResource,
    ) -> Result<(), VirtualizationError>;
    async fn get_metrics(&self, vm_id: &str) -> Result<VirtualMachineMetrics, VirtualizationError>;
    async fn get_logs(&self, vm_id: &str) -> Result<Vec<String>, VirtualizationError>;
    async fn send_signal(&self, vm_id: &str, signal: i32) -> Result<(), VirtualizationError>;
    async fn connect_console(&self, vm_id: &str) -> Result<String, VirtualizationError>;
}
