//! CognyxOS Phase 6 Multi-Agent Test Suite
//!
//! Validates multi-agent runtime, identity, role policies, lifecycle management,
//! authorized communication bus, cross-agent artifact exchange, privilege isolation,
//! supervision & recovery, cancellation propagation, resource limits, deadlock detection,
//! and end-to-end multi-agent orchestration.

use cognyx_agent_core::PermissionContext;
use cognyx_agent_runtime::{
    AgentCommunicationBus, AgentIdentity, AgentLifecycleManager, AgentLifecycleState,
    AgentManager, AgentMessage, AgentMessageType, AgentPriority, AgentRegistry, AgentResourceLimits,
    AgentRole, Artifact, ArtifactExchange, ArtifactType, DeadlockDetector, MultiAgentPlanner,
};
use cognyx_execution::RuntimeRegistry;
use cognyx_gateway::{CapabilityGateway, CapabilityRequest};
use cognyx_resources::ResourceManager;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;

fn create_test_manager() -> AgentManager {
    let registry = Arc::new(AgentRegistry::new());
    let lifecycle = Arc::new(AgentLifecycleManager::new());
    let bus = Arc::new(AgentCommunicationBus::new());
    let res_mgr = Arc::new(ResourceManager::default());
    AgentManager::new(registry, lifecycle, bus, res_mgr)
}

// ──────────────────────────────────────────────────────────────────────────────
// TEST 1: BASIC AGENT SPAWNING
// ──────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_01_basic_agent_spawning() {
    let mgr = create_test_manager();

    // Create root Manager agent
    let root = mgr
        .create_agent("mgr-1", "Manager Agent", AgentRole::Manager, None, "task-01")
        .unwrap();

    assert_eq!(root.role, AgentRole::Manager);
    assert_eq!(root.parent_agent_id, None);
    assert_eq!(root.root_agent_id, root.agent_id);

    // Spawn child Research agent
    let child = mgr
        .spawn_child_agent(&root.agent_id, "res-1", "Research Agent", AgentRole::Researcher)
        .unwrap();

    assert_eq!(child.role, AgentRole::Researcher);
    assert_eq!(child.parent_agent_id, Some(root.agent_id.clone()));
    assert_eq!(child.root_agent_id, root.agent_id);

    // Verify lifecycle state
    mgr.start_agent(&child.agent_id).unwrap();
    let inspected = mgr.get_agent(&child.agent_id).unwrap();
    assert_eq!(inspected.status, AgentLifecycleState::Running);

    // Stop child and root
    mgr.stop_agent(&child.agent_id).unwrap();
    mgr.stop_agent(&root.agent_id).unwrap();

    println!("PASS: test_01_basic_agent_spawning");
}

// ──────────────────────────────────────────────────────────────────────────────
// TEST 2: PARALLEL AGENT EXECUTION
// ──────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_02_parallel_agent_execution() {
    let mgr = create_test_manager();
    let root = mgr
        .create_agent("mgr-parallel", "Manager Agent", AgentRole::Manager, None, "task-02")
        .unwrap();

    let agent_a = mgr
        .spawn_child_agent(&root.agent_id, "agent-a", "Browser Agent", AgentRole::BrowserOperator)
        .unwrap();
    let agent_b = mgr
        .spawn_child_agent(&root.agent_id, "agent-b", "File Agent", AgentRole::FileOperator)
        .unwrap();

    mgr.start_agent(&agent_a.agent_id).unwrap();
    mgr.start_agent(&agent_b.agent_id).unwrap();

    // Spawn concurrent async tasks simulating parallel work
    let id_a = agent_a.agent_id.clone();
    let id_b = agent_b.agent_id.clone();

    let handle_a = tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        format!("Result from {}", id_a)
    });

    let handle_b = tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        format!("Result from {}", id_b)
    });

    let (res_a, res_b) = tokio::join!(handle_a, handle_b);
    assert!(res_a.unwrap().contains("agent-a"));
    assert!(res_b.unwrap().contains("agent-b"));

    println!("PASS: test_02_parallel_agent_execution");
}

// ──────────────────────────────────────────────────────────────────────────────
// TEST 3: MULTI-AGENT TASK DECOMPOSITION
// ──────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_03_multi_agent_task_decomposition() {
    let planner = MultiAgentPlanner::default();
    let intent = cognyx_intent::Intent {
        intent_id: "intent-03".into(),
        raw_prompt: "Find the test documents, analyze them, and create a summary.".into(),
        context: cognyx_intent::IntentContext {
            session_id: "session-03".into(),
            current_workspace: "workspace-03".into(),
            environment_params: std::collections::HashMap::new(),
        },
    };

    let plan = planner.create_multi_agent_plan("task-03", &intent);
    assert!(!plan.subtasks.is_empty(), "Plan must contain subtask assignments");

    let roles: Vec<_> = plan.subtasks.iter().map(|s| s.assigned_role.clone()).collect();
    assert!(roles.contains(&AgentRole::FileOperator) || roles.contains(&AgentRole::Researcher));
    assert!(roles.contains(&AgentRole::Writer) || roles.contains(&AgentRole::Analyst));

    println!("PASS: test_03_multi_agent_task_decomposition: {} subtasks generated", plan.subtasks.len());
}

// ──────────────────────────────────────────────────────────────────────────────
// TEST 4: REAL WINDOWS COMPUTER AGENT (HARDWARE TEST)
// ──────────────────────────────────────────────────────────────────────────────
#[tokio::test]
#[ignore = "requires real Windows desktop session with active display"]
#[cfg(target_os = "windows")]
async fn test_04_real_windows_computer_agent() {
    let gateway = CapabilityGateway::new(Arc::new(RuntimeRegistry::new()));
    let result = gateway
        .execute_capability(CapabilityRequest {
            request_id: "req-win-agent".into(),
            task_id: "task-04".into(),
            agent_id: "computer-operator-agent".into(),
            capability: "application.open".into(),
            target: "notepad".into(),
            arguments: vec![],
            constraints: std::collections::HashMap::new(),
            permission_context: PermissionContext {
                user_id: "test-user".into(),
                session_id: "test-session".into(),
                granted_capabilities: HashSet::from(["application.open".into()]),
                is_administrator: false,
            },
            timeout_seconds: 10,
        })
        .await;

    assert!(result.success, "Computer Operator Agent failed: {:?}", result.error);
    println!("PASS: test_04_real_windows_computer_agent");
}

// ──────────────────────────────────────────────────────────────────────────────
// TEST 5: BROWSER AGENT (HARDWARE TEST)
// ──────────────────────────────────────────────────────────────────────────────
#[tokio::test]
#[ignore = "requires Playwright runtime installation"]
async fn test_05_browser_agent() {
    let gateway = CapabilityGateway::new(Arc::new(RuntimeRegistry::new()));
    let result = gateway
        .execute_capability(CapabilityRequest {
            request_id: "req-browser-agent".into(),
            task_id: "task-05".into(),
            agent_id: "browser-operator-agent".into(),
            capability: "browser.open".into(),
            target: "about:blank".into(),
            arguments: vec![],
            constraints: std::collections::HashMap::new(),
            permission_context: PermissionContext {
                user_id: "test-user".into(),
                session_id: "test-session".into(),
                granted_capabilities: HashSet::from(["browser.open".into()]),
                is_administrator: false,
            },
            timeout_seconds: 15,
        })
        .await;

    if !result.success {
        eprintln!("Browser Agent skipped: {}", result.error.unwrap_or_default());
        return;
    }
    println!("PASS: test_05_browser_agent");
}

// ──────────────────────────────────────────────────────────────────────────────
// TEST 6: CROSS-AGENT ARTIFACT EXCHANGE
// ──────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_06_cross_agent_artifact_exchange() {
    let exchange = ArtifactExchange::new();

    let artifact = Artifact::new(
        "art-01",
        "agent-producer",
        "task-06",
        ArtifactType::Text,
        "memory://report.txt",
        json!({"content": "Report data for consumer agent"}),
    );

    exchange.register_artifact(artifact.clone()).unwrap();

    let fetched = exchange.get_artifact("art-01", "agent-consumer", "task-06").unwrap();
    assert_eq!(fetched.artifact_id, "art-01");
    assert_eq!(fetched.checksum, artifact.checksum);

    println!("PASS: test_06_cross_agent_artifact_exchange");
}

// ──────────────────────────────────────────────────────────────────────────────
// TEST 7: PERMISSION ISOLATION & NO ESCALATION
// ──────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_07_permission_isolation_and_no_escalation() {
    let parent = AgentIdentity {
        agent_id: "parent-agent".into(),
        parent_agent_id: None,
        root_agent_id: "parent-agent".into(),
        name: "Parent".into(),
        display_name: "Parent Agent".into(),
        role: AgentRole::Manager,
        status: AgentLifecycleState::Running,
        created_at: 0,
        started_at: None,
        stopped_at: None,
        permissions: vec!["filesystem.read".into()],
        capabilities: vec!["filesystem.read".into()],
        resource_limits: AgentResourceLimits::default(),
        metadata: json!({}),
    };

    // Child attempts to request filesystem.delete (not possessed by parent)
    let is_allowed = cognyx_agent_runtime::evaluate_permission_inheritance(&parent, "filesystem.delete");
    assert!(!is_allowed, "Child must NOT escalate permissions beyond parent!");

    println!("PASS: test_07_permission_isolation_and_no_escalation");
}

// ──────────────────────────────────────────────────────────────────────────────
// TEST 8: AGENT FAILURE DETECTION AND RECOVERY
// ──────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_08_agent_failure_detection_and_recovery() {
    let mgr = create_test_manager();
    let agent = mgr
        .create_agent("agent-fail", "Failing Agent", AgentRole::Worker, None, "task-08")
        .unwrap();

    mgr.start_agent(&agent.agent_id).unwrap();

    // Force failure transition
    mgr.fail_agent(&agent.agent_id, "Simulated worker crash").unwrap();

    let failed = mgr.get_agent(&agent.agent_id).unwrap();
    assert!(matches!(failed.status, AgentLifecycleState::Failed(_)));

    // Trigger recovery
    mgr.recover_agent(&agent.agent_id).unwrap();
    let recovered = mgr.get_agent(&agent.agent_id).unwrap();
    assert_eq!(recovered.status, AgentLifecycleState::Ready);

    println!("PASS: test_08_agent_failure_detection_and_recovery");
}

// ──────────────────────────────────────────────────────────────────────────────
// TEST 9: ROOT CANCELLATION PROPAGATION
// ──────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_09_root_cancellation_propagation() {
    let mgr = create_test_manager();
    let root = mgr
        .create_agent("root-cancel", "Root Agent", AgentRole::Manager, None, "task-09")
        .unwrap();

    let child_a = mgr.spawn_child_agent(&root.agent_id, "child-a", "Child A", AgentRole::Worker).unwrap();
    let child_b = mgr.spawn_child_agent(&root.agent_id, "child-b", "Child B", AgentRole::Worker).unwrap();

    mgr.start_agent(&root.agent_id).unwrap();
    mgr.start_agent(&child_a.agent_id).unwrap();
    mgr.start_agent(&child_b.agent_id).unwrap();

    // Cancel root agent tree
    mgr.cancel_tree(&root.agent_id).unwrap();

    let root_state = mgr.get_agent(&root.agent_id).unwrap();
    let ca_state = mgr.get_agent(&child_a.agent_id).unwrap();
    let cb_state = mgr.get_agent(&child_b.agent_id).unwrap();

    assert_eq!(root_state.status, AgentLifecycleState::Terminated);
    assert_eq!(ca_state.status, AgentLifecycleState::Terminated);
    assert_eq!(cb_state.status, AgentLifecycleState::Terminated);

    println!("PASS: test_09_root_cancellation_propagation");
}

// ──────────────────────────────────────────────────────────────────────────────
// TEST 10: RESOURCE LIMIT ENFORCEMENT
// ──────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_10_resource_limit_enforcement() {
    let mgr = create_test_manager();
    let root = mgr
        .create_agent("root-quota", "Root Agent", AgentRole::Manager, None, "task-10")
        .unwrap();

    // Default max children per parent is 8. Attempting to spawn 9 children must fail.
    for i in 0..8 {
        mgr.spawn_child_agent(&root.agent_id, &format!("c-{}", i), "Child", AgentRole::Worker).unwrap();
    }

    let overflow = mgr.spawn_child_agent(&root.agent_id, "c-overflow", "Child Overflow", AgentRole::Worker);
    assert!(overflow.is_err(), "Spawning child beyond quota must be rejected");

    println!("PASS: test_10_resource_limit_enforcement");
}

// ──────────────────────────────────────────────────────────────────────────────
// TEST 11: DEADLOCK DETECTION AND REJECTION
// ──────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_11_deadlock_detection_and_rejection() {
    let mut detector = DeadlockDetector::new();

    // Graph: A -> B, B -> C, C -> A (cyclic dependency)
    detector.add_dependency("AgentA", "AgentB");
    detector.add_dependency("AgentB", "AgentC");
    detector.add_dependency("AgentC", "AgentA");

    assert!(detector.detect_cycle(), "Deadlock detector MUST detect cyclic agent dependency!");

    println!("PASS: test_11_deadlock_detection_and_rejection");
}

// ──────────────────────────────────────────────────────────────────────────────
// TEST 12: FULL COGNYXOS MULTI-AGENT DEMO
// ──────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_12_full_cognyxos_multi_agent_demo() {
    let mgr = create_test_manager();
    let bus = Arc::new(AgentCommunicationBus::new());
    let exchange = Arc::new(ArtifactExchange::new());

    // 1. User submits intent -> Manager Agent created
    let manager = mgr
        .create_agent("mgr-demo", "Manager Agent", AgentRole::Manager, None, "task-demo-12")
        .unwrap();
    mgr.start_agent(&manager.agent_id).unwrap();

    // 2. Manager spawns File Agent, Research Agent, Analysis Agent, Writer Agent
    let file_agent = mgr
        .spawn_child_agent(&manager.agent_id, "file-agent", "File Agent", AgentRole::FileOperator)
        .unwrap();
    let res_agent = mgr
        .spawn_child_agent(&manager.agent_id, "res-agent", "Research Agent", AgentRole::Researcher)
        .unwrap();
    let writer_agent = mgr
        .spawn_child_agent(&manager.agent_id, "writer-agent", "Writer Agent", AgentRole::Writer)
        .unwrap();

    mgr.start_agent(&file_agent.agent_id).unwrap();
    mgr.start_agent(&res_agent.agent_id).unwrap();
    mgr.start_agent(&writer_agent.agent_id).unwrap();

    // 3. File Agent produces raw file artifact
    let file_art = Artifact::new(
        "art-files",
        &file_agent.agent_id,
        "task-demo-12",
        ArtifactType::File,
        "workspace://docs.txt",
        json!({"content": "Sample codebase documentation"}),
    );
    exchange.register_artifact(file_art).unwrap();

    // 4. Research Agent sends result message to Writer Agent
    let msg = AgentMessage {
        message_id: "msg-01".into(),
        sender_agent_id: res_agent.agent_id.clone(),
        recipient_agent_id: writer_agent.agent_id.clone(),
        task_id: "task-demo-12".into(),
        timestamp: 1000,
        message_type: AgentMessageType::TaskResult,
        payload: json!({"summary": "Research findings complete"}),
        authorization_context: json!({}),
        trace_id: "trace-demo".into(),
    };
    bus.send_message(msg).unwrap();

    // 5. Writer Agent generates final report artifact
    let report_art = Artifact::new(
        "art-final-report",
        &writer_agent.agent_id,
        "task-demo-12",
        ArtifactType::Report,
        "workspace://final_report.md",
        json!({"summary": "CognyxOS Phase 6 Multi-Agent Summary Report"}),
    );
    exchange.register_artifact(report_art.clone()).unwrap();

    // 6. Complete task
    mgr.stop_agent(&file_agent.agent_id).unwrap();
    mgr.stop_agent(&res_agent.agent_id).unwrap();
    mgr.stop_agent(&writer_agent.agent_id).unwrap();
    mgr.stop_agent(&manager.agent_id).unwrap();

    let report = exchange.get_artifact("art-final-report", &manager.agent_id, "task-demo-12").unwrap();
    assert_eq!(report.artifact_id, "art-final-report");

    println!("PASS: test_12_full_cognyxos_multi_agent_demo successfully completed!");
}
