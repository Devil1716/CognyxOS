use cognyx_execution::{LinuxRuntime, RuntimeRegistry};
use cognyx_service_workspace::{ctx_for, WorkspaceManager};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    info!("Starting CognyxOS Workspace Manager Service...");

    let registry = Arc::new(RuntimeRegistry::new());
    registry.register(Box::new(LinuxRuntime::new("linux-host", "Linux host")));
    let manager = WorkspaceManager::new(Arc::clone(&registry));
    manager.attach_in_memory_runtime("linux-host");

    let ctx = ctx_for("system", &["filesystem.write", "filesystem.read"]);
    let ws = manager.create_workspace("default", "system", "linux-host", &ctx)?;
    info!(workspace_id = %ws.id, "Default workspace ready");

    Ok(())
}
