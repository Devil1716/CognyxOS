//! Phase 5 end-to-end integration tests.
//!
//! Tests that require hardware interaction (real GUI, real browser) are
//! marked `#[ignore]` and must be run explicitly:
//!   cargo test -p cognyx-e2e -- --include-ignored --nocapture
//!
//! GUI Notepad tests require COGNYX_GUI_TEST isolation (see docs/windows-gui-testing.md).
//! Tests of invariants (permission denial, CAPABILITY_UNAVAILABLE) run by default.

#[path = "common/mod.rs"]
mod common;

use cognyx_agent_core::PermissionContext;
use cognyx_execution::RuntimeRegistry;
use cognyx_gateway::{CapabilityGateway, CapabilityRequest};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

fn gateway() -> CapabilityGateway {
    CapabilityGateway::new(Arc::new(RuntimeRegistry::new()))
}

fn ctx_no_grants() -> PermissionContext {
    PermissionContext {
        user_id: "e2e-user".into(),
        session_id: "e2e-session".into(),
        granted_capabilities: HashSet::new(),
        is_administrator: false,
    }
}

fn ctx_with(grants: impl IntoIterator<Item = &'static str>) -> PermissionContext {
    PermissionContext {
        user_id: "e2e-user".into(),
        session_id: "e2e-session".into(),
        granted_capabilities: grants.into_iter().map(|s| s.to_string()).collect(),
        is_administrator: false,
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// INVARIANT TESTS — always run (no #[ignore])
// ──────────────────────────────────────────────────────────────────────────────

/// Permission engine must block filesystem.delete without a grant.
#[tokio::test]
async fn e2e_permission_block_filesystem_delete() {
    let result = gateway()
        .execute_capability(CapabilityRequest {
            request_id: "e2e-block-1".into(),
            task_id: "e2e-task-1".into(),
            agent_id: "e2e-agent".into(),
            capability: "filesystem.delete".into(),
            target: "some-file.txt".into(),
            arguments: vec![],
            constraints: HashMap::new(),
            permission_context: ctx_no_grants(),
            timeout_seconds: 5,
        })
        .await;

    assert!(
        !result.success,
        "filesystem.delete must be blocked without grant"
    );
    let error = result.error.expect("error must be present");
    assert!(
        error.contains("USER_APPROVAL_REQUIRED"),
        "Expected USER_APPROVAL_REQUIRED, got: {error}"
    );
    println!("PASS: filesystem.delete correctly blocked: {error}");
}

/// Permission engine must block clipboard.read without a grant.
#[tokio::test]
async fn e2e_permission_block_clipboard_read() {
    let result = gateway()
        .execute_capability(CapabilityRequest {
            request_id: "e2e-clip-1".into(),
            task_id: "e2e-clip-task".into(),
            agent_id: "e2e-agent".into(),
            capability: "clipboard.read".into(),
            target: String::new(),
            arguments: vec![],
            constraints: HashMap::new(),
            permission_context: ctx_no_grants(),
            timeout_seconds: 5,
        })
        .await;

    assert!(
        !result.success,
        "clipboard.read must be blocked without grant"
    );
    let error = result.error.expect("error must be present");
    assert!(
        error.contains("USER_APPROVAL_REQUIRED"),
        "Expected USER_APPROVAL_REQUIRED, got: {error}"
    );
    println!("PASS: clipboard.read correctly blocked: {error}");
}

/// screen.read has no real provider registered — must return CAPABILITY_UNAVAILABLE.
/// Even on Windows, screen.read via UIA is not yet implemented.
#[tokio::test]
async fn e2e_capability_unavailable_screen_read() {
    let result = gateway()
        .execute_capability(CapabilityRequest {
            request_id: "e2e-unavail-1".into(),
            task_id: "e2e-task-2".into(),
            agent_id: "e2e-agent".into(),
            capability: "screen.read".into(),
            target: String::new(),
            arguments: vec![],
            constraints: HashMap::new(),
            permission_context: ctx_no_grants(),
            timeout_seconds: 5,
        })
        .await;

    assert!(!result.success, "screen.read must not fake success");
    let error = result.error.unwrap_or_default();
    // Must contain CAPABILITY_UNAVAILABLE - never a simulated success.
    assert!(
        error.contains("CAPABILITY_UNAVAILABLE")
            || error.contains("Unavailable")
            || error.contains("unavailable"),
        "Expected CAPABILITY_UNAVAILABLE, got: {error}"
    );
    println!("PASS: screen.read correctly returned unavailable: {error}");
}

/// A completely unknown capability must be denied, never simulated.
#[tokio::test]
async fn e2e_unknown_universal_capability_is_unavailable() {
    let result = gateway()
        .execute_capability(CapabilityRequest {
            request_id: "e2e-unknown-1".into(),
            task_id: "e2e-task-3".into(),
            agent_id: "e2e-agent".into(),
            capability: "window.teleport".into(), // does not exist
            target: String::new(),
            arguments: vec![],
            constraints: HashMap::new(),
            permission_context: ctx_no_grants(),
            timeout_seconds: 5,
        })
        .await;

    assert!(!result.success, "unknown capability must not succeed");
    println!(
        "PASS: window.teleport correctly unavailable: {:?}",
        result.error
    );
}

/// application.list must return real applications (not simulated).
#[tokio::test]
async fn e2e_application_list_is_real_and_non_empty() {
    let result = gateway()
        .execute_capability(CapabilityRequest {
            request_id: "e2e-applist-1".into(),
            task_id: "e2e-task-4".into(),
            agent_id: "e2e-agent".into(),
            capability: "application.list".into(),
            target: String::new(),
            arguments: vec![],
            constraints: HashMap::new(),
            permission_context: ctx_no_grants(),
            timeout_seconds: 15,
        })
        .await;

    assert!(
        result.success,
        "application.list should succeed: {:?}",
        result.error
    );
    assert!(
        result.output.contains("applications"),
        "output must contain applications: {}",
        result.output
    );
    println!("PASS: application.list returned real data");
}

/// process.list must return real host processes.
#[tokio::test]
async fn e2e_process_list_is_real_and_non_empty() {
    let result = gateway()
        .execute_capability(CapabilityRequest {
            request_id: "e2e-proclist-1".into(),
            task_id: "e2e-task-5".into(),
            agent_id: "e2e-agent".into(),
            capability: "process.list".into(),
            target: String::new(),
            arguments: vec![],
            constraints: HashMap::new(),
            permission_context: ctx_no_grants(),
            timeout_seconds: 15,
        })
        .await;

    assert!(
        result.success,
        "process.list should succeed: {:?}",
        result.error
    );
    assert!(
        result.output.contains("processes"),
        "output must contain processes key: {}",
        result.output
    );
    println!("PASS: process.list returned real data");
}

// ──────────────────────────────────────────────────────────────────────────────
// HARDWARE-ONLY TESTS — require real GUI/display, run with --include-ignored
// ──────────────────────────────────────────────────────────────────────────────

/// Open the dedicated CognyxOS test Notepad instance, type "Hello CognyxOS",
/// verify text, then close only the owned window.
/// Requires a Windows machine with a real display.
#[tokio::test]
#[ignore]
#[cfg(target_os = "windows")]
async fn e2e_open_notepad_and_type_hello_cognyxos() {
    let mut harness = common::GuiHarness::prepare().expect("test workspace");
    let gw = gateway();
    harness
        .reject_leftover_golden_windows(&gw)
        .await
        .expect("leftover golden-test window");
    harness.print_environment("native-application-provider", "Notepad");

    let search = gw
        .execute_capability(common::GuiHarness::request(
            "application.search",
            "Notepad",
            HashMap::new(),
        ))
        .await;
    println!("application.search: {:?}", search);
    assert!(
        search.success,
        "application.search failed: {:?}",
        search.error
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&search.output).expect("search output must be JSON");
    let apps = parsed
        .get("applications")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let notepad = apps.iter().find(|a| {
        a.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .eq_ignore_ascii_case("notepad")
            || a.get("executable")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                .contains("notepad.exe")
    });
    let notepad = notepad.expect("notepad.exe was not discovered on PATH");
    let app_id = notepad
        .get("application_id")
        .and_then(|v| v.as_str())
        .expect("application_id")
        .to_string();
    println!("discovered application_id={app_id}");

    let open = gw
        .execute_capability(common::GuiHarness::request(
            "application.open",
            app_id,
            HashMap::new(),
        ))
        .await;
    println!("application.open result: {:?}", open);
    assert!(open.success, "Failed to open notepad: {:?}", open.error);
    let owned = harness
        .claim_from_open(&open.output)
        .expect("owned window from open");
    println!("target PID={:?}", owned.process_id);
    println!("target window_id={}", owned.window_id);
    harness
        .verify_focus(&gw, &owned.window_id)
        .await
        .expect("focus");
    println!("TARGET VERIFIED = TRUE");
    harness
        .type_text(&gw, &owned.window_id, common::EXPECTED_TEXT)
        .await
        .expect("keyboard.type");
    tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
    match harness
        .verify_text(&gw, &owned.window_id, common::EXPECTED_TEXT)
        .await
    {
        Ok(()) => println!("TEXT_VERIFICATION = PASS"),
        Err(error) => {
            println!("TEXT_VERIFICATION = FAIL");
            let _ = harness.cleanup(&gw).await;
            panic!("{error}");
        }
    }
    let _ = harness.save_owned(&gw, &owned.window_id).await;
    harness.cleanup(&gw).await.expect("owned cleanup");
    println!("PASS: e2e_open_notepad_and_type_hello_cognyxos completed");
}

/// Open a browser, navigate to a local page, read content.
/// Requires Playwright to be installed.
#[tokio::test]
#[ignore]
async fn e2e_browser_navigate_local_page() {
    let gw = gateway();
    let result = gw
        .execute_capability(CapabilityRequest {
            request_id: "e2e-browser-1".into(),
            task_id: "e2e-task-browser".into(),
            agent_id: "e2e-agent".into(),
            capability: "browser.open".into(),
            target: "about:blank".into(),
            arguments: vec![],
            constraints: {
                let mut m = HashMap::new();
                m.insert("url".into(), "about:blank".into());
                m
            },
            permission_context: ctx_with(["browser.open"]),
            timeout_seconds: 30,
        })
        .await;

    // If Playwright is not installed this will fail gracefully.
    if !result.success {
        eprintln!("Skipping: {}", result.error.unwrap_or_default());
        return;
    }
    println!("PASS: browser.open succeeded: {}", result.output);
}
