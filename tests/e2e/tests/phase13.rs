//! Phase 13: natural-language application plans through the real kernel path.
//!
//! Hardware GUI tests are `#[ignore]` and must be run with:
//!   COGNYX_GUI_TEST=1 cargo test -p cognyx-e2e --test phase13 -- --include-ignored --nocapture

mod common;

use cognyx_agent_core::PermissionContext;
use cognyx_agent_kernel::AgentKernelServer;
use cognyx_execution::{native_host_runtime_id, RuntimeRegistry};
use cognyx_gateway::{CapabilityGateway, CapabilityRequest};
use cognyx_intent::IntentEngine;
use cognyx_planner::{AgentPlanner, Plan, PlanStep};
use cognyx_service_workspace::{ctx_for, WorkspaceManager};
use cognyx_shell::{AgentKernelAdapter, CognyxShell};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

fn capabilities(plan: &cognyx_planner::Plan) -> Vec<&str> {
    plan.steps
        .iter()
        .map(|step| step.required_capabilities[0].as_str())
        .collect()
}

#[tokio::test]
async fn test_1_open_notepad_plans_search_then_open() {
    let intent = IntentEngine::default().parse_prompt("Open Notepad").await;
    let plan = AgentPlanner::default()
        .create_plan("task-1", &intent)
        .await
        .unwrap();
    assert!(plan.validate().is_valid);
    assert_eq!(
        capabilities(&plan),
        vec!["application.search", "application.open"]
    );
    assert_eq!(plan.steps[0].parameters["query"], "Notepad");
    assert_eq!(
        plan.steps[1].parameters["application_id"],
        "${step-1.applications[0].application_id}"
    );
    assert!(!capabilities(&plan).contains(&"bash"));
}

#[tokio::test]
async fn test_2_open_notepad_and_type_plans_search_open_type() {
    let intent = IntentEngine::default()
        .parse_prompt("Open Notepad and type Hello CognyxOS")
        .await;
    assert_eq!(intent.parameters.get("application").unwrap(), "Notepad");
    assert_eq!(intent.parameters.get("text").unwrap(), "Hello CognyxOS");
    let plan = AgentPlanner::default()
        .create_plan("task-2", &intent)
        .await
        .unwrap();
    assert!(plan.validate().is_valid);
    assert_eq!(
        capabilities(&plan),
        vec!["application.search", "application.open", "keyboard.type"]
    );
    assert_eq!(plan.steps[2].parameters["text"], "Hello CognyxOS");
    assert_eq!(plan.steps[2].depends_on_step_ids, vec!["step-2"]);
}

#[tokio::test]
async fn test_3_open_calculator_plans_search_then_open() {
    let intent = IntentEngine::default()
        .parse_prompt("Open Calculator")
        .await;
    let plan = AgentPlanner::default()
        .create_plan("task-3", &intent)
        .await
        .unwrap();
    assert!(plan.validate().is_valid);
    assert_eq!(
        capabilities(&plan),
        vec!["application.search", "application.open"]
    );
    assert_eq!(plan.steps[0].parameters["query"], "Calculator");
}

#[tokio::test]
async fn test_4_unknown_application_fails_search_through_real_shell() {
    let server = Arc::new(AgentKernelServer::new());
    let host_id = native_host_runtime_id();
    let ws_mgr = Arc::new(WorkspaceManager::new(Arc::clone(&server.registry)));
    ws_mgr.attach_in_memory_runtime(host_id);
    let ctx = ctx_for("user", &["filesystem.write", "filesystem.read"]);
    let ws = ws_mgr
        .create_workspace("phase13", "user", host_id, &ctx)
        .unwrap();
    let kernel = Arc::new(AgentKernelAdapter::from_server(Arc::clone(&server)));
    let shell = CognyxShell::new(kernel, ws_mgr, ws.id);
    let task = shell.submit_intent("Open ZyxxNotAnApp999").await.unwrap();
    assert_ne!(task.task_id, "task-1", "must not be RecordingKernel ids");
    assert!(task.task_id.len() > 8);
    let inspected = wait_shell(&shell, &task.task_id).await;
    assert_eq!(inspected.status, "failed");
    let error = inspected.error.unwrap_or_default();
    assert!(error.contains("APPLICATION_NOT_FOUND"), "got {error}");
}

#[test]
fn test_5_missing_application_id_is_plan_invalid() {
    let plan = Plan {
        plan_id: "p".into(),
        task_id: "t".into(),
        steps: vec![PlanStep {
            step_id: "step-1".into(),
            description: "open".into(),
            target_runtime_kind: "WindowsVm".into(),
            required_capabilities: vec!["application.open".into()],
            depends_on_step_ids: vec![],
            parameters: HashMap::new(),
            preconditions: vec![],
            postconditions: vec![],
        }],
        constraints: vec![],
        created_at_ms: 0,
    };
    assert!(plan
        .validate()
        .validation_errors
        .iter()
        .any(|error| error.contains("PLAN_INVALID") && error.contains("application_id")));
}

#[test]
fn test_6_missing_keyboard_text_is_plan_invalid() {
    let plan = Plan {
        plan_id: "p".into(),
        task_id: "t".into(),
        steps: vec![PlanStep {
            step_id: "step-1".into(),
            description: "type".into(),
            target_runtime_kind: "WindowsVm".into(),
            required_capabilities: vec!["keyboard.type".into()],
            depends_on_step_ids: vec![],
            parameters: HashMap::new(),
            preconditions: vec![],
            postconditions: vec![],
        }],
        constraints: vec![],
        created_at_ms: 0,
    };
    assert!(plan
        .validate()
        .validation_errors
        .iter()
        .any(|error| error.contains("keyboard.type requires 'text'")));
}

#[tokio::test]
async fn test_7_terminal_execute_still_requires_authorization() {
    let gateway = CapabilityGateway::new(Arc::new(RuntimeRegistry::new()));
    let denied = gateway
        .execute_capability(CapabilityRequest {
            request_id: "term-deny".into(),
            task_id: "t".into(),
            agent_id: "a".into(),
            capability: "terminal.execute".into(),
            target: "hostname".into(),
            arguments: vec![],
            constraints: HashMap::new(),
            permission_context: PermissionContext {
                user_id: "u".into(),
                session_id: "s".into(),
                granted_capabilities: HashSet::new(),
                is_administrator: false,
            },
            timeout_seconds: 5,
        })
        .await;
    assert!(!denied.success);
    let error = denied.error.unwrap_or_default();
    assert!(
        error.contains("USER_APPROVAL_REQUIRED") || error.contains("DENIED"),
        "got {error}"
    );
}

#[tokio::test]
async fn test_8_permission_engine_still_blocks_filesystem_delete() {
    let gateway = CapabilityGateway::new(Arc::new(RuntimeRegistry::new()));
    let denied = gateway
        .execute_capability(CapabilityRequest {
            request_id: "fs-deny".into(),
            task_id: "t".into(),
            agent_id: "a".into(),
            capability: "filesystem.delete".into(),
            target: "never-delete.txt".into(),
            arguments: vec![],
            constraints: HashMap::new(),
            permission_context: PermissionContext {
                user_id: "u".into(),
                session_id: "s".into(),
                granted_capabilities: HashSet::new(),
                is_administrator: false,
            },
            timeout_seconds: 5,
        })
        .await;
    assert!(!denied.success);
    assert!(denied
        .error
        .unwrap_or_default()
        .contains("USER_APPROVAL_REQUIRED"));
}

#[tokio::test]
async fn multi_action_open_type_close_has_explicit_dependencies() {
    let intent = IntentEngine::default()
        .parse_prompt("Open Notepad, type Hello, and close it")
        .await;
    let plan = AgentPlanner::default()
        .create_plan("task-multi", &intent)
        .await
        .unwrap();
    assert!(plan.validate().is_valid);
    assert_eq!(
        capabilities(&plan),
        vec![
            "application.search",
            "application.open",
            "keyboard.type",
            "window.close"
        ]
    );
    assert_eq!(plan.steps[3].parameters["window_id"], "${step-2.window_id}");
    assert!(plan.steps[3]
        .depends_on_step_ids
        .iter()
        .any(|id| id == "step-2"));
    assert!(plan.steps[3]
        .depends_on_step_ids
        .iter()
        .any(|id| id == "step-3"));
}

/// Golden path through production Shell → AgentKernelAdapter → AgentKernelServer.
/// Requires a real Windows desktop session and COGNYX_GUI_TEST isolation.
#[tokio::test]
#[ignore = "requires real Windows desktop session with active display"]
#[cfg(target_os = "windows")]
async fn golden_shell_open_notepad_and_type_hello_cognyxos() {
    let mut harness = common::GuiHarness::prepare().expect("test workspace");
    let server = Arc::new(AgentKernelServer::new());
    let host_id = native_host_runtime_id();
    assert!(
        host_id.contains("windows"),
        "golden test must use windows-host, got {host_id}"
    );
    let ws_mgr = Arc::new(WorkspaceManager::new(Arc::clone(&server.registry)));
    ws_mgr.attach_in_memory_runtime(host_id);
    let ctx = ctx_for("user", &["filesystem.write", "filesystem.read"]);
    let ws = ws_mgr
        .create_workspace("phase13-hw", "user", host_id, &ctx)
        .unwrap();
    let kernel = Arc::new(AgentKernelAdapter::from_server(Arc::clone(&server)));
    let shell = CognyxShell::new(kernel, ws_mgr, ws.id);
    let gw = CapabilityGateway::new(Arc::clone(&server.registry));
    harness
        .reject_leftover_golden_windows(&gw)
        .await
        .expect("leftover golden-test window");
    harness.snapshot(&gw).await.expect("window snapshot");
    harness.print_environment("native-application-provider", "Notepad");

    let task = shell
        .submit_intent("Open Notepad and type Hello CognyxOS")
        .await
        .unwrap();
    assert!(!task.task_id.eq("task-1"), "must not be RecordingKernel");
    let inspected = wait_shell(&shell, &task.task_id).await;
    println!(
        "golden status={} error={:?} runtime={:?}",
        inspected.status, inspected.error, inspected.runtime_id
    );
    assert_eq!(
        inspected.status, "completed",
        "golden kernel path failed: {:?}",
        inspected.error
    );
    tokio::time::sleep(Duration::from_millis(500)).await;

    let owned = harness
        .discover_owned(&gw)
        .await
        .expect("test-owned window");
    println!("target PID={:?}", owned.process_id);
    println!("target window_id={}", owned.window_id);
    println!("target title={}", owned.title);
    harness
        .verify_focus(&gw, &owned.window_id)
        .await
        .expect("focus");
    println!("TARGET VERIFIED = TRUE");
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
}

#[tokio::test]
#[ignore = "requires real Windows desktop session with active display"]
#[cfg(target_os = "windows")]
async fn hardware_notepad_search_open_type_close() {
    let mut harness = common::GuiHarness::prepare().expect("test workspace");
    let gw = CapabilityGateway::new(Arc::new(RuntimeRegistry::new()));
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
    assert!(search.success, "{:?}", search.error);
    assert!(
        !search.assigned_runtime_id.to_lowercase().contains("linux"),
        "{}",
        search.assigned_runtime_id
    );
    let parsed: serde_json::Value = serde_json::from_str(&search.output).unwrap();
    let app_id = parsed["applications"][0]["application_id"]
        .as_str()
        .expect("application_id")
        .to_string();
    let open = gw
        .execute_capability(common::GuiHarness::request(
            "application.open",
            app_id,
            HashMap::new(),
        ))
        .await;
    assert!(open.success, "{:?}", open.error);
    println!("application.open {}", open.output);
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
    tokio::time::sleep(Duration::from_millis(400)).await;
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
    harness
        .save_owned(&gw, &owned.window_id)
        .await
        .expect("save test document");
    harness.cleanup(&gw).await.expect("owned cleanup");
}

#[test]
fn safety_filter_rejects_personal_targets() {
    assert!(cognyx_capability::gui_test::is_protected_title(
        "*.env - Notepad"
    ));
    assert!(cognyx_capability::gui_test::is_protected_path(
        r"C:\Users\someone\Documents\notes.txt"
    ));
    assert!(cognyx_capability::gui_test::is_test_owned_title(
        "CognyxOS-Golden-Test.txt - Notepad"
    ));
    assert!(!cognyx_capability::gui_test::is_test_owned_title(
        "Untitled - Notepad"
    ));
}

async fn wait_shell(
    shell: &CognyxShell<AgentKernelAdapter>,
    task_id: &str,
) -> cognyx_shell::TaskView {
    let mut view = shell.inspect_task(task_id).await.unwrap();
    for _ in 0..60 {
        if view.status == "completed" || view.status == "failed" || view.status == "cancelled" {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
        view = shell.inspect_task(task_id).await.unwrap();
    }
    view
}
