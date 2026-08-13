//! Native host providers. They never invoke a shell: terminal execution is an
//! allowlisted executable plus argument vector, and filesystem paths remain scoped.
use crate::model::*;
use crate::provider::*;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::env;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;

fn err(code: CapabilityErrorCode, message: impl Into<String>) -> CapabilityError {
    CapabilityError {
        code,
        message: message.into(),
        retryable: false,
    }
}
fn definition(id: &str, description: &str, idempotency: Idempotency) -> CapabilityDefinition {
    let runtime = match env::consts::OS {
        "windows" => CapabilityRuntime::Windows,
        "macos" => CapabilityRuntime::MacOS,
        _ => CapabilityRuntime::Linux,
    };
    let mut d = CapabilityDefinition::basic(id, description, vec![runtime], idempotency);
    d.metadata.required_permissions.push(id.into());
    d.metadata.audit_policy = AuditPolicy::Full;
    d
}
fn process_id(input: &Value) -> Result<u32, CapabilityError> {
    input
        .get("process_id")
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| {
            err(
                CapabilityErrorCode::InvalidInput,
                "input.process_id must be a positive integer",
            )
        })
}

#[derive(Clone, Debug)]
pub struct ApplicationRecord {
    pub application_id: String,
    pub name: String,
    pub display_name: String,
    pub executable: PathBuf,
    pub runtime_id: String,
}
pub struct ApplicationRegistry {
    runtime_id: String,
    entries: std::sync::RwLock<Vec<ApplicationRecord>>,
}
impl ApplicationRegistry {
    pub fn new(runtime_id: impl Into<String>) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            entries: std::sync::RwLock::new(vec![]),
        }
    }
    pub fn discover(&self) -> Vec<ApplicationRecord> {
        let mut found = Vec::new();
        let mut seen = HashSet::new();
        for dir in env::split_paths(&env::var_os("PATH").unwrap_or_default()) {
            let Ok(items) = std::fs::read_dir(dir) else {
                continue;
            };
            for item in items.flatten().take(256) {
                let path = item.path();
                if !path.is_file() {
                    continue;
                }
                let executable = if cfg!(target_os = "windows") {
                    path.extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
                } else {
                    true
                };
                if !executable {
                    continue;
                }
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if name.is_empty() || !seen.insert(path.clone()) {
                    continue;
                }
                found.push(ApplicationRecord {
                    application_id: format!("app:{}", path.to_string_lossy()),
                    display_name: name.clone(),
                    name,
                    executable: path,
                    runtime_id: self.runtime_id.clone(),
                });
            }
        }
        *self.entries.write().expect("application registry lock") = found.clone();
        found
    }
    pub fn find(&self, id: &str) -> Option<ApplicationRecord> {
        self.discover()
            .into_iter()
            .find(|app| app.application_id == id)
    }
}

pub struct NativeApplicationProvider {
    provider_id: String,
    runtime_id: String,
    registry: ApplicationRegistry,
}
impl NativeApplicationProvider {
    pub fn new(provider_id: impl Into<String>, runtime_id: impl Into<String>) -> Self {
        let runtime_id = runtime_id.into();
        Self {
            provider_id: provider_id.into(),
            registry: ApplicationRegistry::new(runtime_id.clone()),
            runtime_id,
        }
    }
    fn records_json(records: Vec<ApplicationRecord>) -> Value {
        json!(records.into_iter().map(|a| json!({"application_id": a.application_id, "name": a.name, "display_name": a.display_name, "executable": a.executable, "runtime_id": a.runtime_id, "capabilities": ["application.open"], "status": "discovered"})).collect::<Vec<_>>())
    }
}
#[async_trait]
impl CapabilityProvider for NativeApplicationProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }
    fn runtime_id(&self) -> &str {
        &self.runtime_id
    }
    fn priority(&self) -> u8 {
        5
    }
    fn definitions(&self) -> Vec<CapabilityDefinition> {
        vec![
            definition(
                "application.list",
                "Dynamically discover executable applications on the native runtime",
                Idempotency::ReadOnly,
            ),
            definition(
                "application.search",
                "Search dynamically discovered applications",
                Idempotency::ReadOnly,
            ),
            definition(
                "application.inspect",
                "Inspect a dynamically discovered application",
                Idempotency::ReadOnly,
            ),
            definition(
                "application.open",
                "Launch a dynamically discovered application",
                Idempotency::NonIdempotent,
            ),
        ]
    }
    async fn execute(
        &self,
        context: CapabilityProviderContext,
    ) -> Result<CapabilityProviderResult, CapabilityError> {
        let op = context.request.capability_id.as_str();
        let records = self.registry.discover();
        let output = match op {
            "application.list" => json!({"applications": Self::records_json(records)}),
            "application.search" => {
                let query = context
                    .request
                    .input
                    .get("query")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        err(CapabilityErrorCode::InvalidInput, "input.query is required")
                    })?
                    .to_ascii_lowercase();
                json!({"applications": Self::records_json(records.into_iter().filter(|a| a.name.to_ascii_lowercase().contains(&query) || a.display_name.to_ascii_lowercase().contains(&query)).collect())})
            }
            "application.inspect" => {
                let id = context
                    .request
                    .input
                    .get("application_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        err(
                            CapabilityErrorCode::InvalidInput,
                            "input.application_id is required",
                        )
                    })?;
                let app = self.registry.find(id).ok_or_else(|| {
                    err(
                        CapabilityErrorCode::ApplicationNotFound,
                        "application is not currently discoverable",
                    )
                })?;
                json!({"application_id": app.application_id, "name": app.name, "display_name": app.display_name, "executable": app.executable, "runtime_id": app.runtime_id, "status": "discovered"})
            }
            "application.open" => {
                let id = context
                    .request
                    .input
                    .get("application_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        err(
                            CapabilityErrorCode::InvalidInput,
                            "input.application_id is required",
                        )
                    })?;
                let app = self.registry.find(id).ok_or_else(|| {
                    err(
                        CapabilityErrorCode::ApplicationNotFound,
                        "application must be dynamically discovered before it can be opened",
                    )
                })?;
                let child = std::process::Command::new(&app.executable)
                    .spawn()
                    .map_err(|e| err(CapabilityErrorCode::Internal, e.to_string()))?;
                json!({"application_id": app.application_id, "process_id": child.id(), "status": "started"})
            }
            _ => {
                return Err(err(
                    CapabilityErrorCode::Unsupported,
                    "unsupported application operation",
                ))
            }
        };
        Ok(CapabilityProviderResult {
            output,
            artifacts: vec![],
            side_effects: if op == "application.open" {
                vec!["application.started".into()]
            } else {
                vec![]
            },
            metadata: json!({"native": true, "host_os": env::consts::OS, "discovery": "PATH executable metadata", "cache_authoritative": false}),
        })
    }
}

pub struct NativeProcessProvider {
    provider_id: String,
    runtime_id: String,
}
impl NativeProcessProvider {
    pub fn new(provider_id: impl Into<String>, runtime_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            runtime_id: runtime_id.into(),
        }
    }
    async fn list_native() -> Result<Value, CapabilityError> {
        #[cfg(target_os = "windows")]
        let output = tokio::process::Command::new("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;
        #[cfg(target_os = "linux")]
        let output = tokio::process::Command::new("ps")
            .args(["-eo", "pid=,comm="])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;
        #[cfg(target_os = "macos")]
        let output = tokio::process::Command::new("ps")
            .args(["-axo", "pid=,comm="])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;
        let output = output.map_err(|e| err(CapabilityErrorCode::Internal, e.to_string()))?;
        if !output.status.success() {
            return Err(err(
                CapabilityErrorCode::Internal,
                String::from_utf8_lossy(&output.stderr),
            ));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let processes = text.lines().take(1_000).filter_map(|line| {
            #[cfg(target_os = "windows")]
            { let fields: Vec<_> = line.trim_matches('"').split("\",\"").collect(); let id = fields.get(1)?.parse::<u32>().ok()?; Some(json!({"process_id": id, "name": fields.first()?.to_string(), "state": "running"})) }
            #[cfg(not(target_os = "windows"))]
            { let mut parts = line.split_whitespace(); let id = parts.next()?.parse::<u32>().ok()?; Some(json!({"process_id": id, "name": parts.collect::<Vec<_>>().join(" "), "state": "running"})) }
        }).collect::<Vec<_>>();
        Ok(json!({"processes": processes}))
    }
}
#[async_trait]
impl CapabilityProvider for NativeProcessProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }
    fn runtime_id(&self) -> &str {
        &self.runtime_id
    }
    fn priority(&self) -> u8 {
        5
    }
    fn definitions(&self) -> Vec<CapabilityDefinition> {
        vec![
            definition(
                "process.list",
                "List native runtime processes",
                Idempotency::ReadOnly,
            ),
            definition(
                "process.inspect",
                "Inspect a native runtime process",
                Idempotency::ReadOnly,
            ),
            definition(
                "process.stop",
                "Stop a native runtime process",
                Idempotency::Destructive,
            ),
        ]
    }
    async fn execute(
        &self,
        context: CapabilityProviderContext,
    ) -> Result<CapabilityProviderResult, CapabilityError> {
        let op = context.request.capability_id.as_str();
        let output = match op {
            "process.list" => Self::list_native().await?,
            "process.inspect" => {
                let id = process_id(&context.request.input)?;
                let processes = Self::list_native().await?;
                let process = processes["processes"]
                    .as_array()
                    .and_then(|p| {
                        p.iter()
                            .find(|p| p["process_id"].as_u64() == Some(id as u64))
                    })
                    .cloned()
                    .ok_or_else(|| {
                        err(
                            CapabilityErrorCode::ApplicationNotFound,
                            "process not found",
                        )
                    })?;
                json!({"process": process})
            }
            "process.stop" => {
                let id = process_id(&context.request.input)?;
                #[cfg(target_os = "windows")]
                let result = tokio::process::Command::new("taskkill")
                    .args(["/PID", &id.to_string(), "/T"])
                    .output()
                    .await;
                #[cfg(not(target_os = "windows"))]
                let result = tokio::process::Command::new("kill")
                    .args(["-TERM", &id.to_string()])
                    .output()
                    .await;
                let result =
                    result.map_err(|e| err(CapabilityErrorCode::Internal, e.to_string()))?;
                if !result.status.success() {
                    return Err(err(
                        CapabilityErrorCode::Internal,
                        String::from_utf8_lossy(&result.stderr),
                    ));
                }
                json!({"process_id": id, "stopped": true})
            }
            _ => {
                return Err(err(
                    CapabilityErrorCode::Unsupported,
                    "unsupported process operation",
                ))
            }
        };
        Ok(CapabilityProviderResult {
            output,
            artifacts: vec![],
            side_effects: if op == "process.stop" {
                vec!["process.stopped".into()]
            } else {
                vec![]
            },
            metadata: json!({"native": true, "host_os": env::consts::OS, "shell": false}),
        })
    }
}

pub struct NativeTerminalProvider {
    provider_id: String,
    runtime_id: String,
    root: PathBuf,
    allowed_executables: HashSet<String>,
}
impl NativeTerminalProvider {
    pub fn new(
        provider_id: impl Into<String>,
        runtime_id: impl Into<String>,
        root: impl Into<PathBuf>,
        allowed_executables: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            runtime_id: runtime_id.into(),
            root: root.into(),
            allowed_executables: allowed_executables
                .into_iter()
                .map(|v| v.to_ascii_lowercase())
                .collect(),
        }
    }
    fn working_directory(&self, input: &Value) -> Result<PathBuf, CapabilityError> {
        let raw = input
            .get("working_directory")
            .and_then(Value::as_str)
            .unwrap_or(".");
        let path = Path::new(raw);
        if path.is_absolute()
            || path.components().any(|c| {
                matches!(
                    c,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(err(
                CapabilityErrorCode::PermissionDenied,
                "working_directory must stay within the configured root",
            ));
        }
        Ok(self.root.join(path))
    }
}
#[async_trait]
impl CapabilityProvider for NativeTerminalProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }
    fn runtime_id(&self) -> &str {
        &self.runtime_id
    }
    fn priority(&self) -> u8 {
        1
    }
    fn definitions(&self) -> Vec<CapabilityDefinition> {
        let mut d = definition(
            "terminal.execute",
            "Execute an allowlisted native executable without a shell",
            Idempotency::NonIdempotent,
        );
        d.metadata.security_level = SecurityLevel::Critical;
        d.metadata.risk_level = RiskLevel::Critical;
        vec![d]
    }
    async fn execute(
        &self,
        context: CapabilityProviderContext,
    ) -> Result<CapabilityProviderResult, CapabilityError> {
        let input = &context.request.input;
        let executable = input
            .get("executable")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                err(
                    CapabilityErrorCode::InvalidInput,
                    "input.executable is required",
                )
            })?;
        if !self
            .allowed_executables
            .contains(&executable.to_ascii_lowercase())
        {
            return Err(err(
                CapabilityErrorCode::PermissionDenied,
                "executable is not in this provider's allowlist",
            ));
        }
        let args = input
            .get("args")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                err(
                    CapabilityErrorCode::InvalidInput,
                    "input.args must be an array",
                )
            })?
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "args must be strings"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let cwd = self.working_directory(input)?;
        let mut command = tokio::process::Command::new(executable);
        command
            .args(&args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(vars) = input.get("environment").and_then(Value::as_object) {
            command.env_clear();
            for (key, value) in vars {
                if let Some(value) = value.as_str() {
                    command.env(key, value);
                }
            }
        }
        let child = command
            .spawn()
            .map_err(|e| err(CapabilityErrorCode::Internal, e.to_string()))?;
        let pid = child.id();
        let timeout_ms = context.request.timeout_ms.unwrap_or(30_000);
        let output = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            child.wait_with_output(),
        )
        .await
        .map_err(|_| err(CapabilityErrorCode::Timeout, "terminal process timed out"))?
        .map_err(|e| err(CapabilityErrorCode::Internal, e.to_string()))?;
        Ok(CapabilityProviderResult {
            output: json!({"stdout": String::from_utf8_lossy(&output.stdout), "stderr": String::from_utf8_lossy(&output.stderr), "exit_code": output.status.code(), "process_id": pid}),
            artifacts: vec![],
            side_effects: vec!["terminal.executed".into()],
            metadata: json!({"native": true, "shell": false, "allowlisted": true}),
        })
    }
}

#[cfg(target_os = "windows")]
pub struct WindowsClipboardProvider {
    provider_id: String,
    runtime_id: String,
}
#[cfg(target_os = "windows")]
impl WindowsClipboardProvider {
    pub fn new(provider_id: impl Into<String>, runtime_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            runtime_id: runtime_id.into(),
        }
    }
}
#[cfg(target_os = "windows")]
#[async_trait]
impl CapabilityProvider for WindowsClipboardProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }
    fn runtime_id(&self) -> &str {
        &self.runtime_id
    }
    fn priority(&self) -> u8 {
        1
    }
    fn definitions(&self) -> Vec<CapabilityDefinition> {
        vec![
            definition(
                "clipboard.read",
                "Read the Windows clipboard",
                Idempotency::ReadOnly,
            ),
            definition(
                "clipboard.write",
                "Write the Windows clipboard",
                Idempotency::NonIdempotent,
            ),
        ]
    }
    async fn execute(
        &self,
        context: CapabilityProviderContext,
    ) -> Result<CapabilityProviderResult, CapabilityError> {
        let op = context.request.capability_id.as_str();
        let output = match op {
            "clipboard.read" => {
                let result = tokio::process::Command::new("powershell.exe")
                    .args([
                        "-NoProfile",
                        "-NonInteractive",
                        "-Command",
                        "Get-Clipboard -Raw",
                    ])
                    .output()
                    .await
                    .map_err(|e| err(CapabilityErrorCode::Internal, e.to_string()))?;
                if !result.status.success() {
                    return Err(err(
                        CapabilityErrorCode::Internal,
                        String::from_utf8_lossy(&result.stderr),
                    ));
                }
                json!({"text": String::from_utf8_lossy(&result.stdout)})
            }
            "clipboard.write" => {
                let text = context
                    .request
                    .input
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        err(CapabilityErrorCode::InvalidInput, "input.text is required")
                    })?;
                let encoded = {
                    use base64::Engine;
                    base64::engine::general_purpose::STANDARD.encode(text.as_bytes())
                };
                let script = format!("[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{encoded}')) | Set-Clipboard");
                let result = tokio::process::Command::new("powershell.exe")
                    .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                    .output()
                    .await
                    .map_err(|e| err(CapabilityErrorCode::Internal, e.to_string()))?;
                if !result.status.success() {
                    return Err(err(
                        CapabilityErrorCode::Internal,
                        String::from_utf8_lossy(&result.stderr),
                    ));
                }
                json!({"written": true})
            }
            _ => {
                return Err(err(
                    CapabilityErrorCode::Unsupported,
                    "unsupported clipboard operation",
                ))
            }
        };
        Ok(CapabilityProviderResult {
            output,
            artifacts: vec![],
            side_effects: if op == "clipboard.write" {
                vec!["clipboard.updated".into()]
            } else {
                vec![]
            },
            metadata: json!({"native": true, "api": "Windows PowerShell clipboard cmdlets"}),
        })
    }
}

#[derive(Clone, Debug)]
pub struct VisionRequest {
    pub image_artifact: String,
    pub query: Option<String>,
}
#[derive(Clone, Debug)]
pub struct VisionElement {
    pub label: Option<String>,
    pub confidence: f32,
    pub bounds: Option<(u32, u32, u32, u32)>,
}
#[derive(Clone, Debug)]
pub struct VisionResult {
    pub elements: Vec<VisionElement>,
    pub text: Option<String>,
    pub provider_id: String,
}

/// Stable browser abstraction. No browser backend is registered until it can
/// execute these operations against an actual browser session.
#[derive(Clone, Debug)]
pub struct BrowserSession {
    pub session_id: String,
    pub runtime_id: String,
    pub browser_id: String,
}
#[derive(Clone, Debug)]
pub struct Browser {
    pub browser_id: String,
    pub name: String,
    pub executable: Option<PathBuf>,
    pub capabilities: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct BrowserTab {
    pub tab_id: String,
    pub url: String,
    pub title: Option<String>,
}
#[derive(Clone, Debug)]
pub struct BrowserElement {
    pub element_id: String,
    pub role: Option<String>,
    pub name: Option<String>,
    pub text: Option<String>,
}
#[derive(Clone, Debug)]
pub struct BrowserResult {
    pub session_id: String,
    pub output: Value,
    pub artifacts: Vec<String>,
}
#[async_trait]
pub trait BrowserProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn discover(&self) -> Vec<Browser>;
    async fn execute_browser(
        &self,
        session: &BrowserSession,
        capability: &str,
        input: &Value,
    ) -> Result<BrowserResult, CapabilityError>;
}
