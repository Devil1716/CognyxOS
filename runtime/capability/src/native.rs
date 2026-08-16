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
        if let Some(application) = self
            .entries
            .read()
            .expect("application registry lock")
            .iter()
            .find(|application| application.application_id == id)
            .cloned()
        {
            return Some(application);
        }
        self.discover()
            .into_iter()
            .find(|app| app.application_id == id)
    }

    pub fn remember(&self, application: ApplicationRecord) {
        let mut entries = self.entries.write().expect("application registry lock");
        if !entries
            .iter()
            .any(|known| known.application_id == application.application_id)
        {
            entries.push(application);
        }
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
                let mut matched: Vec<ApplicationRecord> = records
                    .into_iter()
                    .filter(|a| {
                        a.name.to_ascii_lowercase().contains(&query)
                            || a.display_name.to_ascii_lowercase().contains(&query)
                    })
                    .collect();
                // Per-dir listing is truncated; exact PATH lookup still finds the named exe.
                if matched.is_empty() {
                    let exe_name = if cfg!(target_os = "windows") {
                        format!("{query}.exe")
                    } else {
                        query.clone()
                    };
                    for dir in env::split_paths(&env::var_os("PATH").unwrap_or_default()) {
                        let path = dir.join(&exe_name);
                        if path.is_file() {
                            matched.push(ApplicationRecord {
                                application_id: format!("app:{}", path.to_string_lossy()),
                                display_name: query.clone(),
                                name: query.clone(),
                                executable: path,
                                runtime_id: self.runtime_id.clone(),
                            });
                            break;
                        }
                    }
                }
                if matched.is_empty() {
                    return Err(err(
                        CapabilityErrorCode::ApplicationNotFound,
                        format!("no dynamically discovered application matches '{query}'"),
                    ));
                }
                for application in &matched {
                    self.registry.remember(application.clone());
                }
                json!({"applications": Self::records_json(matched)})
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
                #[cfg(target_os = "windows")]
                let gui_test = crate::gui_test::enabled();
                #[cfg(target_os = "windows")]
                let title_hint = if gui_test {
                    crate::gui_test::GOLDEN_TITLE_MARKER.to_string()
                } else if app.display_name.is_empty() {
                    app.name.clone()
                } else {
                    app.display_name.clone()
                };
                #[cfg(target_os = "windows")]
                let existing_windows = if gui_test {
                    visible_window_ids()
                } else {
                    titled_window_ids(&title_hint)
                };
                let mut extra_args: Vec<String> = context
                    .request
                    .input
                    .get("arguments")
                    .and_then(Value::as_array)
                    .map(|args| {
                        args.iter()
                            .filter_map(Value::as_str)
                            .map(ToOwned::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
                #[cfg(target_os = "windows")]
                let mut test_document = None;
                #[cfg(target_os = "windows")]
                let executable = if gui_test {
                    tracing::info!(
                        "COGNYX_GUI_TEST=1: restricting application.open to the CognyxOS test workspace"
                    );
                    let document = crate::gui_test::ensure_golden_document()
                        .map_err(|error| err(CapabilityErrorCode::InvalidInput, error))?;
                    let document_text = document.to_string_lossy().to_string();
                    if extra_args
                        .iter()
                        .any(|argument| crate::gui_test::is_protected_path(argument))
                    {
                        return Err(err(
                            CapabilityErrorCode::InvalidInput,
                            "TEST_TARGET_UNSAFE: spawn arguments include a protected path",
                        ));
                    }
                    if !extra_args
                        .iter()
                        .any(|argument| argument.eq_ignore_ascii_case(&document_text))
                    {
                        extra_args.push(document_text.clone());
                    }
                    test_document = Some(document_text);
                    if crate::gui_test::is_notepad_application(
                        &app.name,
                        &app.executable.to_string_lossy(),
                    ) {
                        crate::gui_test::isolated_notepad_executable()
                            .unwrap_or_else(|| app.executable.clone())
                    } else {
                        app.executable.clone()
                    }
                } else {
                    app.executable.clone()
                };
                #[cfg(not(target_os = "windows"))]
                let executable = app.executable.clone();
                let mut command = std::process::Command::new(&executable);
                for argument in &extra_args {
                    command.arg(argument);
                }
                let child = command
                    .spawn()
                    .map_err(|e| err(CapabilityErrorCode::Internal, e.to_string()))?;
                let process_id = child.id();
                #[cfg(target_os = "windows")]
                let focused =
                    wait_and_focus_process_window(process_id, &title_hint, existing_windows).await;
                #[cfg(target_os = "windows")]
                if let Some((window_id, title)) = &focused {
                    if crate::gui_test::is_protected_title(title) {
                        return Err(err(
                            CapabilityErrorCode::InvalidInput,
                            format!("TEST_TARGET_UNSAFE: window {window_id} title '{title}' is protected"),
                        ));
                    }
                    if gui_test && !crate::gui_test::is_test_owned_title(title) {
                        return Err(err(
                            CapabilityErrorCode::InvalidInput,
                            format!("TEST_TARGET_UNSAFE: window {window_id} title '{title}' is not the golden test document"),
                        ));
                    }
                }
                #[cfg(target_os = "windows")]
                if gui_test && focused.is_none() {
                    return Err(err(
                        CapabilityErrorCode::InvalidInput,
                        "TEST_TARGET_UNSAFE: could not uniquely identify a test-owned window after launch",
                    ));
                }
                #[cfg(not(target_os = "windows"))]
                let focused: Option<(String, String)> = None;
                let mut output = json!({"application_id": app.application_id, "process_id": process_id, "status": "started"});
                if let Some((window_id, title)) = focused {
                    output["window_id"] = json!(window_id);
                    output["window_title"] = json!(title);
                    output["focused"] = json!(true);
                    output["test_owned"] = json!(crate::gui_test::is_test_owned_title(&title));
                }
                #[cfg(target_os = "windows")]
                if let Some(document) = test_document {
                    output["test_document"] = json!(document);
                }
                output
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

#[cfg(target_os = "windows")]
struct WindowHunt {
    pid: u32,
    needle: String,
    skip: Vec<usize>,
    hwnd: Option<windows::Win32::Foundation::HWND>,
    title: String,
    prefer_pid: bool,
}

#[cfg(target_os = "windows")]
async fn wait_and_focus_process_window(
    pid: u32,
    title_hint: &str,
    existing: Vec<usize>,
) -> Option<(String, String)> {
    let needle = title_hint.to_ascii_lowercase();
    for _ in 0..16 {
        let needle = needle.clone();
        let skip = existing.clone();
        if let Some(found) =
            tokio::task::spawn_blocking(move || find_and_focus_window(pid, needle, skip, true))
                .await
                .ok()
                .flatten()
        {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            return Some(found);
        }
        let needle = title_hint.to_ascii_lowercase();
        let skip = existing.clone();
        if let Some(found) =
            tokio::task::spawn_blocking(move || find_and_focus_window(pid, needle, skip, false))
                .await
                .ok()
                .flatten()
        {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            return Some(found);
        }
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    }
    None
}

#[cfg(target_os = "windows")]
fn titled_window_ids(title_hint: &str) -> Vec<usize> {
    let needle = title_hint.to_ascii_lowercase();
    let mut ids = Vec::new();
    collect_titled_windows(&needle, &mut ids);
    ids
}

#[cfg(target_os = "windows")]
fn visible_window_ids() -> Vec<usize> {
    let mut ids = Vec::new();
    collect_titled_windows("", &mut ids);
    ids
}

#[cfg(target_os = "windows")]
fn collect_titled_windows(needle: &str, ids: &mut Vec<usize>) {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, IsWindowVisible};

    struct Collect {
        needle: String,
        ids: *mut Vec<usize>,
    }
    let mut collect = Collect {
        needle: needle.to_string(),
        ids,
    };
    unsafe {
        unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let collect = &mut *(lparam.0 as *mut Collect);
            if IsWindowVisible(hwnd).as_bool() {
                let mut title = [0u16; 512];
                let len = GetWindowTextW(hwnd, &mut title);
                if len > 0 {
                    let title =
                        String::from_utf16_lossy(&title[..len as usize]).to_ascii_lowercase();
                    if collect.needle.is_empty() || title.contains(&collect.needle) {
                        (*collect.ids).push(hwnd.0 as usize);
                    }
                }
            }
            BOOL(1)
        }
        let _ = EnumWindows(
            Some(callback),
            LPARAM(&mut collect as *mut Collect as isize),
        );
    }
}

#[cfg(target_os = "windows")]
fn find_and_focus_window(
    pid: u32,
    needle: String,
    skip: Vec<usize>,
    prefer_pid: bool,
) -> Option<(String, String)> {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    };

    let mut hunt = WindowHunt {
        pid,
        needle,
        skip,
        hwnd: None,
        title: String::new(),
        prefer_pid,
    };
    unsafe {
        unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let hunt = &mut *(lparam.0 as *mut WindowHunt);
            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1);
            }
            let mut window_pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
            let mut class_name = [0u16; 256];
            let class_len = GetClassNameW(hwnd, &mut class_name);
            let class_name = if class_len > 0 {
                String::from_utf16_lossy(&class_name[..class_len as usize])
            } else {
                String::new()
            };
            if class_name.contains("Popup")
                || class_name.contains("IME")
                || class_name.eq_ignore_ascii_case("tooltips_class32")
            {
                return BOOL(1);
            }
            let mut title = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title);
            let title = if len > 0 {
                String::from_utf16_lossy(&title[..len as usize])
            } else {
                String::new()
            };
            let title_l = title.to_ascii_lowercase();
            let class_l = class_name.to_ascii_lowercase();
            let title_matches = !hunt.needle.is_empty() && title_l.contains(&hunt.needle);
            let class_matches = class_l == hunt.needle || class_l == "notepad";
            let test_owned = crate::gui_test::is_test_owned_title(&title);
            if crate::gui_test::is_protected_title(&title) {
                return BOOL(1);
            }
            if hunt.skip.contains(&(hwnd.0 as usize)) {
                return BOOL(1);
            }
            if crate::gui_test::enabled() && !test_owned {
                return BOOL(1);
            }
            if hunt.prefer_pid {
                if window_pid == hunt.pid && title_matches {
                    hunt.hwnd = Some(hwnd);
                    hunt.title = title;
                    return BOOL(0);
                }
                return BOOL(1);
            }
            if title_matches && (window_pid == hunt.pid || class_matches) {
                hunt.hwnd = Some(hwnd);
                hunt.title = title;
                return BOOL(0);
            }
            BOOL(1)
        }
        let _ = EnumWindows(
            Some(callback),
            LPARAM(&mut hunt as *mut WindowHunt as isize),
        );
        let hwnd = hunt.hwnd?;
        crate::windows_providers::force_foreground_window(hwnd);
        Some((format!("hwnd:{}", hwnd.0 as usize), hunt.title))
    }
}
