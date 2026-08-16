use async_trait::async_trait;
use cognyx_execution::{ExecutionRuntime, RuntimeKind, RuntimeRegistry, RuntimeStatus};
use cognyx_service_workspace::{
    ctx_for, ArtifactIngestRequest, WorkspaceError, WorkspaceItemKind, WorkspaceManager,
    LOGICAL_ARTIFACTS, LOGICAL_DOCUMENTS, LOGICAL_PROJECTS,
};
use std::sync::Arc;

struct TestRuntime {
    id: String,
    kind: RuntimeKind,
    status: RuntimeStatus,
}

impl TestRuntime {
    fn new(id: &str, kind: RuntimeKind) -> Self {
        Self {
            id: id.to_string(),
            kind,
            status: RuntimeStatus::Running,
        }
    }
}

#[async_trait]
impl ExecutionRuntime for TestRuntime {
    fn runtime_id(&self) -> &str {
        &self.id
    }
    fn runtime_type(&self) -> RuntimeKind {
        self.kind.clone()
    }
    fn status(&self) -> RuntimeStatus {
        self.status.clone()
    }
    fn capabilities(&self) -> Vec<String> {
        vec!["filesystem.read".into(), "filesystem.write".into()]
    }
    fn location(&self) -> String {
        "test".into()
    }
    fn security_level(&self) -> u32 {
        2
    }
    fn available_tools(&self) -> Vec<String> {
        vec![]
    }
    async fn start(&mut self) -> Result<(), String> {
        self.status = RuntimeStatus::Running;
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), String> {
        self.status = RuntimeStatus::Stopped;
        Ok(())
    }
    async fn pause(&mut self) -> Result<(), String> {
        self.status = RuntimeStatus::Paused;
        Ok(())
    }
    async fn resume(&mut self) -> Result<(), String> {
        self.status = RuntimeStatus::Running;
        Ok(())
    }
    async fn execute_command(&self, _command: &str, _args: &[&str]) -> Result<String, String> {
        Ok("ok".into())
    }
}

fn harness() -> (WorkspaceManager, String) {
    let registry = Arc::new(RuntimeRegistry::new());
    registry.register(Box::new(TestRuntime::new(
        "linux-host",
        RuntimeKind::NativeLinux,
    )));
    registry.register(Box::new(TestRuntime::new(
        "windows-vm",
        RuntimeKind::WindowsVm,
    )));
    registry.register(Box::new(TestRuntime::new("macos-vm", RuntimeKind::MacOsVm)));
    registry.register(Box::new(TestRuntime::new(
        "container-1",
        RuntimeKind::Container,
    )));

    let manager = WorkspaceManager::new(Arc::clone(&registry));
    manager.attach_in_memory_runtime("linux-host");
    manager.attach_in_memory_runtime("windows-vm");
    manager.attach_in_memory_runtime("macos-vm");
    manager.attach_in_memory_runtime("container-1");

    let ctx = owner_ctx();
    let ws = manager
        .create_workspace("primary", "owner", "linux-host", &ctx)
        .expect("workspace");
    (manager, ws.id)
}

fn owner_ctx() -> cognyx_service_workspace::PermissionContext {
    ctx_for(
        "owner",
        &[
            "filesystem.write",
            "filesystem.read",
            "filesystem.copy",
            "filesystem.move",
            "filesystem.delete",
        ],
    )
}

#[tokio::test]
async fn create_workspace_and_logical_layout() {
    let (manager, ws_id) = harness();
    let ws = manager.get_workspace(&ws_id).unwrap();
    assert_eq!(ws.name, "primary");
    let found = manager.search("Documents");
    assert!(found.iter().any(|i| i.location == LOGICAL_DOCUMENTS));
    let state = manager.state();
    assert_eq!(state.active_workspace.as_deref(), Some(ws_id.as_str()));
}

#[tokio::test]
async fn create_folder_and_file_then_read() {
    let (manager, ws_id) = harness();
    let ctx = owner_ctx();
    let folder = manager
        .create_folder(&ws_id, "/Workspace/Projects/alpha", "linux-host", &ctx)
        .unwrap();
    assert_eq!(folder.kind, WorkspaceItemKind::Folder);

    let file = manager
        .create_file(
            &ws_id,
            "/Workspace/Documents/hello.txt",
            "linux-host",
            b"hello cognyx",
            &ctx,
        )
        .await
        .unwrap();
    let body = manager.read_file(&file.id, &ctx).await.unwrap();
    assert_eq!(body, b"hello cognyx");
    let reference = manager.reference_for(&ws_id, &file.id).unwrap();
    assert_eq!(reference.runtime_id, "linux-host");
    assert!(reference.physical_location.starts_with("linux-host:"));
}

#[tokio::test]
async fn copy_linux_to_windows_and_back() {
    let (manager, ws_id) = harness();
    let ctx = owner_ctx();
    let src = manager
        .create_file(
            &ws_id,
            "/Workspace/Documents/report.txt",
            "linux-host",
            b"cross-os",
            &ctx,
        )
        .await
        .unwrap();

    let on_windows = manager
        .copy_file(&ws_id, &src.id, "windows-vm", None, &ctx)
        .await
        .unwrap();
    assert_eq!(on_windows.runtime_id, "windows-vm");
    assert_eq!(
        manager.read_file(&on_windows.id, &ctx).await.unwrap(),
        b"cross-os"
    );

    let back = manager
        .copy_file(
            &ws_id,
            &on_windows.id,
            "linux-host",
            Some("/Workspace/Documents/report-roundtrip.txt"),
            &ctx,
        )
        .await
        .unwrap();
    assert_eq!(back.runtime_id, "linux-host");
    assert_eq!(
        manager.read_file(&back.id, &ctx).await.unwrap(),
        b"cross-os"
    );
}

#[tokio::test]
async fn copy_linux_to_macos_when_available() {
    let (manager, ws_id) = harness();
    let ctx = owner_ctx();
    let src = manager
        .create_file(
            &ws_id,
            "/Workspace/Documents/mac.txt",
            "linux-host",
            b"darwin",
            &ctx,
        )
        .await
        .unwrap();
    let dest = manager
        .copy_file(&ws_id, &src.id, "macos-vm", None, &ctx)
        .await
        .unwrap();
    assert_eq!(dest.runtime_id, "macos-vm");
}

#[tokio::test]
async fn artifact_create_and_share() {
    let (manager, ws_id) = harness();
    let ctx = owner_ctx();
    let artifact = manager
        .ingest_artifact(
            ArtifactIngestRequest {
                workspace_id: &ws_id,
                runtime_id: "linux-host",
                source_artifact_id: "art-phase6-1",
                source_task_id: "task-1",
                source_agent_id: "agent-writer",
                name: "summary.json",
                data: b"{\"ok\":true}",
            },
            &ctx,
        )
        .await
        .unwrap();
    assert_eq!(artifact.item.kind, WorkspaceItemKind::Artifact);
    assert!(artifact.item.location.starts_with(LOGICAL_ARTIFACTS));

    let shared = manager
        .share_artifact(&artifact.item.id, "teammate", &ctx)
        .unwrap();
    assert!(shared.shared_with.contains(&"teammate".to_string()));

    let teammate = ctx_for("teammate", &["filesystem.read"]);
    manager.get_artifact(&artifact.item.id, &teammate).unwrap();
}

#[tokio::test]
async fn conflict_detection_does_not_overwrite() {
    let (manager, ws_id) = harness();
    let ctx = owner_ctx();
    let left = manager
        .create_file(
            &ws_id,
            "/Workspace/Documents/clash.txt",
            "linux-host",
            b"v1",
            &ctx,
        )
        .await
        .unwrap();
    manager
        .create_file(
            &ws_id,
            "/Workspace/Documents/clash.txt",
            "windows-vm",
            b"other",
            &ctx,
        )
        .await
        .unwrap();

    let err = manager
        .copy_file(&ws_id, &left.id, "windows-vm", None, &ctx)
        .await
        .unwrap_err();
    assert!(matches!(err, WorkspaceError::Conflict(_)));
}

#[tokio::test]
async fn versioning_and_restore() {
    let (manager, ws_id) = harness();
    let ctx = owner_ctx();
    let file = manager
        .create_file(
            &ws_id,
            "/Workspace/Documents/notes.txt",
            "linux-host",
            b"one",
            &ctx,
        )
        .await
        .unwrap();
    manager.write_file(&file.id, b"two", &ctx).await.unwrap();
    let restored = manager.restore(&file.id, 1, &ctx).await.unwrap();
    assert!(restored.version > 1);
    let body = manager.read_file(&file.id, &ctx).await.unwrap();
    assert_eq!(body, b"one");
}

#[tokio::test]
async fn permission_enforcement() {
    let (manager, ws_id) = harness();
    let owner = owner_ctx();
    let file = manager
        .create_file(
            &ws_id,
            "/Workspace/Documents/secret.txt",
            "linux-host",
            b"nope",
            &owner,
        )
        .await
        .unwrap();

    let stranger = ctx_for("stranger", &["filesystem.read", "filesystem.write"]);
    let err = manager.read_file(&file.id, &stranger).await.unwrap_err();
    assert!(matches!(err, WorkspaceError::PermissionDenied(_)));
}

#[tokio::test]
async fn runtime_unavailable_is_surfaced() {
    let (manager, ws_id) = harness();
    let ctx = owner_ctx();
    let src = manager
        .create_file(
            &ws_id,
            "/Workspace/Documents/hold.txt",
            "linux-host",
            b"data",
            &ctx,
        )
        .await
        .unwrap();
    manager.set_runtime_available("windows-vm", false).unwrap();
    let err = manager
        .copy_file(&ws_id, &src.id, "windows-vm", None, &ctx)
        .await
        .unwrap_err();
    assert!(matches!(err, WorkspaceError::RuntimeUnavailable(_)));
}

#[tokio::test]
async fn workspace_recovery_from_checkpoint() {
    let (manager, ws_id) = harness();
    let ctx = owner_ctx();
    manager
        .create_file(
            &ws_id,
            "/Workspace/Projects/keep.txt",
            "linux-host",
            b"persist",
            &ctx,
        )
        .await
        .unwrap();
    let checkpoint = manager.snapshot();

    let registry = manager.registry();
    let recovered = WorkspaceManager::new(registry);
    recovered.attach_in_memory_runtime("linux-host");
    recovered.restore_checkpoint(checkpoint);
    assert!(recovered.get_workspace(&ws_id).is_ok());
    assert!(recovered
        .search("keep.txt")
        .iter()
        .any(|i| i.location == "/Workspace/Projects/keep.txt"));
}

#[tokio::test]
async fn search_covers_files_tasks_and_artifacts() {
    let (manager, ws_id) = harness();
    let ctx = owner_ctx();
    manager
        .create_file(
            &ws_id,
            "/Workspace/Documents/presentation.md",
            "linux-host",
            b"# deck",
            &ctx,
        )
        .await
        .unwrap();
    manager.record_active_task("task-research");
    manager.record_running_agent("agent-file");
    manager.record_open_application("app-writer");
    let hits = manager.search("presentation");
    assert_eq!(hits.len(), 1);
    let state = manager.state();
    assert!(state.active_tasks.contains(&"task-research".to_string()));
    assert!(state.running_agents.contains(&"agent-file".to_string()));
    assert!(state.open_applications.contains(&"app-writer".to_string()));
    let _ = LOGICAL_PROJECTS;
}
