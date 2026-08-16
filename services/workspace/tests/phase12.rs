use async_trait::async_trait;
use cognyx_execution::{ExecutionRuntime, RuntimeKind, RuntimeRegistry, RuntimeStatus};
use cognyx_service_workspace::{ctx_for, WorkspaceError, WorkspaceManager};
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
        1
    }
    fn available_tools(&self) -> Vec<String> {
        vec![]
    }
    async fn start(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn pause(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn resume(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn execute_command(&self, _command: &str, _args: &[&str]) -> Result<String, String> {
        Ok("ok".into())
    }
}

#[tokio::test]
async fn missing_linux_macos_runtime_returns_runtime_unavailable() {
    let registry = Arc::new(RuntimeRegistry::new());
    registry.register(Box::new(TestRuntime::new(
        "windows-host",
        RuntimeKind::WindowsVm,
    )));
    let mgr = WorkspaceManager::new(Arc::clone(&registry));
    mgr.attach_in_memory_runtime("windows-host");
    let ctx = ctx_for("owner", &["filesystem.write", "filesystem.read"]);
    let err = mgr
        .create_workspace("x", "owner", "linux-host", &ctx)
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("RUNTIME_UNAVAILABLE"),
        "must not silently use InMemoryFilesystem: {msg}"
    );
    assert!(matches!(err, WorkspaceError::RuntimeUnavailable(_)));

    let err2 = mgr
        .create_workspace("y", "owner", "macos-host", &ctx)
        .unwrap_err();
    assert!(err2.to_string().contains("RUNTIME_UNAVAILABLE"));
}

#[cfg(windows)]
#[tokio::test]
async fn host_filesystem_create_read_write_under_dedicated_root() {
    let registry = Arc::new(RuntimeRegistry::new());
    registry.register(Box::new(TestRuntime::new(
        "windows-host",
        RuntimeKind::WindowsVm,
    )));
    let mgr = WorkspaceManager::new(Arc::clone(&registry));
    let fs = mgr
        .attach_dedicated_host_filesystem("windows-host")
        .expect("attach host fs");
    let root_s = fs.root().to_string_lossy().replace(r"\\?\", "");
    assert!(
        root_s.to_lowercase().contains(r"c:\cognyxostestworkspace"),
        "host root must be dedicated dir, got {root_s}"
    );
    let ctx = ctx_for("owner", &["filesystem.write", "filesystem.read"]);
    let ws = mgr
        .create_workspace("phase12-host", "owner", "windows-host", &ctx)
        .unwrap();
    mgr.create_folder(&ws.id, "/Workspace/Documents", "windows-host", &ctx)
        .ok();
    let file = mgr
        .create_file(
            &ws.id,
            "/Workspace/Documents/phase12-host.txt",
            "windows-host",
            b"hello-host",
            &ctx,
        )
        .await
        .unwrap();
    assert_eq!(mgr.read_file(&file.id, &ctx).await.unwrap(), b"hello-host");
    mgr.write_file(&file.id, b"updated-host", &ctx)
        .await
        .unwrap();
    assert_eq!(
        mgr.read_file(&file.id, &ctx).await.unwrap(),
        b"updated-host"
    );
}

#[tokio::test]
async fn in_memory_filesystem_remains_for_unit_tests() {
    let registry = Arc::new(RuntimeRegistry::new());
    registry.register(Box::new(TestRuntime::new(
        "windows-host",
        RuntimeKind::WindowsVm,
    )));
    let mgr = WorkspaceManager::new(registry);
    mgr.attach_in_memory_runtime("windows-host");
    let ctx = ctx_for("owner", &["filesystem.write", "filesystem.read"]);
    let ws = mgr
        .create_workspace("mem", "owner", "windows-host", &ctx)
        .unwrap();
    let f = mgr
        .create_file(
            &ws.id,
            "/Workspace/Documents/m.txt",
            "windows-host",
            b"x",
            &ctx,
        )
        .await
        .unwrap();
    assert_eq!(mgr.read_file(&f.id, &ctx).await.unwrap(), b"x");
}
