use crate::model::{WorkerError, WorkerResult};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct ArtifactBlob {
    pub artifact_id: String,
    pub bytes: Vec<u8>,
    pub checksum: String,
    pub encrypted: bool,
}

impl ArtifactBlob {
    pub fn new(artifact_id: impl Into<String>, bytes: Vec<u8>) -> Self {
        let checksum = format!("{:x}", Sha256::digest(&bytes));
        Self {
            artifact_id: artifact_id.into(),
            bytes,
            checksum,
            encrypted: true,
        }
    }
}

pub struct ArtifactTransfer {
    store: Mutex<HashMap<String, ArtifactBlob>>,
    inflight: Mutex<HashMap<String, bool>>,
}

impl Default for ArtifactTransfer {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactTransfer {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
            inflight: Mutex::new(HashMap::new()),
        }
    }

    pub fn put(&self, blob: ArtifactBlob) -> WorkerResult<()> {
        if blob.checksum != format!("{:x}", Sha256::digest(&blob.bytes)) {
            return Err(WorkerError::Integrity(blob.artifact_id));
        }
        self.store
            .lock()
            .unwrap()
            .insert(blob.artifact_id.clone(), blob);
        Ok(())
    }

    pub fn get(&self, artifact_id: &str) -> WorkerResult<ArtifactBlob> {
        self.store
            .lock()
            .unwrap()
            .get(artifact_id)
            .cloned()
            .ok_or_else(|| WorkerError::NotFound(artifact_id.to_string()))
    }

    pub fn resume(&self, artifact_id: &str, blob: ArtifactBlob) -> WorkerResult<()> {
        self.inflight
            .lock()
            .unwrap()
            .insert(artifact_id.to_string(), true);
        self.put(blob)?;
        self.inflight.lock().unwrap().remove(artifact_id);
        Ok(())
    }
}
