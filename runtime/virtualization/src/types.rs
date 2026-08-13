use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum VirtualMachineState {
    Stopped,
    Starting,
    Running,
    Paused,
    Stopping,
    Failed(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtualMachineResource {
    pub cpus: u32,
    pub memory_mb: u64,
    pub disk_gb: u64,
    pub vram_mb: u64,
    pub gpu_passthrough: bool,
}

impl Default for VirtualMachineResource {
    fn default() -> Self {
        Self {
            cpus: 2,
            memory_mb: 4096,
            disk_gb: 40,
            vram_mb: 128,
            gpu_passthrough: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtualMachineNetwork {
    pub mode: String, // "nat", "bridge", "isolated"
    pub mac_address: String,
    pub ip_address: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtualMachineStorage {
    pub disk_path: PathBuf,
    pub format: String, // "qcow2", "raw"
    pub size_gb: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtualMachineDevice {
    pub device_id: String,
    pub device_type: String, // "usb", "pci", "virtio"
    pub host_address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtualMachineConfig {
    pub vm_id: String,
    pub name: String,
    pub os_type: String, // "windows", "linux", "macos"
    pub resources: VirtualMachineResource,
    pub storage: VirtualMachineStorage,
    pub network: VirtualMachineNetwork,
    pub uefi: bool,
    pub tpm: bool,
    pub secure_boot: bool,
    pub devices: Vec<VirtualMachineDevice>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtualMachineSnapshot {
    pub snapshot_id: String,
    pub vm_id: String,
    pub name: String,
    pub created_at: u64,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtualMachineMetrics {
    pub cpu_usage_percent: f64,
    pub memory_used_mb: u64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
}
