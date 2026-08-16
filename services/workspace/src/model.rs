use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const LOGICAL_ROOT: &str = "/Workspace";
pub const LOGICAL_PROJECTS: &str = "/Workspace/Projects";
pub const LOGICAL_DOCUMENTS: &str = "/Workspace/Documents";
pub const LOGICAL_DOWNLOADS: &str = "/Workspace/Downloads";
pub const LOGICAL_APPLICATIONS: &str = "/Workspace/Applications";
pub const LOGICAL_TASKS: &str = "/Workspace/Tasks";
pub const LOGICAL_ARTIFACTS: &str = "/Workspace/Artifacts";

pub fn default_logical_layout() -> [&'static str; 7] {
    [
        LOGICAL_ROOT,
        LOGICAL_PROJECTS,
        LOGICAL_DOCUMENTS,
        LOGICAL_DOWNLOADS,
        LOGICAL_APPLICATIONS,
        LOGICAL_TASKS,
        LOGICAL_ARTIFACTS,
    ]
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::now_v7())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkspaceItemKind {
    Folder,
    File,
    Application,
    Task,
    Artifact,
    Reference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WorkspacePermission {
    pub owner: String,
    pub read: Vec<String>,
    pub write: Vec<String>,
    pub admin: Vec<String>,
}

impl WorkspacePermission {
    pub fn owner_only(owner: impl Into<String>) -> Self {
        let owner = owner.into();
        Self {
            owner: owner.clone(),
            read: vec![owner.clone()],
            write: vec![owner.clone()],
            admin: vec![owner],
        }
    }

    pub fn allows_read(&self, principal: &str) -> bool {
        self.owner == principal
            || self.read.iter().any(|p| p == principal || p == "*")
            || self.admin.iter().any(|p| p == principal)
    }

    pub fn allows_write(&self, principal: &str) -> bool {
        self.owner == principal
            || self.write.iter().any(|p| p == principal || p == "*")
            || self.admin.iter().any(|p| p == principal)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WorkspaceMetadata {
    pub labels: HashMap<String, String>,
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub permissions: WorkspacePermission,
    pub created_at: u64,
    pub modified_at: u64,
    pub metadata: WorkspaceMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceItem {
    pub id: String,
    pub kind: WorkspaceItemKind,
    pub name: String,
    pub location: String,
    pub runtime_id: String,
    pub owner: String,
    pub permissions: WorkspacePermission,
    pub created_at: u64,
    pub modified_at: u64,
    pub metadata: WorkspaceMetadata,
    pub parent_id: Option<String>,
    pub checksum: Option<String>,
    pub version: u64,
}

impl WorkspaceItem {
    pub fn item_type(&self) -> &WorkspaceItemKind {
        &self.kind
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceFolder;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceFile;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceApplication;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceTask;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceArtifact {
    pub item: WorkspaceItem,
    pub source_artifact_id: String,
    pub source_task_id: String,
    pub source_agent_id: String,
    pub shared_with: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceReference {
    pub workspace_id: String,
    pub item_id: String,
    pub runtime_id: String,
    pub physical_location: String,
    pub logical_location: String,
    pub permissions: WorkspacePermission,
    pub checksum: String,
    pub version: u64,
}

pub fn checksum_bytes(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(data))
}

pub fn physical_location(runtime_id: &str, logical: &str) -> String {
    format!("{runtime_id}:{logical}")
}

pub struct ArtifactIngestRequest<'a> {
    pub workspace_id: &'a str,
    pub runtime_id: &'a str,
    pub source_artifact_id: &'a str,
    pub source_task_id: &'a str,
    pub source_agent_id: &'a str,
    pub name: &'a str,
    pub data: &'a [u8],
}

pub fn validate_logical_path(path: &str) -> Result<(), crate::error::WorkspaceError> {
    if !path.starts_with(LOGICAL_ROOT) || path.contains("..") || path.contains('\\') {
        return Err(crate::error::WorkspaceError::InvalidPath(path.to_string()));
    }
    Ok(())
}
