use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ShellError {
    #[error("kernel error: {0}")]
    Kernel(String),
    #[error("approval required: {0}")]
    ApprovalRequired(String),
    #[error("approval denied: {0}")]
    Denied(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("workspace: {0}")]
    Workspace(String),
}

pub type ShellResult<T> = Result<T, ShellError>;
