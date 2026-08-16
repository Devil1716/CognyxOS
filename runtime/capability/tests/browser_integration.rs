mod test_server;

use cognyx_capability::{
    CapabilityRequest, CapabilityStatus, UniversalBrowserProvider, UniversalCapabilityLayer,
};
use serde_json::json;
use std::sync::Arc;
use test_server::start_test_server;

fn request(capability: &str, args: serde_json::Value) -> CapabilityRequest {
    CapabilityRequest {
        request_id: "test".into(),
        task_id: "test".into(),
        agent_id: "test".into(),
        capability_id: capability.into(),
        requested_version: None,
        runtime_hint: None,
        input: args,
        timeout_ms: Some(30000),
        trace_id: "test".into(),
        span_id: "test".into(),
    }
}

#[tokio::test]
async fn browser_playwright_availability_check() {
    let provider = UniversalBrowserProvider::new("browser-test", "test-runtime");
    let layer = UniversalCapabilityLayer::default();
    layer.register_provider(Arc::new(provider)).unwrap();

    let open_result = layer
        .execute(request("browser.open", json!({"url": "about:blank"})))
        .await;
    if open_result.status != CapabilityStatus::Completed {
        // Playwright not available - skip
        eprintln!(
            "Skipping tests: Playwright unavailable: {}",
            open_result.error.unwrap().message
        );
        return;
    }
    let session_id = open_result.output["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    layer
        .execute(request("browser.close", json!({"session_id": &session_id})))
        .await;
}

#[tokio::test]
async fn browser_open_and_close_session() {
    let provider = UniversalBrowserProvider::new("browser-test", "test-runtime");
    let layer = UniversalCapabilityLayer::default();
    layer.register_provider(Arc::new(provider)).unwrap();

    let open_result = layer
        .execute(request("browser.open", json!({"url": "about:blank"})))
        .await;
    if open_result.status != CapabilityStatus::Completed {
        return;
    }

    let session_id = open_result.output["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    let close_result = layer
        .execute(request("browser.close", json!({"session_id": &session_id})))
        .await;
    assert_eq!(close_result.status, CapabilityStatus::Completed);
}

#[tokio::test]
async fn browser_navigate_and_read_local_page() {
    let provider = UniversalBrowserProvider::new("browser-test", "test-runtime");
    let layer = UniversalCapabilityLayer::default();
    layer.register_provider(Arc::new(provider)).unwrap();

    let (base_url, _server) = start_test_server().await;

    let open_result = layer
        .execute(request("browser.open", json!({"url": base_url})))
        .await;
    if open_result.status != CapabilityStatus::Completed {
        return;
    }
    let session_id = open_result.output["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    let read = layer
        .execute(request("browser.read", json!({"session_id": &session_id})))
        .await;
    assert_eq!(read.status, CapabilityStatus::Completed);
    assert!(read.output["text"].as_str().unwrap().contains("CognyxOS"));

    layer
        .execute(request("browser.close", json!({"session_id": &session_id})))
        .await;
}

#[tokio::test]
async fn browser_click_button_on_local_page() {
    let provider = UniversalBrowserProvider::new("browser-test", "test-runtime");
    let layer = UniversalCapabilityLayer::default();
    layer.register_provider(Arc::new(provider)).unwrap();

    let (base_url, _server) = start_test_server().await;

    let open_result = layer
        .execute(request("browser.open", json!({"url": base_url})))
        .await;
    if open_result.status != CapabilityStatus::Completed {
        return;
    }
    let session_id = open_result.output["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    let click = layer
        .execute(request(
            "browser.click",
            json!({"session_id": &session_id, "selector": "#test-button"}),
        ))
        .await;
    assert_eq!(click.status, CapabilityStatus::Completed);

    let read = layer
        .execute(request("browser.read", json!({"session_id": &session_id})))
        .await;
    assert!(read.output["text"]
        .as_str()
        .unwrap()
        .contains("Button clicked!"));

    layer
        .execute(request("browser.close", json!({"session_id": &session_id})))
        .await;
}

#[tokio::test]
async fn browser_type_into_input() {
    let provider = UniversalBrowserProvider::new("browser-test", "test-runtime");
    let layer = UniversalCapabilityLayer::default();
    layer.register_provider(Arc::new(provider)).unwrap();

    let (base_url, _server) = start_test_server().await;

    let open_result = layer
        .execute(request("browser.open", json!({"url": base_url})))
        .await;
    if open_result.status != CapabilityStatus::Completed {
        return;
    }
    let session_id = open_result.output["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    let type_res = layer
        .execute(request(
            "browser.type",
            json!({"session_id": &session_id, "selector": "#test-input", "text": "Hello World"}),
        ))
        .await;
    assert_eq!(type_res.status, CapabilityStatus::Completed);

    layer
        .execute(request(
            "browser.click",
            json!({"session_id": &session_id, "selector": "#test-button"}),
        ))
        .await;

    let read = layer
        .execute(request("browser.read", json!({"session_id": &session_id})))
        .await;
    assert!(read.output["text"]
        .as_str()
        .unwrap()
        .contains("Button clicked! Input: Hello World"));

    layer
        .execute(request("browser.close", json!({"session_id": &session_id})))
        .await;
}

#[tokio::test]
async fn browser_screenshot_local_page() {
    let provider = UniversalBrowserProvider::new("browser-test", "test-runtime");
    let layer = UniversalCapabilityLayer::default();
    layer.register_provider(Arc::new(provider)).unwrap();

    let (base_url, _server) = start_test_server().await;

    let open_result = layer
        .execute(request("browser.open", json!({"url": base_url})))
        .await;
    if open_result.status != CapabilityStatus::Completed {
        return;
    }
    let session_id = open_result.output["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    let screenshot = layer
        .execute(request(
            "browser.screenshot",
            json!({"session_id": &session_id}),
        ))
        .await;
    assert_eq!(screenshot.status, CapabilityStatus::Completed);
    let b64 = screenshot.output["image_b64"].as_str().unwrap();
    assert!(!b64.is_empty());

    layer
        .execute(request("browser.close", json!({"session_id": &session_id})))
        .await;
}
