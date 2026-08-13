use async_trait::async_trait;
use cognyx_capability::*;
use serde_json::json;
use std::sync::Arc;

fn request(id: &str, input: serde_json::Value) -> CapabilityRequest {
    CapabilityRequest {
        request_id: "req-1".into(),
        task_id: "task-1".into(),
        agent_id: "agent-1".into(),
        capability_id: id.into(),
        requested_version: Some(CapabilityVersion::v1()),
        runtime_hint: None,
        input,
        timeout_ms: Some(1_000),
        trace_id: "trace-1".into(),
        span_id: "span-1".into(),
    }
}

#[tokio::test]
async fn registry_versions_discovery_and_provider_priority_work() {
    let layer = UniversalCapabilityLayer::default();
    layer
        .register_provider(Arc::new(AdapterProvider::new(
            "linux",
            "linux-1",
            LinuxCapabilityAdapter,
        )))
        .unwrap();
    let definition = layer
        .registry()
        .lookup("screen.capture", Some(&CapabilityVersion::v1()))
        .unwrap();
    assert_eq!(definition.version, CapabilityVersion::v1());
    assert_eq!(
        layer.registry().provider_ids_for("screen.capture"),
        vec!["linux"]
    );
    let result = layer.execute(request("screen.capture", json!({}))).await;
    assert_eq!(result.status, CapabilityStatus::Completed);
    assert_eq!(result.output["simulated"], true);
}

#[tokio::test]
async fn scoped_filesystem_contract_has_consistent_semantics() {
    let root = std::env::temp_dir().join(format!("cognyx-capability-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&root).unwrap();
    let layer = UniversalCapabilityLayer::default();
    layer
        .register_provider(Arc::new(LocalFilesystemProvider::new(
            "fs",
            "test-runtime",
            &root,
        )))
        .unwrap();
    assert_eq!(
        layer
            .execute(request(
                "filesystem.write",
                json!({"path":"hello.txt", "content":"hello"})
            ))
            .await
            .status,
        CapabilityStatus::Completed
    );
    let read = layer
        .execute(request("filesystem.read", json!({"path":"hello.txt"})))
        .await;
    assert_eq!(read.output["content"], "hello");
    assert_eq!(
        layer
            .execute(request(
                "filesystem.copy",
                json!({"path":"hello.txt", "target":"copy.txt"})
            ))
            .await
            .status,
        CapabilityStatus::Completed
    );
    assert_eq!(
        layer
            .execute(request("filesystem.delete", json!({"path":"copy.txt"})))
            .await
            .status,
        CapabilityStatus::Completed
    );
    let escaped = layer
        .execute(request("filesystem.read", json!({"path":"../outside"})))
        .await;
    assert_eq!(
        escaped.error.unwrap().code,
        CapabilityErrorCode::PermissionDenied
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cross_os_adapter_contracts_expose_universal_names() {
    let linux = AdapterProvider::new("linux", "linux-1", LinuxCapabilityAdapter);
    let windows = AdapterProvider::new("windows", "win-1", WindowsCapabilityAdapter);
    let macos = AdapterProvider::new("macos", "mac-1", MacOSCapabilityAdapter);
    let container = AdapterProvider::new("container", "ctr-1", ContainerCapabilityAdapter);
    for provider in [
        &linux as &dyn CapabilityProvider,
        &windows,
        &macos,
        &container,
    ] {
        assert!(provider
            .definitions()
            .iter()
            .any(|d| d.capability_id == "filesystem.read"));
        assert!(provider
            .definitions()
            .iter()
            .any(|d| d.capability_id == "process.start"));
    }
}

struct UnavailableProvider;
#[async_trait]
impl CapabilityProvider for UnavailableProvider {
    fn provider_id(&self) -> &str {
        "unavailable"
    }
    fn runtime_id(&self) -> &str {
        "test"
    }
    fn definitions(&self) -> Vec<CapabilityDefinition> {
        vec![CapabilityDefinition::basic(
            "test.failover",
            "test",
            vec![CapabilityRuntime::Linux],
            Idempotency::ReadOnly,
        )]
    }
    fn health(&self) -> CapabilityProviderHealth {
        CapabilityProviderHealth {
            availability: ProviderAvailability::Unavailable,
            ..Default::default()
        }
    }
    async fn execute(
        &self,
        _: CapabilityProviderContext,
    ) -> Result<CapabilityProviderResult, CapabilityError> {
        unreachable!()
    }
}
#[tokio::test]
async fn unavailable_provider_normalizes_to_runtime_unavailable() {
    let layer = UniversalCapabilityLayer::default();
    layer
        .register_provider(Arc::new(UnavailableProvider))
        .unwrap();
    let result = layer.execute(request("test.failover", json!({}))).await;
    assert_eq!(
        result.error.unwrap().code,
        CapabilityErrorCode::RuntimeUnavailable
    );
}
