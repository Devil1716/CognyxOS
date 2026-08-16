use cognyx_execution::native_host_runtime_id;
use cognyx_service_workspace::{ctx_for, WorkspaceManager};
use cognyx_shell::{AgentKernelAdapter, CognyxShell};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    info!("Launching CognyxOS shell");

    // Production path: AgentKernelServer via adapter. RecordingKernel is TEST ONLY.
    let kernel = Arc::new(AgentKernelAdapter::new());
    let host_id = native_host_runtime_id();
    let registry = Arc::clone(&kernel.server().registry);
    let workspace = Arc::new(WorkspaceManager::new(registry));
    workspace.attach_dedicated_host_filesystem(host_id)?;
    let ctx = ctx_for("user", &["filesystem.write", "filesystem.read"]);
    let ws = workspace.create_workspace("home", "user", host_id, &ctx)?;

    let shell = CognyxShell::new(kernel, workspace, ws.id);
    info!(dock = ?shell.dock(), runtime = host_id, "Shell ready");
    Ok(())
}
