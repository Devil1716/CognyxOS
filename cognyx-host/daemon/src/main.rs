//! CognyxOS Host Daemon (cognyxd)
//!
//! Privileged daemon responsible for:
//! - VM creation and lifecycle management
//! - Network bridge and namespace management
//! - Storage pool and volume management
//! - Device assignment (GPU passthrough)
//! - Resource limits and policy enforcement
//! - Audit logging

mod config;
mod error;
mod network;
mod storage;
mod vm;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::signal;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::error::Result;

#[derive(Clone)]
pub struct DaemonState {
    pub config: Arc<Config>,
    pub socket_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!("CognyxOS Host Daemon starting");

    // Load configuration
    let config_path = std::env::var("COGNYX_CONFIG")
        .unwrap_or_else(|_| "/etc/cognyx/config.yaml".to_string());
    
    let config = Config::load(&config_path).unwrap_or_else(|e| {
        warn!("Failed to load config from {}: {}, using defaults", config_path, e);
        Config::default()
    });

    let socket_path = PathBuf::from(&config.api.socket_path);

    // Create runtime directory if it doesn't exist
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let state = DaemonState {
        config: Arc::new(config),
        socket_path,
    };

    info!("Configuration loaded successfully");
    info!("Machine mode: {}", state.config.host.mode);
    info!("Hypervisor: {}", state.config.virtualization.hypervisor);

    // Start Unix domain socket server
    let listener = tokio::net::UnixListener::bind(&state.socket_path)?;
    info!("Listening on {}", state.socket_path.display());

    // Spawn API handler
    let api_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = run_api_server(listener, api_state).await {
            error!("API server failed: {}", e);
        }
    });

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal");
        }
        Err(e) => {
            error!("Failed to listen for shutdown signal: {}", e);
        }
    }

    // Cleanup
    cleanup(&state).await;
    
    info!("CognyxOS Host Daemon stopped");
    Ok(())
}

async fn run_api_server(
    listener: tokio::net::UnixListener,
    state: DaemonState,
) -> Result<()> {
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, state).await {
                        error!("Connection handler error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}

async fn handle_connection(
    _stream: tokio::net::UnixStream,
    _state: DaemonState,
) -> Result<()> {
    // TODO: Implement request parsing and dispatch
    // For now, just keep connection open
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    Ok(())
}

async fn cleanup(state: &DaemonState) {
    // Remove socket file
    if let Err(e) = tokio::fs::remove_file(&state.socket_path).await {
        warn!("Failed to remove socket file: {}", e);
    }
    
    info!("Cleanup complete");
}
