mod gui;

use cognyx_execution::native_host_runtime_id;
use cognyx_service_workspace::{ctx_for, WorkspaceManager};
use cognyx_shell::{AgentKernelAdapter, CognyxShell};
use std::sync::Arc;

fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let host_id = native_host_runtime_id();
    let (shell, workspace_id) = runtime
        .block_on(async {
            let kernel = Arc::new(AgentKernelAdapter::new());
            let registry = Arc::clone(&kernel.server().registry);
            let workspace = Arc::new(WorkspaceManager::new(registry));
            workspace.attach_dedicated_host_filesystem(host_id)?;
            let ctx = ctx_for("user", &["filesystem.write", "filesystem.read"]);
            let ws = workspace.create_workspace("home", "user", host_id, &ctx)?;
            Ok::<_, Box<dyn std::error::Error>>((
                Arc::new(CognyxShell::new(kernel, workspace, ws.id.clone())),
                ws.id,
            ))
        })
        .expect("shell bootstrap");

    gui::run(runtime, shell, host_id.to_string(), workspace_id)
}
