use crate::backend::{VirtualizationBackend, VirtualizationError};
use crate::types::*;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct MockBackend {
    pub vms: Arc<DashMap<String, (VirtualMachineConfig, VirtualMachineState)>>,
    pub snapshots: Arc<DashMap<String, VirtualMachineSnapshot>>,
}

impl MockBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl VirtualizationBackend for MockBackend {
    async fn create_vm(&self, config: VirtualMachineConfig) -> Result<String, VirtualizationError> {
        let id = config.vm_id.clone();
        self.vms
            .insert(id.clone(), (config, VirtualMachineState::Stopped));
        Ok(id)
    }

    async fn start_vm(&self, vm_id: &str) -> Result<(), VirtualizationError> {
        let mut entry = self
            .vms
            .get_mut(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;
        entry.1 = VirtualMachineState::Running;
        Ok(())
    }

    async fn stop_vm(&self, vm_id: &str) -> Result<(), VirtualizationError> {
        let mut entry = self
            .vms
            .get_mut(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;
        entry.1 = VirtualMachineState::Stopped;
        Ok(())
    }

    async fn pause_vm(&self, vm_id: &str) -> Result<(), VirtualizationError> {
        let mut entry = self
            .vms
            .get_mut(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;
        entry.1 = VirtualMachineState::Paused;
        Ok(())
    }

    async fn resume_vm(&self, vm_id: &str) -> Result<(), VirtualizationError> {
        let mut entry = self
            .vms
            .get_mut(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;
        entry.1 = VirtualMachineState::Running;
        Ok(())
    }

    async fn restart_vm(&self, vm_id: &str) -> Result<(), VirtualizationError> {
        self.stop_vm(vm_id).await?;
        self.start_vm(vm_id).await
    }

    async fn shutdown_vm(&self, vm_id: &str) -> Result<(), VirtualizationError> {
        self.stop_vm(vm_id).await
    }

    async fn force_stop_vm(&self, vm_id: &str) -> Result<(), VirtualizationError> {
        self.stop_vm(vm_id).await
    }

    async fn delete_vm(&self, vm_id: &str) -> Result<(), VirtualizationError> {
        self.vms
            .remove(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;
        Ok(())
    }

    async fn snapshot_vm(
        &self,
        vm_id: &str,
        name: &str,
    ) -> Result<VirtualMachineSnapshot, VirtualizationError> {
        if !self.vms.contains_key(vm_id) {
            return Err(VirtualizationError::NotFound(vm_id.to_string()));
        }
        let snap_id = format!("snap-mock-{}", uuid::Uuid::now_v7());
        let snap = VirtualMachineSnapshot {
            snapshot_id: snap_id.clone(),
            vm_id: vm_id.to_string(),
            name: name.to_string(),
            created_at: 1000,
            size_bytes: 1024,
        };
        self.snapshots.insert(snap_id, snap.clone());
        Ok(snap)
    }

    async fn restore_snapshot(
        &self,
        vm_id: &str,
        snapshot_id: &str,
    ) -> Result<(), VirtualizationError> {
        if !self.vms.contains_key(vm_id) {
            return Err(VirtualizationError::NotFound(vm_id.to_string()));
        }
        if !self.snapshots.contains_key(snapshot_id) {
            return Err(VirtualizationError::NotFound(snapshot_id.to_string()));
        }
        Ok(())
    }

    async fn clone_vm(&self, vm_id: &str, new_name: &str) -> Result<String, VirtualizationError> {
        let entry = self
            .vms
            .get(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;
        let new_id = format!("vm-clone-{}", uuid::Uuid::now_v7());
        let mut cfg = entry.0.clone();
        cfg.vm_id = new_id.clone();
        cfg.name = new_name.to_string();
        self.vms
            .insert(new_id.clone(), (cfg, VirtualMachineState::Stopped));
        Ok(new_id)
    }

    async fn inspect_vm(&self, vm_id: &str) -> Result<VirtualMachineConfig, VirtualizationError> {
        let entry = self
            .vms
            .get(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;
        Ok(entry.0.clone())
    }

    async fn get_state(&self, vm_id: &str) -> Result<VirtualMachineState, VirtualizationError> {
        let entry = self
            .vms
            .get(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;
        Ok(entry.1.clone())
    }

    async fn attach_device(
        &self,
        vm_id: &str,
        device: VirtualMachineDevice,
    ) -> Result<(), VirtualizationError> {
        let mut entry = self
            .vms
            .get_mut(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;
        entry.0.devices.push(device);
        Ok(())
    }

    async fn detach_device(&self, vm_id: &str, device_id: &str) -> Result<(), VirtualizationError> {
        let mut entry = self
            .vms
            .get_mut(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;
        entry.0.devices.retain(|d| d.device_id != device_id);
        Ok(())
    }

    async fn resize_resources(
        &self,
        vm_id: &str,
        new_resources: VirtualMachineResource,
    ) -> Result<(), VirtualizationError> {
        let mut entry = self
            .vms
            .get_mut(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;
        entry.0.resources = new_resources;
        Ok(())
    }

    async fn get_metrics(&self, vm_id: &str) -> Result<VirtualMachineMetrics, VirtualizationError> {
        if !self.vms.contains_key(vm_id) {
            return Err(VirtualizationError::NotFound(vm_id.to_string()));
        }
        Ok(VirtualMachineMetrics {
            cpu_usage_percent: 5.0,
            memory_used_mb: 512,
            disk_read_bytes: 1000,
            disk_write_bytes: 1000,
            net_rx_bytes: 500,
            net_tx_bytes: 500,
        })
    }

    async fn get_logs(&self, vm_id: &str) -> Result<Vec<String>, VirtualizationError> {
        if !self.vms.contains_key(vm_id) {
            return Err(VirtualizationError::NotFound(vm_id.to_string()));
        }
        Ok(vec!["[Mock] VM ready".to_string()])
    }

    async fn send_signal(&self, vm_id: &str, _signal: i32) -> Result<(), VirtualizationError> {
        if !self.vms.contains_key(vm_id) {
            return Err(VirtualizationError::NotFound(vm_id.to_string()));
        }
        Ok(())
    }

    async fn connect_console(&self, vm_id: &str) -> Result<String, VirtualizationError> {
        if !self.vms.contains_key(vm_id) {
            return Err(VirtualizationError::NotFound(vm_id.to_string()));
        }
        Ok(format!("mock-console://{}", vm_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_mock_backend_lifecycle() {
        let backend = MockBackend::new();
        let cfg = VirtualMachineConfig {
            vm_id: "test-vm-1".to_string(),
            name: "Test VM".to_string(),
            os_type: "linux".to_string(),
            resources: Default::default(),
            storage: VirtualMachineStorage {
                disk_path: PathBuf::from("/tmp/test.qcow2"),
                format: "qcow2".to_string(),
                size_gb: 20,
            },
            network: VirtualMachineNetwork {
                mode: "nat".to_string(),
                mac_address: "52:54:00:12:34:56".to_string(),
                ip_address: Some("192.168.122.100".to_string()),
            },
            uefi: true,
            tpm: false,
            secure_boot: false,
            devices: vec![],
        };

        let id = backend.create_vm(cfg).await.unwrap();
        assert_eq!(id, "test-vm-1");

        assert_eq!(
            backend.get_state(&id).await.unwrap(),
            VirtualMachineState::Stopped
        );
        backend.start_vm(&id).await.unwrap();
        assert_eq!(
            backend.get_state(&id).await.unwrap(),
            VirtualMachineState::Running
        );

        let snap = backend.snapshot_vm(&id, "snap1").await.unwrap();
        assert_eq!(snap.vm_id, "test-vm-1");

        backend.stop_vm(&id).await.unwrap();
        assert_eq!(
            backend.get_state(&id).await.unwrap(),
            VirtualMachineState::Stopped
        );
    }
}
