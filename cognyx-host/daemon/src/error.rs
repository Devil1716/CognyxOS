//! Error handling module for cognyxd

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("VM error: {0}")]
    VM(String),
    
    #[error("Permission denied: {0}")]
    Permission(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Timeout")]
    Timeout,
    
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, DaemonError>;
