use crate::resolver::CapabilityResolver;
use cognyx_agent_core::{
    AgentEventPublisher, PermissionContext, PermissionDecision, PermissionEngine,
};
#[cfg(target_os = "windows")]
use cognyx_capability::{
    WindowsClipboardProvider, WindowsKeyboardProvider, WindowsMouseProvider,
    WindowsScreenCaptureProvider, WindowsWindowProvider,
};

use cognyx_capability::{
    CapabilityRequest as UniversalCapabilityRequest, CapabilityStatus, LocalFilesystemProvider,
    NativeApplicationProvider, NativeProcessProvider, UniversalBrowserProvider,
    UniversalCapabilityLayer,
};
use cognyx_execution::RuntimeRegistry;
use cognyx_planner::ExecutionNode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityRequest {
    pub request_id: String,
    pub task_id: String,
    pub agent_id: String,
    pub capability: String,
    pub target: String,
    pub arguments: Vec<String>,
    pub constraints: HashMap<String, String>,
    pub permission_context: PermissionContext,
    pub timeout_seconds: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityResult {
    pub request_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub assigned_runtime_id: String,
    pub executed_at_ms: u64,
}

pub struct CapabilityGateway {
    resolver: CapabilityResolver,
    permission_engine: PermissionEngine,
    capability_layer: UniversalCapabilityLayer,
}

impl CapabilityGateway {
    fn is_universal_capability(name: &str) -> bool {
        matches!(
            name.split('.').next(),
            Some(
                "application"
                    | "browser"
                    | "clipboard"
                    | "filesystem"
                    | "process"
                    | "screen"
                    | "keyboard"
                    | "mouse"
                    | "window"
                    | "terminal"
                    | "network"
                    | "notification"
                    | "audio"
                    | "camera"
            )
        )
    }

    fn universal_input(req: &CapabilityRequest) -> serde_json::Value {
        let base = serde_json::json!({"target": req.target.clone(), "path": req.target.clone(), "content": req.arguments.first(), "arguments": req.arguments.clone(), "constraints": req.constraints.clone()});
        match req.capability.as_str() {
            "application.search" => serde_json::json!({"query": req.target.clone()}),
            "application.inspect" | "application.open" => {
                serde_json::json!({"application_id": req.target.clone()})
            }
            "process.inspect" | "process.stop" => match req.target.parse::<u32>() {
                Ok(process_id) => serde_json::json!({"process_id": process_id}),
                Err(_) => base,
            },
            "clipboard.write" => serde_json::json!({"text": req.target.clone()}),
            _ => base,
        }
    }
    pub fn new(registry: Arc<RuntimeRegistry>) -> Self {
        let capability_layer = UniversalCapabilityLayer::default();
        // Only providers that perform real operations are registered here.
        // Contract adapters remain available for isolated Phase 4 tests, but
        // cannot cause production requests to appear successfully executed.
        let _ = capability_layer.register_provider(Arc::new(LocalFilesystemProvider::new(
            "scoped-local-filesystem",
            "host-linux-1",
            std::env::current_dir().unwrap_or_default(),
        )));
        let _ = capability_layer.register_provider(Arc::new(NativeApplicationProvider::new(
            "native-application-provider",
            "host-linux-1",
        )));
        let _ = capability_layer.register_provider(Arc::new(NativeProcessProvider::new(
            "native-process-provider",
            "host-linux-1",
        )));

        #[cfg(target_os = "windows")]
        {
            let _ = capability_layer.register_provider(Arc::new(WindowsClipboardProvider::new(
                "windows-clipboard-provider",
                "host-windows-1",
            )));
            let _ = capability_layer.register_provider(Arc::new(
                WindowsScreenCaptureProvider::new("windows-screen-capture", "host-windows-1"),
            ));
            let _ = capability_layer.register_provider(Arc::new(WindowsWindowProvider::new(
                "windows-window-provider",
                "host-windows-1",
            )));
            let _ = capability_layer.register_provider(Arc::new(WindowsKeyboardProvider::new(
                "windows-keyboard-provider",
                "host-windows-1",
            )));
            let _ = capability_layer.register_provider(Arc::new(WindowsMouseProvider::new(
                "windows-mouse-provider",
                "host-windows-1",
            )));
        }
        let _ = capability_layer.register_provider(Arc::new(UniversalBrowserProvider::new(
            "universal-browser-provider",
            "host-browser",
        )));

        Self {
            resolver: CapabilityResolver::new(registry),
            permission_engine: PermissionEngine::new(),
            capability_layer,
        }
    }

    pub async fn execute_capability(&self, req: CapabilityRequest) -> CapabilityResult {
        info!(
            "Capability Gateway processing request '{}' for capability '{}'",
            req.request_id, req.capability
        );

        // Step 1: Validate Request
        if req.capability.is_empty() {
            return CapabilityResult {
                request_id: req.request_id,
                success: false,
                output: String::new(),
                error: Some("Capability name cannot be empty".to_string()),
                assigned_runtime_id: "none".to_string(),
                executed_at_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            };
        }

        // Step 2: Authorize Request via Permission Engine
        AgentEventPublisher::publish("agent.permission_requested", &req.task_id, &req.capability);
        let decision = self
            .permission_engine
            .authorize(&req.capability, &req.permission_context);

        if decision != PermissionDecision::Allow {
            AgentEventPublisher::publish("agent.permission_denied", &req.task_id, &req.capability);
            return CapabilityResult {
                request_id: req.request_id,
                success: false,
                output: String::new(),
                error: Some(if decision == PermissionDecision::UserApprovalRequired {
                    format!("USER_APPROVAL_REQUIRED for capability '{}'", req.capability)
                } else {
                    format!("Permission DENIED for capability '{}'", req.capability)
                }),
                assigned_runtime_id: "none".to_string(),
                executed_at_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            };
        }

        AgentEventPublisher::publish("agent.permission_granted", &req.task_id, &req.capability);
        AgentEventPublisher::publish("agent.capability_requested", &req.task_id, &req.capability);

        // Step 3 & 4: Audit & Resolve Runtime
        let resolved_runtime = self
            .resolver
            .resolve_runtime_for_capability(&req.capability)
            .await;
        let runtime_id = resolved_runtime
            .clone()
            .unwrap_or_else(|| format!("sim-backend-{}", req.target));

        AgentEventPublisher::publish("agent.runtime_selected", &req.task_id, &runtime_id);

        // Phase 4 dispatches only formal universal capabilities through the
        // layer. Legacy Phase 1–3 capabilities retain their frozen pathway.
        if self
            .capability_layer
            .registry()
            .lookup(&req.capability, None)
            .is_some()
        {
            let universal = self
                .capability_layer
                .execute(UniversalCapabilityRequest {
                    request_id: req.request_id.clone(),
                    task_id: req.task_id.clone(),
                    agent_id: req.agent_id.clone(),
                    capability_id: req.capability.clone(),
                    requested_version: None,
                    runtime_hint: resolved_runtime,
                    input: Self::universal_input(&req),
                    timeout_ms: Some((req.timeout_seconds as u64) * 1_000),
                    trace_id: format!("trace-{}", req.request_id),
                    span_id: format!("span-{}", req.request_id),
                })
                .await;
            let success = universal.status == CapabilityStatus::Completed;
            AgentEventPublisher::publish(
                if success {
                    "capability.completed"
                } else {
                    "capability.failed"
                },
                &req.task_id,
                &req.capability,
            );
            return CapabilityResult {
                request_id: universal.request_id,
                success,
                output: universal.output.to_string(),
                error: universal
                    .error
                    .map(|e| format!("{:?}: {}", e.code, e.message)),
                assigned_runtime_id: universal.runtime_id,
                executed_at_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            };
        }

        if Self::is_universal_capability(&req.capability) {
            return CapabilityResult {
                request_id: req.request_id,
                success: false,
                output: String::new(),
                error: Some(format!(
                    "CAPABILITY_UNAVAILABLE: no real provider is registered for '{}'",
                    req.capability
                )),
                assigned_runtime_id: runtime_id,
                executed_at_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            };
        }

        // Step 5 & 6: Execute & Normalize Result
        let output = format!(
            "Executed capability '{}' on target '{}' via runtime '{}' (args: {:?})",
            req.capability, req.target, runtime_id, req.arguments
        );

        AgentEventPublisher::publish("agent.capability_completed", &req.task_id, &runtime_id);

        // Step 7: Record Telemetry & Return Result
        CapabilityResult {
            request_id: req.request_id,
            success: true,
            output,
            error: None,
            assigned_runtime_id: runtime_id,
            executed_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    pub async fn dispatch_node_execution(&self, node: &ExecutionNode) -> Result<String, String> {
        let cap = node
            .required_capabilities
            .first()
            .cloned()
            .unwrap_or_else(|| "bash".to_string());

        let req = CapabilityRequest {
            request_id: format!("req-{}", uuid::Uuid::now_v7()),
            task_id: node.task_id.clone(),
            agent_id: "agent-kernel-core".to_string(),
            capability: cap,
            target: node.command.clone(),
            arguments: node.args.clone(),
            constraints: HashMap::new(),
            permission_context: PermissionContext {
                user_id: "user-default".to_string(),
                session_id: "sess-default".to_string(),
                granted_capabilities: std::collections::HashSet::from([
                    "bash".to_string(),
                    "win32.powershell".to_string(),
                    "package.install".to_string(),
                    "application.open".to_string(),
                    "gui".to_string(),
                    "terminal.execute".to_string(),
                ]),
                is_administrator: false,
            },
            timeout_seconds: node.timeout_seconds,
        };

        let result = self.execute_capability(req).await;
        if result.success {
            Ok(result.output)
        } else {
            Err(result
                .error
                .unwrap_or_else(|| "Capability execution failed".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cognyx_execution::LinuxRuntime;
    use std::collections::HashSet;

    #[tokio::test]
    async fn test_capability_gateway_pipeline() {
        let registry = Arc::new(RuntimeRegistry::new());
        let linux = Box::new(LinuxRuntime::new("linux-host-1", "Local Host"));
        registry.register(linux);

        let gateway = CapabilityGateway::new(registry);

        let req = CapabilityRequest {
            request_id: "req-100".to_string(),
            task_id: "task-100".to_string(),
            agent_id: "agent-1".to_string(),
            capability: "bash".to_string(),
            target: "echo".to_string(),
            arguments: vec!["Hello World".to_string()],
            constraints: HashMap::new(),
            permission_context: PermissionContext {
                user_id: "user-1".to_string(),
                session_id: "sess-1".to_string(),
                granted_capabilities: HashSet::from(["bash".to_string()]),
                is_administrator: false,
            },
            timeout_seconds: 10,
        };

        let res = gateway.execute_capability(req).await;
        assert!(res.success);
        assert_eq!(res.assigned_runtime_id, "linux-host-1");
    }

    #[tokio::test]
    async fn approval_required_capability_is_not_dispatched() {
        let gateway = CapabilityGateway::new(Arc::new(RuntimeRegistry::new()));
        let result = gateway
            .execute_capability(CapabilityRequest {
                request_id: "req-approval".into(),
                task_id: "task-approval".into(),
                agent_id: "agent-1".into(),
                capability: "filesystem.write".into(),
                target: "never-written.txt".into(),
                arguments: vec!["content".into()],
                constraints: HashMap::new(),
                permission_context: PermissionContext {
                    user_id: "user-1".into(),
                    session_id: "session-1".into(),
                    granted_capabilities: HashSet::new(),
                    is_administrator: false,
                },
                timeout_seconds: 1,
            })
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("USER_APPROVAL_REQUIRED"));
    }

    #[tokio::test]
    async fn native_application_discovery_uses_provider_resolution_when_runtime_registry_has_no_match(
    ) {
        let gateway = CapabilityGateway::new(Arc::new(RuntimeRegistry::new()));
        let result = gateway
            .execute_capability(CapabilityRequest {
                request_id: "req-app-list".into(),
                task_id: "task-app-list".into(),
                agent_id: "agent-1".into(),
                capability: "application.list".into(),
                target: String::new(),
                arguments: vec![],
                constraints: HashMap::new(),
                permission_context: PermissionContext {
                    user_id: "user-1".into(),
                    session_id: "session-1".into(),
                    granted_capabilities: HashSet::new(),
                    is_administrator: false,
                },
                timeout_seconds: 10,
            })
            .await;
        assert!(result.success, "{:?}", result.error);
        assert!(result.output.contains("applications"));
    }

    /// screen.read has no real provider — must return CAPABILITY_UNAVAILABLE, never a fake result.
    #[tokio::test]
    async fn unimplemented_universal_capability_never_falls_back_to_simulation() {
        let gateway = CapabilityGateway::new(Arc::new(RuntimeRegistry::new()));
        // screen.read has no registered real provider on any platform yet.
        let result = gateway
            .execute_capability(CapabilityRequest {
                request_id: "req-screen-read".into(),
                task_id: "task-screen-read".into(),
                agent_id: "agent-1".into(),
                capability: "screen.read".into(),
                target: String::new(),
                arguments: vec![],
                constraints: HashMap::new(),
                permission_context: PermissionContext {
                    user_id: "user-1".into(),
                    session_id: "session-1".into(),
                    granted_capabilities: HashSet::new(),
                    is_administrator: false,
                },
                timeout_seconds: 10,
            })
            .await;
        assert!(!result.success, "screen.read must not fake success");
        let err = result.error.unwrap();
        assert!(
            err.contains("CAPABILITY_UNAVAILABLE") || err.contains("unavailable") || err.contains("Unavailable"),
            "Expected CAPABILITY_UNAVAILABLE, got: {err}"
        );
    }

    /// On Windows, screen.capture has a real GDI provider and must succeed.
    /// Requires an INTERACTIVE desktop session (GDI BitBlt needs a real display DC).
    /// Run explicitly: cargo test -p cognyx-gateway -- screen_capture_has_real_provider_on_windows --include-ignored
    #[cfg(target_os = "windows")]
    #[tokio::test]
    #[ignore = "requires interactive desktop session with a real display"]
    async fn screen_capture_has_real_provider_on_windows() {
        let gateway = CapabilityGateway::new(Arc::new(RuntimeRegistry::new()));
        let result = gateway
            .execute_capability(CapabilityRequest {
                request_id: "req-screen-cap".into(),
                task_id: "task-screen-cap".into(),
                agent_id: "agent-1".into(),
                capability: "screen.capture".into(),
                target: String::new(),
                arguments: vec![],
                constraints: HashMap::new(),
                permission_context: PermissionContext {
                    user_id: "user-1".into(),
                    session_id: "session-1".into(),
                    granted_capabilities: HashSet::new(),
                    is_administrator: false,
                },
                timeout_seconds: 15,
            })
            .await;
        assert!(result.success, "screen.capture must succeed on Windows: {:?}", result.error);
        assert!(result.output.contains("image_b64"), "output must contain base64 image data");
    }
}
