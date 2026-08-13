use cognyx_capability::*;
use serde_json::json;
use std::sync::Arc;

fn request(capability_id: &str, input: serde_json::Value) -> CapabilityRequest {
    CapabilityRequest {
        request_id: format!("native-{}", uuid::Uuid::now_v7()),
        task_id: "native-test".into(),
        agent_id: "test".into(),
        capability_id: capability_id.into(),
        requested_version: None,
        runtime_hint: Some("native-test".into()),
        input,
        timeout_ms: Some(10_000),
        trace_id: "trace".into(),
        span_id: "span".into(),
    }
}

#[tokio::test]
async fn native_process_listing_returns_real_host_processes() {
    let layer = UniversalCapabilityLayer::default();
    layer
        .register_provider(Arc::new(NativeProcessProvider::new(
            "process",
            "native-test",
        )))
        .unwrap();
    let result = layer.execute(request("process.list", json!({}))).await;
    assert_eq!(result.status, CapabilityStatus::Completed);
    assert!(result.output["processes"].as_array().is_some());
    assert_eq!(result.metadata["native"], true);
}

#[tokio::test]
async fn native_application_discovery_is_dynamic() {
    let layer = UniversalCapabilityLayer::default();
    layer
        .register_provider(Arc::new(NativeApplicationProvider::new(
            "apps",
            "native-test",
        )))
        .unwrap();
    let result = layer.execute(request("application.list", json!({}))).await;
    assert_eq!(result.status, CapabilityStatus::Completed);
    assert!(result.output["applications"].is_array());
    assert_eq!(result.metadata["cache_authoritative"], false);
}

#[cfg(target_os = "windows")]
#[tokio::test]
async fn allowlisted_terminal_executes_a_harmless_native_command() {
    let layer = UniversalCapabilityLayer::default();
    layer
        .register_provider(Arc::new(NativeTerminalProvider::new(
            "terminal",
            "native-test",
            std::env::current_dir().unwrap(),
            ["where.exe".to_string()],
        )))
        .unwrap();
    let result = layer
        .execute(request(
            "terminal.execute",
            json!({"executable":"where.exe", "args":["where.exe"], "working_directory":"."}),
        ))
        .await;
    assert_eq!(result.status, CapabilityStatus::Completed);
    assert_eq!(result.metadata["shell"], false);
    assert_eq!(result.output["exit_code"], 0);
}

#[tokio::test]
async fn terminal_rejects_unallowlisted_executables() {
    let layer = UniversalCapabilityLayer::default();
    layer
        .register_provider(Arc::new(NativeTerminalProvider::new(
            "terminal",
            "native-test",
            std::env::current_dir().unwrap(),
            Vec::<String>::new(),
        )))
        .unwrap();
    let result = layer
        .execute(request(
            "terminal.execute",
            json!({"executable":"not-allowed", "args":[]}),
        ))
        .await;
    assert_eq!(
        result.error.unwrap().code,
        CapabilityErrorCode::PermissionDenied
    );
}
