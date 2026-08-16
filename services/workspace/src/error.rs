use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorkspaceError {
    #[error("workspace not found: {0}")]
    WorkspaceNotFound(String),
    #[error("item not found: {0}")]
    ItemNotFound(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("user approval required for capability: {0}")]
    ApprovalRequired(String),
    #[error("RUNTIME_UNAVAILABLE: {0}")]
    RuntimeUnavailable(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("checksum mismatch: {0}")]
    ChecksumMismatch(String),
    #[error("partial transfer: {0}")]
    PartialTransfer(String),
    #[error("invalid logical path: {0}")]
    InvalidPath(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("version not found: {0}")]
    VersionNotFound(String),
    #[error("io error: {0}")]
    Io(String),
}

pub type WorkspaceResult<T> = Result<T, WorkspaceError>;
