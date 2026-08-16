use cognyx_execution::RuntimeRegistry;
use cognyx_worker::{
    ArtifactBlob, Worker, WorkerError, WorkerHealth, WorkerRegistry, WorkerStatus,
};
use std::sync::Arc;

fn registry() -> WorkerRegistry {
    WorkerRegistry::new(Arc::new(RuntimeRegistry::new()))
}

fn online_worker(id: &str, os: &str) -> Worker {
    let mut w = Worker::new(id, os);
    w.status = WorkerStatus::Online;
    w.health = WorkerHealth::Healthy;
    w
}

#[test]
fn worker_registration_discovery_heartbeat_health() {
    let reg = registry();
    let w = online_worker("w-linux", "linux");
    let token = w.identity.token.clone();
    let id = reg.register(w).unwrap();
    assert!(reg.runtime_ids().contains(&id));
    assert_eq!(reg.list().len(), 1);
    assert_eq!(reg.heartbeat(&id, &token).unwrap(), WorkerHealth::Healthy);
    assert_eq!(reg.health(&id).unwrap(), WorkerHealth::Healthy);
}

#[tokio::test]
async fn remote_capability_and_task() {
    let reg = registry();
    let mut w = online_worker("w-win", "windows");
    w.capabilities.applications.push("photoshop".into());
    let token = w.identity.token.clone();
    let id = reg.register(w).unwrap();
    let selected = reg
        .select_worker(Some("windows"), 1024, false, Some("photoshop"))
        .unwrap();
    assert_eq!(selected.identity.worker_id, id);
    let assigned = reg
        .assign_task("task-1", &id, "kernel", &token, false)
        .unwrap();
    assert_eq!(assigned, id);
    assert_eq!(reg.assigned_worker("task-1").as_deref(), Some(id.as_str()));
}

#[test]
fn artifact_transfer_checksum() {
    let reg = registry();
    let blob = ArtifactBlob::new("art-1", b"payload".to_vec());
    reg.transfers.put(blob.clone()).unwrap();
    let got = reg.transfers.get("art-1").unwrap();
    assert_eq!(got.checksum, blob.checksum);
    assert!(got.encrypted);
}

#[test]
fn worker_failure_migrates_via_checkpoint() {
    let reg = registry();
    let a = online_worker("w-a", "linux");
    let b = online_worker("w-b", "linux");
    let token_a = a.identity.token.clone();
    reg.register(a).unwrap();
    reg.register(b).unwrap();
    reg.assign_task("task-mig", "w-a", "kernel", &token_a, false)
        .unwrap();
    let chk = reg.checkpoint_task("task-mig");
    let next = reg.migrate("task-mig", &chk, true, "kernel").unwrap();
    assert_eq!(next, "w-b");
    assert_eq!(reg.health("w-a").unwrap(), WorkerHealth::Disconnected);
}

#[test]
fn non_transferable_state_is_not_migrated() {
    let reg = registry();
    let chk = cognyx_task_manager::CheckpointEngine::create_checkpoint(
        cognyx_task_manager::CheckpointRequest {
            task_id: "t".into(),
            task_state: "running".into(),
            graph_id: "g".into(),
            completed_nodes: vec![],
            pending_nodes: vec![],
            current_node: None,
            assigned_runtime: Some("w-a".into()),
            node_outputs: Default::default(),
        },
    );
    let err = reg.migrate("t", &chk, false, "kernel").unwrap_err();
    assert!(matches!(err, WorkerError::NonTransferable(_)));
}

#[test]
fn network_and_auth_failures() {
    let reg = registry();
    let w = online_worker("w-net", "linux");
    let token = w.identity.token.clone();
    reg.register(w).unwrap();
    assert!(matches!(
        reg.heartbeat("w-net", "bad-token"),
        Err(WorkerError::AuthenticationFailure)
    ));
    assert!(matches!(
        reg.authorize("w-net", "stranger"),
        Err(WorkerError::AuthorizationFailure(_))
    ));
    reg.disconnect("w-net");
    let err = reg
        .assign_task("task-x", "w-net", "kernel", &token, false)
        .unwrap_err();
    assert!(matches!(
        err,
        WorkerError::Unavailable(_) | WorkerError::AuthenticationFailure | WorkerError::Network(_)
    ));
}

#[test]
fn duplicate_destructive_execution_blocked() {
    let reg = registry();
    let mut w = online_worker("w-d", "linux");
    w.policy.allow_destructive = true;
    let token = w.identity.token.clone();
    reg.register(w).unwrap();
    reg.assign_task("task-rm", "w-d", "kernel", &token, true)
        .unwrap();
    let err = reg
        .assign_task("task-rm", "w-d", "kernel", &token, true)
        .unwrap_err();
    assert!(matches!(err, WorkerError::DuplicateDestructive(_)));
}

#[test]
fn resource_aware_scheduling_prefers_low_latency() {
    let reg = registry();
    let mut slow = online_worker("slow", "linux");
    slow.resources.latency_ms = 80;
    slow.resources.ram_mb = 16384;
    let mut fast = online_worker("fast", "linux");
    fast.resources.latency_ms = 3;
    fast.resources.ram_mb = 16384;
    let mut gpu = online_worker("gpu", "linux");
    gpu.capabilities.gpu = true;
    gpu.resources.gpu_count = 1;
    gpu.resources.latency_ms = 40;
    reg.register(slow).unwrap();
    reg.register(fast).unwrap();
    reg.register(gpu).unwrap();
    let picked = reg.select_worker(Some("linux"), 8192, false, None).unwrap();
    assert_eq!(picked.identity.worker_id, "fast");
    let gpu_pick = reg.select_worker(None, 0, true, None).unwrap();
    assert_eq!(gpu_pick.identity.worker_id, "gpu");
}

#[test]
fn cancel_and_restore_checkpoint() {
    let reg = registry();
    let w = online_worker("w-c", "linux");
    let token = w.identity.token.clone();
    reg.register(w).unwrap();
    reg.assign_task("task-c", "w-c", "kernel", &token, false)
        .unwrap();
    let chk = reg.checkpoint_task("task-c");
    assert_eq!(chk.task_id, "task-c");
    reg.cancel_task("task-c").unwrap();
    assert!(reg.assigned_worker("task-c").is_none());
}
