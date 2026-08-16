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
use cognyx_execution::{native_host_runtime_id, RuntimeRegistry};
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
    fn validate_capability_request(req: &CapabilityRequest) -> Result<(), String> {
        let required = match req.capability.as_str() {
            "application.search" => Some("query"),
            "application.open" | "application.inspect" => Some("application_id"),
            "keyboard.type" => Some("text"),
            "window.focus" | "window.activate" | "window.close" => Some("window_id"),
            "browser.navigate" => Some("url"),
            "process.stop" => Some("process_id"),
            "terminal.execute" => Some("command"),
            "filesystem.write" => Some("destination"),
            _ => None,
        };
        if let Some(name) = required {
            if req.target.trim().is_empty() {
                return Err(format!(
                    "PLAN_INVALID: {} requires {}",
                    req.capability, name
                ));
            }
        }
        Ok(())
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
            "keyboard.type" => {
                let mut input = serde_json::json!({"text": req.target.clone()});
                if let Some(window_id) = req.constraints.get("window_id") {
                    input["window_id"] = serde_json::Value::String(window_id.clone());
                }
                input
            }
            "keyboard.press" => serde_json::json!({"key": req.target.clone()}),
            "keyboard.hotkey" => {
                let keys: Vec<String> = req
                    .target
                    .split('+')
                    .map(|part| part.trim().to_ascii_lowercase())
                    .filter(|part| !part.is_empty())
                    .collect();
                let mut input = serde_json::json!({"keys": keys, "key": req.target.clone()});
                if let Some(window_id) = req.constraints.get("window_id") {
                    input["window_id"] = serde_json::Value::String(window_id.clone());
                }
                input
            }
            "window.focus" | "window.activate" | "window.close" | "window.inspect"
            | "window.minimize" | "window.maximize" => {
                serde_json::json!({"window_id": req.target.clone()})
            }
            _ => base,
        }
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn unavailable(req: &CapabilityRequest, runtime_id: String) -> CapabilityResult {
        CapabilityResult {
            request_id: req.request_id.clone(),
            success: false,
            output: String::new(),
            error: Some(format!(
                "CAPABILITY_UNAVAILABLE: no real provider is registered for '{}'",
                req.capability
            )),
            assigned_runtime_id: runtime_id,
            executed_at_ms: Self::now_ms(),
        }
    }

    fn dedicated_fs_root() -> std::path::PathBuf {
        #[cfg(target_os = "windows")]
        {
            std::path::PathBuf::from(r"C:\CognyxOSTestWorkspace")
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::temp_dir().join("CognyxOSTestWorkspace")
        }
    }

    pub fn new(registry: Arc<RuntimeRegistry>) -> Self {
        let capability_layer = UniversalCapabilityLayer::default();
        // Runtime identity comes from the native host OS / RuntimeRegistry, never
        // a hardcoded host-linux-1 label on Windows.
        let host_id = native_host_runtime_id();
        let ids = registry.list_runtime_ids();
        // Prefer a registered native-host id. Never fall back to a Linux
        // runtime identity on Windows (Notepad is not host-linux-1).
        let runtime_id = ids
            .iter()
            .find(|id| *id == host_id)
            .cloned()
            .unwrap_or_else(|| host_id.to_string());

        let fs_root = Self::dedicated_fs_root();
        let _ = std::fs::create_dir_all(&fs_root);

        // Only providers that perform real operations are registered here.
        // Contract adapters remain available for isolated Phase 4 tests, but
        // cannot cause production requests to appear successfully executed.
        let _ = capability_layer.register_provider(Arc::new(LocalFilesystemProvider::new(
            "scoped-local-filesystem",
            runtime_id.clone(),
            fs_root,
        )));
        let _ = capability_layer.register_provider(Arc::new(NativeApplicationProvider::new(
            "native-application-provider",
            runtime_id.clone(),
        )));
        let _ = capability_layer.register_provider(Arc::new(NativeProcessProvider::new(
            "native-process-provider",
            runtime_id.clone(),
        )));

        #[cfg(target_os = "windows")]
        {
            let _ = capability_layer.register_provider(Arc::new(WindowsClipboardProvider::new(
                "windows-clipboard-provider",
                runtime_id.clone(),
            )));
            let _ = capability_layer.register_provider(Arc::new(
                WindowsScreenCaptureProvider::new("windows-screen-capture", runtime_id.clone()),
            ));
            let _ = capability_layer.register_provider(Arc::new(WindowsWindowProvider::new(
                "windows-window-provider",
                runtime_id.clone(),
            )));
            let _ = capability_layer.register_provider(Arc::new(WindowsKeyboardProvider::new(
                "windows-keyboard-provider",
                runtime_id.clone(),
            )));
            let _ = capability_layer.register_provider(Arc::new(WindowsMouseProvider::new(
                "windows-mouse-provider",
                runtime_id.clone(),
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
                executed_at_ms: Self::now_ms(),
            };
        }
        if let Err(error) = Self::validate_capability_request(&req) {
            return CapabilityResult {
                request_id: req.request_id,
                success: false,
                output: String::new(),
                error: Some(error),
                assigned_runtime_id: "none".to_string(),
                executed_at_ms: Self::now_ms(),
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
                executed_at_ms: Self::now_ms(),
            };
        }

        AgentEventPublisher::publish("agent.permission_granted", &req.task_id, &req.capability);
        AgentEventPublisher::publish("agent.capability_requested", &req.task_id, &req.capability);

        // Step 3 & 4: Audit & Resolve Runtime. Never invent sim-backend.
        let resolved_runtime = self
            .resolver
            .resolve_runtime_for_capability(&req.capability)
            .await;
        let runtime_id = resolved_runtime
            .clone()
            .unwrap_or_else(|| "none".to_string());

        AgentEventPublisher::publish("agent.runtime_selected", &req.task_id, &runtime_id);

        // Dispatch only formal universal capabilities through the layer.
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
                error: universal.error.map(Self::format_provider_error),
                assigned_runtime_id: universal.runtime_id,
                executed_at_ms: Self::now_ms(),
            };
        }

        // VAL-001: no lookup and no real process execute → honest failure.
        // LinuxRuntime.execute_command / Windows automation are formatted
        // strings, not real execution. Never report success for that path.
        // Never use sim-backend in the production path.
        AgentEventPublisher::publish("agent.capability_failed", &req.task_id, &req.capability);
        Self::unavailable(&req, runtime_id)
    }

    fn format_provider_error(error: cognyx_capability::CapabilityError) -> String {
        match error.code {
            cognyx_capability::CapabilityErrorCode::ApplicationNotFound => {
                format!("APPLICATION_NOT_FOUND: {}", error.message)
            }
            _ if error.message.contains("TEST_TARGET_UNSAFE") => error.message,
            _ => format!("{:?}: {}", error.code, error.message),
        }
    }

    fn resolve_reference(value: &str, outputs: &HashMap<String, String>) -> Result<String, String> {
        let Some(reference) = value
            .strip_prefix("${")
            .and_then(|value| value.strip_suffix('}'))
        else {
            return Ok(value.to_string());
        };
        let mut parts = reference.split('.');
        let step_id = parts.next().unwrap_or_default();
        let output = outputs
            .get(step_id)
            .ok_or_else(|| format!("PLAN_INVALID: output for '{step_id}' is unavailable"))?;
        let mut current: serde_json::Value = serde_json::from_str(output)
            .map_err(|_| format!("PLAN_INVALID: output for '{step_id}' is not structured JSON"))?;
        for part in parts {
            let (field, index) = match part.split_once('[') {
                Some((field, rest)) => (
                    field,
                    Some(
                        rest.strip_suffix(']')
                            .ok_or_else(|| {
                                format!("PLAN_INVALID: malformed reference '{reference}'")
                            })?
                            .parse::<usize>()
                            .map_err(|_| {
                                format!("PLAN_INVALID: malformed reference '{reference}'")
                            })?,
                    ),
                ),
                None => (part, None),
            };
            if !field.is_empty() {
                current = current.get(field).cloned().ok_or_else(|| {
                    format!("APPLICATION_NOT_FOUND: reference '{reference}' was not produced")
                })?;
            }
            if let Some(index) = index {
                current = current.get(index).cloned().ok_or_else(|| {
                    format!("APPLICATION_NOT_FOUND: reference '{reference}' selected no result")
                })?;
            }
        }
        match current {
            serde_json::Value::String(value) => Ok(value),
            serde_json::Value::Number(value) => Ok(value.to_string()),
            serde_json::Value::Bool(value) => Ok(value.to_string()),
            _ => Err(format!(
                "PLAN_INVALID: reference '{reference}' does not resolve to a scalar input"
            )),
        }
    }

    pub async fn dispatch_node_execution_with_outputs(
        &self,
        node: &ExecutionNode,
        outputs: &HashMap<String, String>,
    ) -> Result<String, String> {
        let cap = node
            .required_capabilities
            .first()
            .cloned()
            .ok_or_else(|| "PLAN_INVALID: execution node has no capability".to_string())?;
        if cap == "bash" {
            return Err(
                "PLAN_INVALID: bash is not a valid application execution fallback".to_string(),
            );
        }

        let inputs = node
            .constraints
            .iter()
            .map(|(key, value)| {
                Self::resolve_reference(value, outputs).map(|resolved| (key.clone(), resolved))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        let target = match cap.as_str() {
            "application.search" => inputs.get("query"),
            "application.open" | "application.inspect" => inputs.get("application_id"),
            "keyboard.type" => inputs.get("text"),
            "window.focus" | "window.activate" | "window.close" => inputs.get("window_id"),
            "browser.navigate" => inputs.get("url"),
            "process.stop" => inputs.get("process_id"),
            "terminal.execute" => inputs.get("command"),
            _ => None,
        }
        .cloned()
        .unwrap_or_else(|| node.command.clone());
        let req = CapabilityRequest {
            request_id: format!("req-{}", uuid::Uuid::now_v7()),
            task_id: node.task_id.clone(),
            agent_id: "agent-kernel-core".to_string(),
            capability: cap,
            target,
            arguments: node.args.clone(),
            constraints: inputs,
            permission_context: PermissionContext {
                user_id: "user-default".to_string(),
                session_id: "sess-default".to_string(),
                granted_capabilities: std::collections::HashSet::from([
                    "bash".to_string(),
                    "win32.powershell".to_string(),
                    "package.install".to_string(),
                    "application.search".to_string(),
                    "application.open".to_string(),
                    "gui".to_string(),
                    "terminal.execute".to_string(),
                    "keyboard.type".to_string(),
                    "keyboard.hotkey".to_string(),
                    "window.list".to_string(),
                    "window.focus".to_string(),
                    "window.close".to_string(),
                    "browser.navigate".to_string(),
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

    pub async fn dispatch_node_execution(&self, node: &ExecutionNode) -> Result<String, String> {
        self.dispatch_node_execution_with_outputs(node, &HashMap::new())
            .await
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
        // VAL-001: LinuxRuntime.execute_command is a formatted string, not a
        // real process. bash must fail honestly.
        assert!(
            !res.success,
            "bash must not fake success: output={}",
            res.output
        );
        let err = res.error.unwrap_or_default();
        assert!(
            err.contains("CAPABILITY_UNAVAILABLE") || err.contains("unavailable"),
            "expected CAPABILITY_UNAVAILABLE, got {err}"
        );
        assert!(
            !res.assigned_runtime_id.contains("sim-backend"),
            "must not use sim-backend: {}",
            res.assigned_runtime_id
        );
    }

    #[test]
    fn plan_output_reference_resolves_application_id() {
        let outputs = HashMap::from([(
            "step-1".to_string(),
            r#"{"applications":[{"application_id":"app:dynamic-notepad"}]}"#.to_string(),
        )]);
        assert_eq!(
            CapabilityGateway::resolve_reference(
                "${step-1.applications[0].application_id}",
                &outputs
            )
            .unwrap(),
            "app:dynamic-notepad"
        );
    }

    #[test]
    fn missing_plan_output_is_an_honest_application_not_found_error() {
        let outputs = HashMap::from([("step-1".to_string(), r#"{"applications":[]}"#.to_string())]);
        let error = CapabilityGateway::resolve_reference(
            "${step-1.applications[0].application_id}",
            &outputs,
        )
        .expect_err("empty application search must not create an open request");
        assert!(error.contains("APPLICATION_NOT_FOUND"));
    }

    #[tokio::test]
    async fn planner_bash_node_fails_honestly() {
        let gateway = CapabilityGateway::new(Arc::new(RuntimeRegistry::new()));
        let node = ExecutionNode {
            node_id: "n1".into(),
            task_id: "task-bash".into(),
            name: "echo".into(),
            target_env: cognyx_planner::TargetEnvironment::NativeLinux,
            command: "echo".into(),
            args: vec!["hi".into()],
            depends_on: vec![],
            required_capabilities: vec!["bash".into()],
            constraints: HashMap::new(),
            state: cognyx_planner::NodeState::Pending,
            runtime_requirements: vec![],
            timeout_seconds: 5,
            retry_policy_max_retries: 0,
            env_vars: HashMap::new(),
            input_payload: String::new(),
            output_result: None,
        };
        let err = gateway
            .dispatch_node_execution(&node)
            .await
            .expect_err("bash planner node must fail");
        assert!(
            err.contains("PLAN_INVALID") || err.contains("CAPABILITY_UNAVAILABLE"),
            "got {err}"
        );
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
        if cfg!(target_os = "windows") {
            assert!(
                !result.assigned_runtime_id.to_lowercase().contains("linux"),
                "Windows host must not label apps linux: {}",
                result.assigned_runtime_id
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn application_search_runtime_id_identifies_windows_not_linux() {
        let gateway = CapabilityGateway::new(Arc::new(RuntimeRegistry::new()));
        let result = gateway
            .execute_capability(CapabilityRequest {
                request_id: "req-app-search".into(),
                task_id: "task-app-search".into(),
                agent_id: "agent-1".into(),
                capability: "application.search".into(),
                target: "notepad".into(),
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
        assert!(result.success, "{:?}", result.error);
        assert!(
            !result.assigned_runtime_id.to_lowercase().contains("linux"),
            "assigned_runtime_id must not contain linux: {}",
            result.assigned_runtime_id
        );
        assert!(
            result
                .assigned_runtime_id
                .to_lowercase()
                .contains("windows"),
            "assigned_runtime_id should identify Windows: {}",
            result.assigned_runtime_id
        );
        assert!(
            result.output.to_lowercase().contains("notepad")
                || result.output.contains("applications"),
            "search output: {}",
            result.output
        );
        assert!(
            !result.output.to_lowercase().contains("host-linux"),
            "application records must not carry host-linux identity: {}",
            result.output
        );
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
            err.contains("CAPABILITY_UNAVAILABLE")
                || err.contains("unavailable")
                || err.contains("Unavailable"),
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
        assert!(
            result.success,
            "screen.capture must succeed on Windows: {:?}",
            result.error
        );
        assert!(
            result.output.contains("image_b64"),
            "output must contain base64 image data"
        );
    }
}
