use crate::quota::{ResourceMetrics, ResourceQuota, ResourceReservation};
use dashmap::DashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum ResourceError {
    #[error("Resource quota exceeded: requested {0}, available {1}")]
    QuotaExceeded(String, String),
    #[error("Reservation not found: {0}")]
    NotFound(String),
}

pub struct ResourceManager {
    quota: ResourceQuota,
    reservations: Arc<DashMap<String, ResourceReservation>>,
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new(ResourceQuota::default())
    }
}

impl ResourceManager {
    pub fn new(quota: ResourceQuota) -> Self {
        Self {
            quota,
            reservations: Arc::new(DashMap::new()),
        }
    }

    pub fn reserve(&self, reservation: ResourceReservation) -> Result<(), ResourceError> {
        let metrics = self.get_current_usage();

        if metrics.allocated_cpus + reservation.cpus > self.quota.max_cpus {
            return Err(ResourceError::QuotaExceeded(
                format!("{} CPUs", reservation.cpus),
                format!(
                    "{} CPUs left",
                    self.quota.max_cpus.saturating_sub(metrics.allocated_cpus)
                ),
            ));
        }

        if metrics.allocated_memory_mb + reservation.memory_mb > self.quota.max_memory_mb {
            return Err(ResourceError::QuotaExceeded(
                format!("{} MB RAM", reservation.memory_mb),
                format!(
                    "{} MB RAM left",
                    self.quota
                        .max_memory_mb
                        .saturating_sub(metrics.allocated_memory_mb)
                ),
            ));
        }

        info!(
            "Reserving resources for runtime '{}': {} CPUs, {} MB RAM",
            reservation.runtime_id, reservation.cpus, reservation.memory_mb
        );
        self.reservations
            .insert(reservation.reservation_id.clone(), reservation);
        Ok(())
    }

    pub fn release(&self, reservation_id: &str) -> Result<(), ResourceError> {
        self.reservations
            .remove(reservation_id)
            .ok_or_else(|| ResourceError::NotFound(reservation_id.to_string()))?;
        info!("Released resource reservation '{}'", reservation_id);
        Ok(())
    }

    pub fn get_current_usage(&self) -> ResourceMetrics {
        let mut total_cpus = 0;
        let mut total_mem = 0;
        let mut total_storage = 0;

        for entry in self.reservations.iter() {
            let res = entry.value();
            total_cpus += res.cpus;
            total_mem += res.memory_mb;
            total_storage += res.storage_gb;
        }

        ResourceMetrics {
            allocated_cpus: total_cpus,
            allocated_memory_mb: total_mem,
            allocated_storage_gb: total_storage,
            active_vms: self.reservations.len() as u32,
            active_containers: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_quota_enforcement() {
        let manager = ResourceManager::new(ResourceQuota {
            max_cpus: 4,
            max_memory_mb: 8192,
            max_storage_gb: 100,
            max_gpus: 1,
            max_vms: 2,
            max_containers: 5,
        });

        let res1 = ResourceReservation {
            reservation_id: "res-1".to_string(),
            runtime_id: "win-vm-1".to_string(),
            cpus: 4,
            memory_mb: 4096,
            storage_gb: 40,
            gpus: 0,
        };
        assert!(manager.reserve(res1).is_ok());

        // Attempting to reserve 2 more CPUs when 4 are already used (max 4) must fail
        let res2 = ResourceReservation {
            reservation_id: "res-2".to_string(),
            runtime_id: "win-vm-2".to_string(),
            cpus: 2,
            memory_mb: 2048,
            storage_gb: 20,
            gpus: 0,
        };
        let err = manager.reserve(res2);
        assert!(err.is_err());
    }
}
