use cognyx_execution::RuntimeRegistry;
use cognyx_service_workspace::{ctx_for, WorkspaceManager};
use cognyx_shell::{
    ApprovalDecision, CognyxShell, ComputerUseFrame, KernelClient, NotificationKind,
    RecordingKernel, RiskLevel, ShellError,
};
use std::sync::Arc;

async fn harness() -> (CognyxShell<RecordingKernel>, Arc<RecordingKernel>, String) {
    let registry = Arc::new(RuntimeRegistry::new());
    registry.register(Box::new(cognyx_execution::LinuxRuntime::new(
        "linux-host",
        "Linux host",
    )));
    let ws_mgr = Arc::new(WorkspaceManager::new(Arc::clone(&registry)));
    ws_mgr.attach_in_memory_runtime("linux-host");
    let ctx = ctx_for(
        "user",
        &[
            "filesystem.write",
            "filesystem.read",
            "filesystem.copy",
            "filesystem.delete",
        ],
    );
    let ws = ws_mgr
        .create_workspace("home", "user", "linux-host", &ctx)
        .unwrap();
    ws_mgr
        .create_file(
            &ws.id,
            "/Workspace/Documents/deck.md",
            "linux-host",
            b"# slides",
            &ctx,
        )
        .await
        .unwrap();
    let kernel = Arc::new(RecordingKernel::new());
    let shell = CognyxShell::new(Arc::clone(&kernel), ws_mgr, ws.id.clone());
    (shell, kernel, ws.id)
}

#[tokio::test]
async fn launch_shell_has_desktop_and_dock() {
    let (shell, _, _) = harness().await;
    assert!(!shell.dock().is_empty());
    assert!(!shell.desktop().workspace_id.is_empty());
}

#[tokio::test]
async fn command_bar_submits_to_kernel_not_local_engine() {
    let (shell, kernel, _) = harness().await;
    let task = shell.submit_intent("Find my presentation.").await.unwrap();
    assert_eq!(
        kernel.submitted_prompts(),
        vec!["Find my presentation.".to_string()]
    );
    assert_eq!(task.status, "running");
    let progress = shell.inspect_task(&task.task_id).await.unwrap();
    assert_eq!(progress.task_id, task.task_id);
}

#[tokio::test]
async fn agent_tree_is_visible() {
    let (shell, _, _) = harness().await;
    let task = shell.submit_intent("Open Photoshop.").await.unwrap();
    let tree = shell.agent_tree(&task.task_id).await.unwrap();
    assert_eq!(tree.role, "manager");
    assert!(!tree.children.is_empty());
    let child = shell
        .inspect_agent(&tree.children[0].agent_id)
        .await
        .unwrap();
    assert_eq!(child.role, "file");
}

#[tokio::test]
async fn approve_and_deny_capability() {
    let (shell, _, _) = harness().await;
    let req = shell.request_approval(
        "task-1",
        "filesystem.write",
        "Agent wants to write report.md",
        "/Workspace/Documents/report.md",
        RiskLevel::Medium,
    );
    assert_eq!(shell.pending_approvals().len(), 1);
    let allowed = shell
        .decide_approval(&req.id, ApprovalDecision::AllowOnce)
        .unwrap();
    assert_eq!(allowed.decided, Some(ApprovalDecision::AllowOnce));

    let deny = shell.request_approval(
        "task-1",
        "terminal.execute",
        "Agent wants a shell",
        "host",
        RiskLevel::High,
    );
    let err = shell
        .decide_approval(&deny.id, ApprovalDecision::Deny)
        .unwrap_err();
    assert!(matches!(err, ShellError::Denied(_)));
}

#[tokio::test]
async fn observe_windows_app_and_browser_without_new_computer_use_engine() {
    let (shell, _, _) = harness().await;
    shell.observe_frame(ComputerUseFrame {
        runtime_id: "windows-vm".into(),
        application: "Photoshop".into(),
        kind: "application".into(),
        note: "Phase 5 stream".into(),
    });
    shell.observe_frame(ComputerUseFrame {
        runtime_id: "linux-host".into(),
        application: "Chrome".into(),
        kind: "browser".into(),
        note: "Phase 5 stream".into(),
    });
    let frames = shell.computer_use_frames();
    assert_eq!(frames.len(), 2);
    shell.open_window("photoshop", "windows-vm", "Photoshop");
    shell.open_window("chrome", "linux-host", "Chrome");
    assert_eq!(shell.windows().len(), 2);
}

#[tokio::test]
async fn switch_workspace_and_search_files() {
    let (shell, _, ws_id) = harness().await;
    let hits = shell.search_workspace("deck");
    assert!(hits.iter().any(|i| i.name == "deck.md"));
    shell.switch_workspace(&ws_id).unwrap();
    assert_eq!(shell.desktop().workspace_id, ws_id);
}

#[tokio::test]
async fn recover_from_task_failure() {
    let (shell, kernel, _) = harness().await;
    kernel.fail_next_submit();
    let failed = shell
        .submit_intent("Continue yesterday's task.")
        .await
        .unwrap();
    assert_eq!(failed.status, "failed");
    let recovered = shell.recover_task(&failed.task_id).await.unwrap();
    assert_eq!(recovered.status, "running");
}

#[tokio::test]
async fn notifications_are_deduped() {
    let (shell, _, _) = harness().await;
    assert!(shell
        .notify(
            NotificationKind::RuntimeUnavailable,
            "Runtime down",
            "windows-vm",
            "windows-vm",
        )
        .is_some());
    assert!(shell
        .notify(
            NotificationKind::RuntimeUnavailable,
            "Runtime down",
            "windows-vm",
            "windows-vm",
        )
        .is_none());
    assert_eq!(shell.notifications().len(), 1);
}

#[tokio::test]
async fn kernel_client_is_the_only_submit_path() {
    let (_shell, kernel, _) = harness().await;
    let _ = kernel
        .submit_intent("Open Chrome and research this.")
        .await
        .unwrap();
    assert_eq!(kernel.submitted_prompts().len(), 1);
}
