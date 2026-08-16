use crate::model::*;
use crate::remote::RemoteWorkerRuntime;
use crate::transfer::ArtifactTransfer;
use cognyx_execution::RuntimeRegistry;
use cognyx_task_manager::{CheckpointEngine, CheckpointRequest, CheckpointState};
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub struct WorkerRegistry {
    runtime_registry: Arc<RuntimeRegistry>,
    workers: DashMap<String, Worker>,
    assigned_tasks: DashMap<String, String>,
    destructive_once: Mutex<HashSet<String>>,
    pub transfers: ArtifactTransfer,
}

impl WorkerRegistry {
    pub fn new(runtime_registry: Arc<RuntimeRegistry>) -> Self {
        Self {
            runtime_registry,
            workers: DashMap::new(),
            assigned_tasks: DashMap::new(),
            destructive_once: Mutex::new(HashSet::new()),
            transfers: ArtifactTransfer::new(),
        }
    }

    pub fn authenticate(&self, worker_id: &str, token: &str) -> WorkerResult<()> {
        let worker = self
            .workers
            .get(worker_id)
            .ok_or(WorkerError::AuthenticationFailure)?;
        if worker.identity.token != token {
            return Err(WorkerError::AuthenticationFailure);
        }
        Ok(())
    }

    pub fn authorize(&self, worker_id: &str, principal: &str) -> WorkerResult<()> {
        let worker = self
            .workers
            .get(worker_id)
            .ok_or_else(|| WorkerError::NotFound(worker_id.to_string()))?;
        if !worker.policy.allowed_principals.contains(principal) {
            return Err(WorkerError::AuthorizationFailure(principal.to_string()));
        }
        Ok(())
    }

    pub fn register(&self, worker: Worker) -> WorkerResult<String> {
        if !worker.tls_required {
            return Err(WorkerError::AuthorizationFailure(
                "tls required for worker communication".into(),
            ));
        }
        let id = worker.identity.worker_id.clone();
        let runtime = Box::new(RemoteWorkerRuntime::new(worker.clone()));
        self.runtime_registry.register(runtime);
        self.workers.insert(id.clone(), worker);
        Ok(id)
    }

    pub fn heartbeat(&self, worker_id: &str, token: &str) -> WorkerResult<WorkerHealth> {
        self.authenticate(worker_id, token)?;
        let mut worker = self
            .workers
            .get_mut(worker_id)
            .ok_or_else(|| WorkerError::NotFound(worker_id.to_string()))?;
        worker.last_heartbeat_secs = now_secs();
        worker.health = WorkerHealth::Healthy;
        worker.status = WorkerStatus::Online;
        Ok(WorkerHealth::Healthy)
    }

    pub fn disconnect(&self, worker_id: &str) {
        if let Some(mut w) = self.workers.get_mut(worker_id) {
            w.health = WorkerHealth::Disconnected;
            w.status = WorkerStatus::Offline;
        }
        self.runtime_registry.unregister(worker_id);
    }

    pub fn health(&self, worker_id: &str) -> WorkerResult<WorkerHealth> {
        self.workers
            .get(worker_id)
            .map(|w| w.health.clone())
            .ok_or_else(|| WorkerError::NotFound(worker_id.to_string()))
    }

    pub fn list(&self) -> Vec<Worker> {
        self.workers.iter().map(|e| e.value().clone()).collect()
    }

    pub fn select_worker(
        &self,
        required_os: Option<&str>,
        min_ram_mb: u32,
        need_gpu: bool,
        capability: Option<&str>,
    ) -> WorkerResult<Worker> {
        let mut ranked: Vec<Worker> = self
            .workers
            .iter()
            .map(|e| e.value().clone())
            .filter(|w| w.health == WorkerHealth::Healthy && w.status != WorkerStatus::Offline)
            .filter(|w| {
                required_os
                    .map(|os| w.capabilities.os.iter().any(|o| o == os))
                    .unwrap_or(true)
            })
            .filter(|w| w.resources.ram_mb >= min_ram_mb)
            .filter(|w| !need_gpu || w.capabilities.gpu)
            .collect();
        if let Some(cap) = capability {
            ranked.retain(|w| {
                w.capabilities.applications.contains(&cap.to_string())
                    || w.capabilities.os.iter().any(|o| o == cap)
            });
        }
        ranked.sort_by_key(|w| (w.resources.latency_ms, std::cmp::Reverse(w.resources.cpu)));
        ranked
            .into_iter()
            .next()
            .ok_or_else(|| WorkerError::Unavailable("no healthy worker".into()))
    }

    pub fn assign_task(
        &self,
        task_id: &str,
        worker_id: &str,
        principal: &str,
        token: &str,
        destructive: bool,
    ) -> WorkerResult<String> {
        self.authenticate(worker_id, token)?;
        self.authorize(worker_id, principal)?;
        let worker = self
            .workers
            .get(worker_id)
            .ok_or_else(|| WorkerError::Unavailable(worker_id.to_string()))?;
        if worker.health == WorkerHealth::Disconnected {
            return Err(WorkerError::Network("disconnected".into()));
        }
        if destructive {
            let mut once = self.destructive_once.lock().unwrap();
            if !once.insert(task_id.to_string()) {
                return Err(WorkerError::DuplicateDestructive(task_id.to_string()));
            }
            if !worker.policy.allow_destructive {
                return Err(WorkerError::AuthorizationFailure(
                    "destructive not allowed".into(),
                ));
            }
        }
        self.assigned_tasks
            .insert(task_id.to_string(), worker_id.to_string());
        Ok(worker_id.to_string())
    }

    pub fn cancel_task(&self, task_id: &str) -> WorkerResult<()> {
        self.assigned_tasks
            .remove(task_id)
            .ok_or_else(|| WorkerError::NotFound(task_id.to_string()))?;
        Ok(())
    }

    pub fn assigned_worker(&self, task_id: &str) -> Option<String> {
        self.assigned_tasks.get(task_id).map(|e| e.clone())
    }

    pub fn checkpoint_task(&self, task_id: &str) -> CheckpointState {
        let assigned = self.assigned_worker(task_id);
        CheckpointEngine::create_checkpoint(CheckpointRequest {
            task_id: task_id.to_string(),
            task_state: "running".into(),
            graph_id: "graph-remote".into(),
            completed_nodes: vec![],
            pending_nodes: vec!["node-1".into()],
            current_node: Some("node-1".into()),
            assigned_runtime: assigned,
            node_outputs: Default::default(),
        })
    }

    pub fn migrate(
        &self,
        task_id: &str,
        checkpoint: &CheckpointState,
        transferable: bool,
        principal: &str,
    ) -> WorkerResult<String> {
        if !transferable {
            return Err(WorkerError::NonTransferable(task_id.to_string()));
        }
        if let Some(old) = &checkpoint.assigned_runtime {
            self.disconnect(old);
        }
        let next = self.select_worker(None, 0, false, None)?;
        let token = next.identity.token.clone();
        let id = next.identity.worker_id.clone();
        self.assign_task(task_id, &id, principal, &token, false)?;
        Ok(id)
    }

    pub fn runtime_ids(&self) -> Vec<String> {
        self.runtime_registry.list_runtime_ids()
    }
}
