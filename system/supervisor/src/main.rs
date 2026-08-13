use cognyx_supervisor::{ProcessSupervisor, ServiceDag, ServiceManifest};
use std::env;
use std::fs;
use std::path::PathBuf;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    info!("Starting CognyxOS System Supervisor (PID 1 Manager)...");

    let manifest_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./config/services"));

    let mut manifests = Vec::new();

    if manifest_dir.exists() {
        if let Ok(entries) = fs::read_dir(&manifest_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        match ServiceManifest::from_toml(&content) {
                            Ok(m) => manifests.push(m),
                            Err(e) => error!("Failed to parse manifest at {:?}: {}", path, e),
                        }
                    }
                }
            }
        }
    } else {
        info!(
            "No service manifest directory found at {:?}. Running empty supervisor loop.",
            manifest_dir
        );
    }

    if !manifests.is_empty() {
        match ServiceDag::new(manifests) {
            Ok(dag) => {
                let sorted_manifests = dag.topological_sort()?;
                info!("Dependency resolution complete. Launch order:");
                for (idx, m) in sorted_manifests.iter().enumerate() {
                    info!("  {}. {} (deps: {:?})", idx + 1, m.name, m.dependencies);
                }

                let supervisor = ProcessSupervisor::new();
                for m in sorted_manifests {
                    let _ = supervisor.spawn_service(m).await;
                }
                supervisor.monitor_services().await;
            }
            Err(e) => error!("DAG construction error: {}", e),
        }
    }

    // Keep supervisor running
    tokio::signal::ctrl_c().await?;
    info!("CognyxOS System Supervisor shutting down.");
    Ok(())
}
