use cognyx_virtualization::{
    MockBackend, VirtualMachineConfig, VirtualMachineNetwork, VirtualMachineResource,
    VirtualMachineState, VirtualMachineStorage, VirtualizationBackend,
};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

pub struct WindowsVmManager {
    backend: Arc<dyn VirtualizationBackend>,
}

impl Default for WindowsVmManager {
    fn default() -> Self {
        Self::new(Arc::new(MockBackend::new()))
    }
}

impl WindowsVmManager {
    pub fn new(backend: Arc<dyn VirtualizationBackend>) -> Self {
        Self { backend }
    }

    pub async fn create_windows_vm(
        &self,
        name: &str,
        cpus: u32,
        memory_mb: u64,
        disk_path: PathBuf,
    ) -> Result<String, String> {
        let vm_id = format!("win-vm-{}", uuid::Uuid::now_v7());
        info!("Creating Windows VM '{}' ({})", name, vm_id);

        let config = VirtualMachineConfig {
            vm_id: vm_id.clone(),
            name: name.to_string(),
            os_type: "windows".to_string(),
            resources: VirtualMachineResource {
                cpus,
                memory_mb,
                disk_gb: 60,
                vram_mb: 256,
                gpu_passthrough: false,
            },
            storage: VirtualMachineStorage {
                disk_path,
                format: "qcow2".to_string(),
                size_gb: 60,
            },
            network: VirtualMachineNetwork {
                mode: "nat".to_string(),
                mac_address: "52:54:00:ab:cd:ef".to_string(),
                ip_address: Some("192.168.122.50".to_string()),
            },
            uefi: true,
            tpm: true,
            secure_boot: true,
            devices: vec![],
        };

        self.backend
            .create_vm(config)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn start_windows_vm(&self, vm_id: &str) -> Result<(), String> {
        if self.backend.get_state(vm_id).await.is_err() {
            let config = VirtualMachineConfig {
                vm_id: vm_id.to_string(),
                name: vm_id.to_string(),
                os_type: "windows".to_string(),
                resources: VirtualMachineResource {
                    cpus: 4,
                    memory_mb: 8192,
                    disk_gb: 60,
                    vram_mb: 256,
                    gpu_passthrough: false,
                },
                storage: VirtualMachineStorage {
                    disk_path: PathBuf::from("/var/lib/cognyxos/disks/win.qcow2"),
                    format: "qcow2".to_string(),
                    size_gb: 60,
                },
                network: VirtualMachineNetwork {
                    mode: "nat".to_string(),
                    mac_address: "52:54:00:11:22:33".to_string(),
                    ip_address: Some("192.168.122.50".to_string()),
                },
                uefi: true,
                tpm: true,
                secure_boot: true,
                devices: vec![],
            };
            let _ = self.backend.create_vm(config).await;
        }
        self.backend
            .start_vm(vm_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn stop_windows_vm(&self, vm_id: &str) -> Result<(), String> {
        self.backend.stop_vm(vm_id).await.map_err(|e| e.to_string())
    }

    pub async fn pause_windows_vm(&self, vm_id: &str) -> Result<(), String> {
        self.backend
            .pause_vm(vm_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn resume_windows_vm(&self, vm_id: &str) -> Result<(), String> {
        self.backend
            .resume_vm(vm_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn snapshot_windows_vm(&self, vm_id: &str, name: &str) -> Result<String, String> {
        let snap = self
            .backend
            .snapshot_vm(vm_id, name)
            .await
            .map_err(|e| e.to_string())?;
        Ok(snap.snapshot_id)
    }

    pub async fn restore_windows_vm(&self, vm_id: &str, snapshot_id: &str) -> Result<(), String> {
        self.backend
            .restore_snapshot(vm_id, snapshot_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_windows_status(&self, vm_id: &str) -> Result<VirtualMachineState, String> {
        self.backend
            .get_state(vm_id)
            .await
            .map_err(|e| e.to_string())
    }
}
