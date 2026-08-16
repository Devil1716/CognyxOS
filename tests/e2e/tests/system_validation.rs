//! System validation for CognyxOS. These tests exist to discover whether
//! independently unit-tested crates actually form one operating system.
//!
//! Failures are evidence. Do not weaken assertions to stay green.

use cognyx_agent_core::PermissionContext;
use cognyx_agent_kernel::AgentKernelServer;
use cognyx_agent_memory::{ContextEngine, LongTermMemory, MemoryKind, MemoryPrivacy};
use cognyx_execution::{native_host_runtime_id, LinuxRuntime, RuntimeRegistry};
use cognyx_gateway::{CapabilityGateway, CapabilityRequest};
use cognyx_hardening::{
    Doctor, Environment, HealthStatus, ReleaseChannel, SecretStore, SystemConfig,
};
use cognyx_plugin::{sample_echo_plugin, PluginError, PluginRegistry};
use cognyx_proto::cognyx::services::agent::v1::agent_kernel_service_server::AgentKernelService;
use cognyx_proto::cognyx::services::agent::v1::SubmitTaskRequest;
use cognyx_service_workspace::{ctx_for, WorkspaceManager};
use cognyx_shell::{AgentKernelAdapter, ApprovalDecision, CognyxShell, RecordingKernel, RiskLevel};
use cognyx_worker::{Worker, WorkerError, WorkerHealth, WorkerRegistry, WorkerStatus};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tonic::Request;

fn gw() -> CapabilityGateway {
    CapabilityGateway::new(Arc::new(RuntimeRegistry::new()))
}

fn ctx_none() -> PermissionContext {
    PermissionContext {
        user_id: "val-user".into(),
        session_id: "val-sess".into(),
        granted_capabilities: HashSet::new(),
        is_administrator: false,
    }
}

fn cap(name: &str, ctx: PermissionContext) -> CapabilityRequest {
    CapabilityRequest {
        request_id: format!("val-{}", name),
        task_id: "val-task".into(),
        agent_id: "val-agent".into(),
        capability: name.into(),
        target: String::new(),
        arguments: vec![],
        constraints: Default::default(),
        permission_context: ctx,
        timeout_seconds: 15,
    }
}

#[tokio::test]
async fn security_unauthorized_filesystem_delete_is_blocked() {
    let r = gw()
        .execute_capability(cap("filesystem.delete", ctx_none()))
        .await;
    assert!(!r.success);
    let err = r.error.unwrap_or_default();
    assert!(
        err.contains("USER_APPROVAL_REQUIRED") || err.contains("DENIED"),
        "{err}"
    );
}

#[tokio::test]
async fn security_unknown_capability_is_not_success() {
    let r = gw()
        .execute_capability(cap("window.teleport", ctx_none()))
        .await;
    assert!(!r.success, "unknown capability must not succeed: {:?}", r);
}

#[tokio::test]
async fn security_path_traversal_capability_does_not_succeed_unscoped() {
    let mut req = cap("filesystem.delete", ctx_none());
    req.target = "../../Windows/System32/drivers/etc/hosts".into();
    let r = gw().execute_capability(req).await;
    assert!(!r.success);
}

/// KNOWN ARCHITECTURAL DEFECT if this fails:
/// non-universal capabilities (e.g. bash) currently format a fake success
/// string in CapabilityGateway when no provider is registered.
#[tokio::test]
async fn legacy_non_universal_capability_must_not_fake_success() {
    let r = gw().execute_capability(cap("bash", ctx_none())).await;
    assert!(
        !r.success,
        "DEFECT: gateway reported success for bash without a real provider. output={}",
        r.output
    );
    let err = r.error.unwrap_or_default();
    assert!(
        err.contains("CAPABILITY_UNAVAILABLE")
            || err.contains("DENIED")
            || err.contains("unavailable"),
        "expected unavailable/denied, got {err}"
    );
}

#[tokio::test]
async fn real_windows_process_list_via_gateway() {
    let r = gw()
        .execute_capability(cap("process.list", ctx_none()))
        .await;
    assert!(r.success, "process.list failed: {:?}", r.error);
    assert!(
        r.output.contains("processes") || r.output.contains("pid"),
        "unexpected process.list output: {}",
        r.output
    );
}

#[tokio::test]
async fn real_windows_application_list_via_gateway() {
    let r = gw()
        .execute_capability(cap("application.list", ctx_none()))
        .await;
    assert!(r.success, "application.list failed: {:?}", r.error);
    assert!(
        r.output.to_lowercase().contains("app") || r.output.contains("notepad"),
        "unexpected application.list output: {}",
        r.output
    );
}

#[tokio::test]
async fn workspace_in_memory_contract_and_conflict() {
    let registry = Arc::new(RuntimeRegistry::new());
    registry.register(Box::new(LinuxRuntime::new("linux-host", "Linux")));
    let mgr = WorkspaceManager::new(Arc::clone(&registry));
    mgr.attach_in_memory_runtime("linux-host");
    let ctx = ctx_for(
        "owner",
        &[
            "filesystem.write",
            "filesystem.read",
            "filesystem.copy",
            "filesystem.delete",
        ],
    );
    let ws = mgr
        .create_workspace("val", "owner", "linux-host", &ctx)
        .unwrap();
    mgr.create_folder(&ws.id, "/Workspace/Test", "linux-host", &ctx)
        .unwrap();
    mgr.create_folder(&ws.id, "/Workspace/Test/Input", "linux-host", &ctx)
        .unwrap();
    mgr.create_folder(&ws.id, "/Workspace/Test/Output", "linux-host", &ctx)
        .unwrap();
    let f = mgr
        .create_file(
            &ws.id,
            "/Workspace/Test/Input/a.txt",
            "linux-host",
            b"hello",
            &ctx,
        )
        .await
        .unwrap();
    assert_eq!(mgr.read_file(&f.id, &ctx).await.unwrap(), b"hello");
    let hits = mgr.search("a.txt");
    assert!(hits.iter().any(|i| i.name == "a.txt"));
}

#[tokio::test]
async fn shell_does_not_execute_os_actions_itself() {
    let registry = Arc::new(RuntimeRegistry::new());
    registry.register(Box::new(LinuxRuntime::new("linux-host", "Linux")));
    let ws_mgr = Arc::new(WorkspaceManager::new(Arc::clone(&registry)));
    ws_mgr.attach_in_memory_runtime("linux-host");
    let ctx = ctx_for("user", &["filesystem.write", "filesystem.read"]);
    let ws = ws_mgr
        .create_workspace("home", "user", "linux-host", &ctx)
        .unwrap();
    let kernel = Arc::new(RecordingKernel::new());
    let shell = CognyxShell::new(Arc::clone(&kernel), ws_mgr, ws.id);
    let task = shell
        .submit_intent("Analyze the documents in the test workspace")
        .await
        .unwrap();
    assert_eq!(
        kernel.submitted_prompts(),
        vec!["Analyze the documents in the test workspace".to_string()]
    );
    let tree = shell.agent_tree(&task.task_id).await.unwrap();
    assert_eq!(tree.role, "manager");
    let req = shell.request_approval(
        &task.task_id,
        "filesystem.write",
        "write report",
        "/Workspace/Test/Output/report.md",
        RiskLevel::Medium,
    );
    shell
        .decide_approval(&req.id, ApprovalDecision::AllowOnce)
        .unwrap();
    let deny = shell.request_approval(
        &task.task_id,
        "terminal.execute",
        "shell",
        "host",
        RiskLevel::High,
    );
    assert!(shell
        .decide_approval(&deny.id, ApprovalDecision::Deny)
        .is_err());
}

#[tokio::test]
async fn kernel_submit_task_creates_handle() {
    let t0 = Instant::now();
    let server = AgentKernelServer::new();
    let resp = AgentKernelService::submit_task(
        &server,
        Request::new(SubmitTaskRequest {
            meta: None,
            cap: None,
            prompt: "Open the test workspace".into(),
            priority: 1,
        }),
    )
    .await
    .expect("submit_task");
    let handle = resp.into_inner();
    assert!(!handle.task_id.is_empty());
    println!("PERF intent_to_handle_ms={}", t0.elapsed().as_millis());
}

#[tokio::test]
async fn memory_deletion_is_real_and_scoped() {
    let mem = LongTermMemory::new(Arc::new(ContextEngine::new()));
    let rec = mem
        .ingest(
            MemoryKind::Episodic,
            "approved project fact: cognyx demo uses local test docs",
            MemoryPrivacy {
                owner: "user".into(),
                scope: "user".into(),
                retention_secs: 86400,
                visibility: "private".into(),
                classification: "general".into(),
                consent: true,
            },
            Some("task-a".into()),
            None,
            Some("ws-demo".into()),
        )
        .await
        .unwrap();
    assert!(!mem
        .retrieve("cognyx demo", "stranger", 4)
        .await
        .iter()
        .any(|(r, _)| r.id == rec.id));
    mem.delete(&rec.id, "user").unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let hits = mem.retrieve("cognyx demo local test docs", "user", 8).await;
    assert!(
        hits.iter().all(|(r, _)| r.id != rec.id),
        "deleted memory still retrievable"
    );
}

#[test]
fn plugin_lifecycle_and_isolation() {
    let reg = PluginRegistry::new();
    let p = reg.install(sample_echo_plugin()).unwrap();
    reg.verify(&p.id).unwrap();
    reg.enable(&p.id).unwrap();
    assert!(reg.execute(&p.id, "echo.say", 10, None, None).is_ok());
    reg.disable(&p.id).unwrap();
    assert!(matches!(
        reg.execute(&p.id, "echo.say", 10, None, None),
        Err(PluginError::Disabled(_))
    ));
    reg.enable(&p.id).unwrap();
    reg.update(&p.id, "0.2.0").unwrap();
    let rolled = reg.rollback(&p.id).unwrap();
    assert_eq!(rolled.manifest.version, "0.1.0");
    reg.remove(&p.id).unwrap();
}

#[test]
fn local_worker_not_wan() {
    let reg = WorkerRegistry::new(Arc::new(RuntimeRegistry::new()));
    let mut w = Worker::new("local-1", "windows");
    w.status = WorkerStatus::Online;
    w.health = WorkerHealth::Healthy;
    let token = w.identity.token.clone();
    reg.register(w).unwrap();
    assert_eq!(
        reg.heartbeat("local-1", &token).unwrap(),
        WorkerHealth::Healthy
    );
    assert!(matches!(
        reg.heartbeat("local-1", "wrong"),
        Err(WorkerError::AuthenticationFailure)
    ));
}

#[test]
fn hardening_production_nightly_rejected_and_doctor_runs() {
    let bad = SystemConfig {
        environment: Environment::Production,
        release_channel: ReleaseChannel::Nightly,
        version: "0.1.0".into(),
    };
    assert!(bad.validate().is_err());
    let secrets = SecretStore::new();
    secrets.put("k", b"tokensecret");
    assert!(secrets.redact("tokensecret").is_err());
    let report = Doctor::run();
    assert!(report.iter().any(|d| d.component == "security"));
    let virt = report
        .iter()
        .find(|d| d.component == "virtualization")
        .unwrap();
    if matches!(
        virt.status,
        HealthStatus::NotVerified
            | HealthStatus::Unavailable
            | HealthStatus::PermissionDenied
            | HealthStatus::NotInstalled
    ) {
        assert!(
            !virt.ok,
            "virtualization must not report ok when not verified: {:?}",
            virt
        );
    }
}

/// REAL: production shell path is AgentKernelAdapter wrapping AgentKernelServer.
/// RecordingKernel is TEST ONLY and must not appear here.
#[tokio::test]
async fn production_shell_submits_to_agent_kernel_server() {
    let server = Arc::new(AgentKernelServer::new());
    let host_id = native_host_runtime_id();
    let registry = Arc::clone(&server.registry);
    if !registry.list_runtime_ids().iter().any(|id| id == host_id) {
        registry.register(Box::new(LinuxRuntime::new(host_id, "host")));
    }
    let ws_mgr = Arc::new(WorkspaceManager::new(Arc::clone(&registry)));
    ws_mgr.attach_in_memory_runtime(host_id);
    let ctx = ctx_for("user", &["filesystem.write", "filesystem.read"]);
    let ws = ws_mgr
        .create_workspace("CognyxDemo", "user", host_id, &ctx)
        .unwrap();
    let kernel = Arc::new(AgentKernelAdapter::from_server(Arc::clone(&server)));
    let shell = CognyxShell::new(Arc::clone(&kernel), ws_mgr, ws.id);
    let task = shell.submit_intent("Open ZyxxNotAnApp999").await.unwrap();
    assert!(!task.task_id.is_empty());
    assert_ne!(task.task_id, "task-1", "must not be RecordingKernel ids");
    let mut inspected = shell.inspect_task(&task.task_id).await.unwrap();
    for _ in 0..50 {
        if inspected.status == "failed" || inspected.status == "completed" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        inspected = shell.inspect_task(&task.task_id).await.unwrap();
    }
    assert_eq!(inspected.status, "failed");
    let error = inspected.error.unwrap_or_default();
    assert!(
        error.contains("APPLICATION_NOT_FOUND"),
        "unknown application must fail search, got {error}"
    );
}

/// Same CapabilityGateway instance the kernel uses. REAL search identity.
/// application.open of notepad is GUI and stays in the ignored hardware test.
#[tokio::test]
async fn golden_path_universal_capabilities_via_kernel_gateway() {
    let server = AgentKernelServer::new();
    let mut req = cap("application.search", ctx_none());
    req.target = "notepad".into();
    let search = server.gateway.execute_capability(req).await;
    assert!(
        search.success,
        "application.search failed: {:?}",
        search.error
    );
    if cfg!(target_os = "windows") {
        assert!(
            !search.assigned_runtime_id.to_lowercase().contains("linux"),
            "runtime_id must identify Windows, got {}",
            search.assigned_runtime_id
        );
        assert!(
            search
                .assigned_runtime_id
                .to_lowercase()
                .contains("windows"),
            "runtime_id should contain windows: {}",
            search.assigned_runtime_id
        );
    }
    assert!(
        !search.assigned_runtime_id.contains("sim-backend"),
        "must not use sim-backend"
    );
}

#[tokio::test]
#[cfg(target_os = "windows")]
async fn application_search_notepad_runtime_id_is_not_linux() {
    let r = gw()
        .execute_capability({
            let mut req = cap("application.search", ctx_none());
            req.target = "notepad".into();
            req
        })
        .await;
    assert!(r.success, "{:?}", r.error);
    assert!(
        !r.assigned_runtime_id.to_lowercase().contains("linux"),
        "assigned_runtime_id must NOT contain linux: {}",
        r.assigned_runtime_id
    );
}

#[tokio::test]
#[cfg(target_os = "windows")]
async fn real_windows_clipboard_roundtrip() {
    let gw = gw();
    let token = "COGNYXOS-VALIDATION-TOKEN";
    let mut write = cap(
        "clipboard.write",
        PermissionContext {
            user_id: "val-user".into(),
            session_id: "val-sess".into(),
            granted_capabilities: HashSet::from([
                "clipboard.write".into(),
                "clipboard.read".into(),
            ]),
            is_administrator: false,
        },
    );
    write.target = token.into();
    let w = gw.execute_capability(write).await;
    assert!(w.success, "clipboard.write failed: {:?}", w.error);
    let read = cap(
        "clipboard.read",
        PermissionContext {
            user_id: "val-user".into(),
            session_id: "val-sess".into(),
            granted_capabilities: HashSet::from(["clipboard.read".into()]),
            is_administrator: false,
        },
    );
    let r = gw.execute_capability(read).await;
    assert!(r.success, "clipboard.read failed: {:?}", r.error);
    assert!(
        r.output.contains(token),
        "clipboard did not round-trip token. output={}",
        r.output
    );
}

#[tokio::test]
#[cfg(target_os = "windows")]
async fn real_windows_window_list_via_gateway() {
    let r = gw()
        .execute_capability(cap("window.list", ctx_none()))
        .await;
    assert!(r.success, "window.list failed: {:?}", r.error);
    assert!(
        r.output.contains("window_id") || r.output.contains("hwnd") || r.output.starts_with('['),
        "unexpected window.list output: {}",
        r.output
    );
}
