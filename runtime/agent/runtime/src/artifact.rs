use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactType {
    File,
    Text,
    Json,
    Image,
    Screenshot,
    Table,
    Dataset,
    Report,
    ApplicationState,
    BrowserResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub artifact_id: String,
    pub owner_agent_id: String,
    pub task_id: String,
    pub artifact_type: ArtifactType,
    pub location: String,
    pub metadata: serde_json::Value,
    pub permissions: Vec<String>,
    pub created_at: u64,
    pub checksum: String,
}

impl Artifact {
    pub fn new(
        artifact_id: impl Into<String>,
        owner_agent_id: impl Into<String>,
        task_id: impl Into<String>,
        artifact_type: ArtifactType,
        location: impl Into<String>,
        metadata: serde_json::Value,
    ) -> Self {
        let id = artifact_id.into();
        let loc = location.into();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let checksum = format!("{:x}", sha2::Sha256::digest(loc.as_bytes()));

        Self {
            artifact_id: id,
            owner_agent_id: owner_agent_id.into(),
            task_id: task_id.into(),
            artifact_type,
            location: loc,
            metadata,
            permissions: vec!["read".into()],
            created_at: now,
            checksum,
        }
    }
}

pub struct ArtifactExchange {
    artifacts: DashMap<String, Arc<Artifact>>,
}

impl Default for ArtifactExchange {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactExchange {
    pub fn new() -> Self {
        Self {
            artifacts: DashMap::new(),
        }
    }

    pub fn register_artifact(&self, artifact: Artifact) -> Result<(), String> {
        self.artifacts
            .insert(artifact.artifact_id.clone(), Arc::new(artifact));
        Ok(())
    }

    pub fn get_artifact(
        &self,
        artifact_id: &str,
        _requester_agent_id: &str,
        _task_id: &str,
    ) -> Option<Arc<Artifact>> {
        self.artifacts.get(artifact_id).map(|a| a.clone())
    }
}
