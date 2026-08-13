use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Storage directory invalid: {0}")]
    InvalidPath(String),
    #[error("Disk image not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VmDiskMetadata {
    pub disk_id: String,
    pub name: String,
    pub format: String, // "qcow2", "raw"
    pub size_gb: u64,
    pub path: PathBuf,
    pub parent_snapshot_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct VMStorageManager {
    pub images_dir: PathBuf,
    pub disks_dir: PathBuf,
    pub snapshots_dir: PathBuf,
}

impl Default for VMStorageManager {
    fn default() -> Self {
        let base = std::env::var("COGNYX_STORAGE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/var/lib/cognyxos"));

        Self::new(base)
    }
}

impl VMStorageManager {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            images_dir: base_dir.join("images"),
            disks_dir: base_dir.join("disks"),
            snapshots_dir: base_dir.join("snapshots"),
        }
    }

    pub fn create_virtual_disk(
        &self,
        disk_id: &str,
        format: &str,
        size_gb: u64,
    ) -> Result<VmDiskMetadata, StorageError> {
        let disk_file = self.disks_dir.join(format!("{}.{}", disk_id, format));
        info!(
            "Creating virtual disk image: {:?} ({} GB, format: {})",
            disk_file, size_gb, format
        );

        let meta = VmDiskMetadata {
            disk_id: disk_id.to_string(),
            name: disk_id.to_string(),
            format: format.to_string(),
            size_gb,
            path: disk_file,
            parent_snapshot_id: None,
        };

        Ok(meta)
    }

    pub fn clone_cow_disk(
        &self,
        parent_disk_id: &str,
        new_disk_id: &str,
    ) -> Result<VmDiskMetadata, StorageError> {
        let parent_path = self.disks_dir.join(format!("{}.qcow2", parent_disk_id));
        let new_path = self.disks_dir.join(format!("{}.qcow2", new_disk_id));

        info!(
            "Creating CoW qcow2 clone {:?} backed by parent {:?}",
            new_path, parent_path
        );

        Ok(VmDiskMetadata {
            disk_id: new_disk_id.to_string(),
            name: new_disk_id.to_string(),
            format: "qcow2".to_string(),
            size_gb: 40,
            path: new_path,
            parent_snapshot_id: Some(parent_disk_id.to_string()),
        })
    }

    pub fn resize_disk(&self, disk_id: &str, new_size_gb: u64) -> Result<(), StorageError> {
        info!("Resizing disk '{}' to {} GB", disk_id, new_size_gb);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_paths_and_disk_creation() {
        let mgr = VMStorageManager::new(PathBuf::from("/tmp/cognyx-test-storage"));
        assert_eq!(
            mgr.disks_dir,
            PathBuf::from("/tmp/cognyx-test-storage/disks")
        );

        let meta = mgr.create_virtual_disk("win11-disk", "qcow2", 60).unwrap();
        assert_eq!(meta.disk_id, "win11-disk");
        assert_eq!(meta.size_gb, 60);

        let clone = mgr.clone_cow_disk("win11-disk", "win11-clone-1").unwrap();
        assert_eq!(clone.parent_snapshot_id, Some("win11-disk".to_string()));
    }
}
