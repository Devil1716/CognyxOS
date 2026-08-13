use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub max_cpus: u32,
    pub max_memory_mb: u64,
    pub max_storage_gb: u64,
    pub max_gpus: u32,
    pub max_vms: u32,
    pub max_containers: u32,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            max_cpus: 16,
            max_memory_mb: 32768,
            max_storage_gb: 500,
            max_gpus: 2,
            max_vms: 5,
            max_containers: 20,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceReservation {
    pub reservation_id: String,
    pub runtime_id: String,
    pub cpus: u32,
    pub memory_mb: u64,
    pub storage_gb: u64,
    pub gpus: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub allocated_cpus: u32,
    pub allocated_memory_mb: u64,
    pub allocated_storage_gb: u64,
    pub active_vms: u32,
    pub active_containers: u32,
}
