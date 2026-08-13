use crate::manifest::{RestartPolicy, ServiceManifest};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

pub struct ManagedProcess {
    pub manifest: ServiceManifest,
    pub child: Option<Child>,
    pub restart_count: u32,
}

#[derive(Default)]
pub struct ProcessSupervisor {
    processes: Arc<RwLock<HashMap<String, ManagedProcess>>>,
}

impl ProcessSupervisor {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn spawn_service(&self, manifest: ServiceManifest) -> Result<(), String> {
        info!("Spawning supervised service: '{}'", manifest.name);

        let mut cmd = Command::new(&manifest.binary_path);
        cmd.args(&manifest.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (k, v) in &manifest.env {
            cmd.env(k, v);
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn process '{}': {}", manifest.name, e))?;

        let name = manifest.name.clone();
        let managed = ManagedProcess {
            manifest,
            child: Some(child),
            restart_count: 0,
        };

        let mut lock = self.processes.write().await;
        lock.insert(name, managed);
        Ok(())
    }

    pub async fn monitor_services(&self) {
        let processes = self.processes.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                let mut lock = processes.write().await;

                for (name, managed) in lock.iter_mut() {
                    if let Some(child) = managed.child.as_mut() {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                warn!("Service '{}' exited with status: {}", name, status);
                                let should_restart = match managed.manifest.restart_policy {
                                    RestartPolicy::Always => true,
                                    RestartPolicy::OnFailure => !status.success(),
                                    RestartPolicy::Never => false,
                                };

                                if should_restart {
                                    managed.restart_count += 1;
                                    info!(
                                        "Restarting service '{}' (attempt #{})",
                                        name, managed.restart_count
                                    );

                                    let mut cmd = Command::new(&managed.manifest.binary_path);
                                    cmd.args(&managed.manifest.args);
                                    for (k, v) in &managed.manifest.env {
                                        cmd.env(k, v);
                                    }
                                    if let Ok(new_child) = cmd.spawn() {
                                        managed.child = Some(new_child);
                                    } else {
                                        error!("Failed to restart service '{}'", name);
                                    }
                                }
                            }
                            Ok(None) => {
                                // Still running
                            }
                            Err(e) => {
                                error!("Error checking service '{}' status: {}", name, e);
                            }
                        }
                    }
                }
            }
        });
    }
}
