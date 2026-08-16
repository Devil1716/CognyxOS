use crate::backend::{VirtualizationBackend, VirtualizationError};
use crate::types::*;
use async_trait::async_trait;
use dashmap::DashMap;
use std::process::Stdio;
use tokio::process::Command;
use tracing::info;

pub struct KvmBackend {
    vms: DashMap<String, (VirtualMachineConfig, VirtualMachineState)>,
}

impl Default for KvmBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl KvmBackend {
    pub fn new() -> Self {
        Self {
            vms: DashMap::new(),
        }
    }

    pub fn build_qemu_args(config: &VirtualMachineConfig) -> Vec<String> {
        let mut args = vec![
            "-enable-kvm".to_string(),
            "-m".to_string(),
            format!("{}", config.resources.memory_mb),
            "-smp".to_string(),
            format!("{}", config.resources.cpus),
            "-drive".to_string(),
            format!(
                "file={},format={},if=virtio",
                config.storage.disk_path.display(),
                config.storage.format
            ),
        ];

        if config.uefi {
            args.push("-bios".to_string());
            args.push("/usr/share/OVMF/OVMF_CODE.fd".to_string());
        }

        if config.network.mode == "nat" {
            args.push("-netdev".to_string());
            args.push("user,id=net0".to_string());
            args.push("-device".to_string());
            args.push("virtio-net-pci,netdev=net0".to_string());
        }

        args
    }
}

#[async_trait]
impl VirtualizationBackend for KvmBackend {
    async fn create_vm(&self, config: VirtualMachineConfig) -> Result<String, VirtualizationError> {
        let vm_id = config.vm_id.clone();
        info!("KvmBackend: Creating VM '{}' ({})", config.name, vm_id);
        self.vms
            .insert(vm_id.clone(), (config, VirtualMachineState::Stopped));
        Ok(vm_id)
    }

    async fn start_vm(&self, vm_id: &str) -> Result<(), VirtualizationError> {
        let mut entry = self
            .vms
            .get_mut(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;

        info!("KvmBackend: Starting VM '{}'", vm_id);
        let qemu_args = Self::build_qemu_args(&entry.0);

        let _ = Command::new("qemu-system-x86_64")
            .args(&qemu_args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        entry.1 = VirtualMachineState::Running;
        Ok(())
    }

    async fn stop_vm(&self, vm_id: &str) -> Result<(), VirtualizationError> {
        let mut entry = self
            .vms
            .get_mut(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;

        info!("KvmBackend: Stopping VM '{}'", vm_id);
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
        let snap_id = format!("snap-{}", uuid::Uuid::now_v7());
        Ok(VirtualMachineSnapshot {
            snapshot_id: snap_id,
            vm_id: vm_id.to_string(),
            name: name.to_string(),
            created_at: 1000,
            size_bytes: 1024 * 1024,
        })
    }

    async fn restore_snapshot(
        &self,
        vm_id: &str,
        _snapshot_id: &str,
    ) -> Result<(), VirtualizationError> {
        if !self.vms.contains_key(vm_id) {
            return Err(VirtualizationError::NotFound(vm_id.to_string()));
        }
        Ok(())
    }

    async fn clone_vm(&self, vm_id: &str, new_name: &str) -> Result<String, VirtualizationError> {
        let entry = self
            .vms
            .get(vm_id)
            .ok_or_else(|| VirtualizationError::NotFound(vm_id.to_string()))?;

        let new_id = format!("vm-{}", uuid::Uuid::now_v7());
        let mut new_config = entry.0.clone();
        new_config.vm_id = new_id.clone();
        new_config.name = new_name.to_string();

        self.vms
            .insert(new_id.clone(), (new_config, VirtualMachineState::Stopped));
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
            cpu_usage_percent: 12.5,
            memory_used_mb: 1024,
            disk_read_bytes: 50000,
            disk_write_bytes: 12000,
            net_rx_bytes: 4000,
            net_tx_bytes: 2000,
        })
    }

    async fn get_logs(&self, vm_id: &str) -> Result<Vec<String>, VirtualizationError> {
        if !self.vms.contains_key(vm_id) {
            return Err(VirtualizationError::NotFound(vm_id.to_string()));
        }
        Ok(vec![
            "[KVM] VM initialized".to_string(),
            "[KVM] Booting kernel".to_string(),
        ])
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
        Ok(format!("/run/cognyxos/vm-{}.sock", vm_id))
    }
}
