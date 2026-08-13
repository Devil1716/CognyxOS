use crate::model::*;
use crate::provider::*;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Boundary implemented only by OS/runtime adapters; it is not exposed to the Agent Kernel.
pub trait CapabilityAdapter: Send + Sync {
    fn adapter_id(&self) -> &'static str;
    fn runtime_kind(&self) -> CapabilityRuntime;
    fn native_operation(&self, capability: &str) -> Option<&'static str>;
}

macro_rules! adapter {
    ($name:ident, $kind:expr, $id:literal, { $($cap:literal => $native:literal),+ $(,)? }) => {
        pub struct $name;
        impl CapabilityAdapter for $name {
            fn adapter_id(&self) -> &'static str { $id }
            fn runtime_kind(&self) -> CapabilityRuntime { $kind }
            fn native_operation(&self, capability: &str) -> Option<&'static str> { match capability { $($cap => Some($native),)+ _ => None } }
        }
    };
}
adapter!(LinuxCapabilityAdapter, CapabilityRuntime::Linux, "linux", {
    "filesystem.read" => "POSIX read", "filesystem.write" => "POSIX write", "process.list" => "/proc", "process.start" => "execve", "terminal.execute" => "restricted shell", "application.list" => "desktop entries", "screen.capture" => "Wayland/X11", "keyboard.type" => "AT-SPI", "mouse.click" => "AT-SPI", "browser.navigate" => "browser accessibility"
});
adapter!(WindowsCapabilityAdapter, CapabilityRuntime::Windows, "windows", {
    "filesystem.read" => "Win32 file API", "filesystem.write" => "Win32 file API", "process.list" => "Toolhelp32", "process.start" => "CreateProcess", "terminal.execute" => "constrained PowerShell", "application.list" => "Windows App SDK", "screen.capture" => "Windows.Graphics.Capture", "keyboard.type" => "UI Automation", "mouse.click" => "UI Automation", "browser.navigate" => "UI Automation"
});
adapter!(MacOSCapabilityAdapter, CapabilityRuntime::MacOS, "macos", {
    "filesystem.read" => "Foundation FileManager", "filesystem.write" => "Foundation FileManager", "process.list" => "NSWorkspace", "process.start" => "NSWorkspace", "terminal.execute" => "restricted process", "application.list" => "NSWorkspace", "screen.capture" => "ScreenCaptureKit", "keyboard.type" => "Accessibility API", "mouse.click" => "Accessibility API", "browser.navigate" => "Accessibility API"
});
adapter!(ContainerCapabilityAdapter, CapabilityRuntime::Container, "container", {
    "filesystem.read" => "container filesystem", "filesystem.write" => "container filesystem", "process.list" => "container process API", "process.start" => "container exec", "terminal.execute" => "container exec"
});

pub struct AdapterProvider<A: CapabilityAdapter> {
    provider_id: String,
    runtime_id: String,
    adapter: A,
}
impl<A: CapabilityAdapter> AdapterProvider<A> {
    pub fn new(provider_id: impl Into<String>, runtime_id: impl Into<String>, adapter: A) -> Self {
        Self {
            provider_id: provider_id.into(),
            runtime_id: runtime_id.into(),
            adapter,
        }
    }
}
#[async_trait]
impl<A: CapabilityAdapter + 'static> CapabilityProvider for AdapterProvider<A> {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }
    fn runtime_id(&self) -> &str {
        &self.runtime_id
    }
    fn priority(&self) -> u8 {
        10
    }
    fn definitions(&self) -> Vec<CapabilityDefinition> {
        [
            "filesystem.read",
            "filesystem.write",
            "process.list",
            "process.start",
            "terminal.execute",
            "application.list",
            "screen.capture",
            "screen.read",
            "keyboard.type",
            "keyboard.press",
            "mouse.move",
            "mouse.click",
            "mouse.double_click",
            "mouse.scroll",
            "clipboard.read",
            "clipboard.write",
            "window.list",
            "window.focus",
            "browser.open",
            "browser.navigate",
            "browser.read",
            "browser.click",
            "browser.type",
        ]
        .into_iter()
        .filter(|c| self.adapter.native_operation(c).is_some())
        .map(|c| {
            let mut d = CapabilityDefinition::basic(
                c,
                format!("Universal {c} capability"),
                vec![self.adapter.runtime_kind()],
                if c.ends_with("read") || c.ends_with("list") {
                    Idempotency::ReadOnly
                } else {
                    Idempotency::NonIdempotent
                },
            );
            d.metadata.required_permissions.push(c.to_string());
            if matches!(c, "terminal.execute" | "filesystem.write") {
                d.metadata.security_level = SecurityLevel::Privileged;
                d.metadata.risk_level = RiskLevel::High;
            }
            d
        })
        .collect()
    }
    async fn execute(
        &self,
        context: CapabilityProviderContext,
    ) -> Result<CapabilityProviderResult, CapabilityError> {
        let native = self
            .adapter
            .native_operation(&context.request.capability_id)
            .ok_or_else(|| CapabilityError {
                code: CapabilityErrorCode::Unsupported,
                message: format!(
                    "{} cannot provide {}",
                    self.adapter.adapter_id(),
                    context.request.capability_id
                ),
                retryable: false,
            })?;
        if context.request.capability_id == "terminal.execute" {
            return Err(CapabilityError { code: CapabilityErrorCode::Unsupported, message: "Terminal execution requires a separately configured constrained terminal provider; this adapter never exposes host shell access.".into(), retryable: false });
        }
        Ok(CapabilityProviderResult {
            output: json!({"simulated": true, "adapter": self.adapter.adapter_id(), "native_operation": native, "input": context.request.input}),
            artifacts: vec![],
            side_effects: vec![],
            metadata: json!({"accessibility_priority": ["structured_api", "accessibility_tree", "application_api", "browser_dom", "ocr", "vision", "coordinates"]}),
        })
    }
}

/// Real, scoped local filesystem provider. Paths are always resolved under `root`.
pub struct LocalFilesystemProvider {
    provider_id: String,
    runtime_id: String,
    root: PathBuf,
}
impl LocalFilesystemProvider {
    pub fn new(
        provider_id: impl Into<String>,
        runtime_id: impl Into<String>,
        root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            runtime_id: runtime_id.into(),
            root: root.into(),
        }
    }
    fn path(&self, value: &Value) -> Result<PathBuf, CapabilityError> {
        let raw = value
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| err(CapabilityErrorCode::InvalidInput, "input.path is required"))?;
        let relative = Path::new(raw);
        if relative.is_absolute()
            || relative.components().any(|c| {
                matches!(
                    c,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(err(
                CapabilityErrorCode::PermissionDenied,
                "path must be a relative path within the provider root",
            ));
        }
        Ok(self.root.join(relative))
    }
}
fn err(code: CapabilityErrorCode, message: impl Into<String>) -> CapabilityError {
    CapabilityError {
        code,
        message: message.into(),
        retryable: false,
    }
}
fn io_error(e: std::io::Error) -> CapabilityError {
    let code = match e.kind() {
        std::io::ErrorKind::NotFound => CapabilityErrorCode::FileNotFound,
        std::io::ErrorKind::PermissionDenied => CapabilityErrorCode::PermissionDenied,
        _ => CapabilityErrorCode::Internal,
    };
    err(code, e.to_string())
}
#[async_trait]
impl CapabilityProvider for LocalFilesystemProvider {
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
        [
            "filesystem.read",
            "filesystem.write",
            "filesystem.copy",
            "filesystem.move",
            "filesystem.delete",
            "filesystem.create",
            "filesystem.list",
            "filesystem.metadata",
            "filesystem.permissions",
        ]
        .into_iter()
        .map(|c| {
            let mut d = CapabilityDefinition::basic(
                c,
                format!("Scoped universal {c}"),
                vec![
                    CapabilityRuntime::Linux,
                    CapabilityRuntime::Windows,
                    CapabilityRuntime::MacOS,
                    CapabilityRuntime::Container,
                ],
                if matches!(
                    c,
                    "filesystem.read"
                        | "filesystem.list"
                        | "filesystem.metadata"
                        | "filesystem.permissions"
                ) {
                    Idempotency::ReadOnly
                } else if c == "filesystem.delete" {
                    Idempotency::Destructive
                } else {
                    Idempotency::NonIdempotent
                },
            );
            d.metadata.required_permissions.push(c.into());
            d.metadata.security_level = if c == "filesystem.delete" {
                SecurityLevel::Critical
            } else {
                SecurityLevel::Sensitive
            };
            d.metadata.risk_level = if c == "filesystem.delete" {
                RiskLevel::Critical
            } else {
                RiskLevel::Medium
            };
            d
        })
        .collect()
    }
    async fn execute(
        &self,
        context: CapabilityProviderContext,
    ) -> Result<CapabilityProviderResult, CapabilityError> {
        let path = self.path(&context.request.input)?;
        let op = context.request.capability_id.as_str();
        let output = match op {
            "filesystem.read" => json!({"content": fs::read_to_string(path).map_err(io_error)?}),
            "filesystem.write" | "filesystem.create" => {
                let content = context
                    .request
                    .input
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        err(
                            CapabilityErrorCode::InvalidInput,
                            "input.content is required",
                        )
                    })?;
                fs::write(path, content).map_err(io_error)?;
                json!({"written": true})
            }
            "filesystem.delete" => {
                if path == self.root {
                    return Err(err(
                        CapabilityErrorCode::PermissionDenied,
                        "cannot delete the provider root",
                    ));
                }
                if path.is_dir() {
                    fs::remove_dir(path).map_err(io_error)?;
                } else {
                    fs::remove_file(path).map_err(io_error)?;
                }
                json!({"deleted": true})
            }
            "filesystem.copy" | "filesystem.move" => {
                let target_raw = context
                    .request
                    .input
                    .get("target")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        err(
                            CapabilityErrorCode::InvalidInput,
                            "input.target is required",
                        )
                    })?;
                let target = self.path(&json!({"path": target_raw}))?;
                if op == "filesystem.copy" {
                    fs::copy(&path, &target).map_err(io_error)?;
                } else {
                    fs::rename(&path, &target).map_err(io_error)?;
                }
                json!({"target": target_raw})
            }
            "filesystem.list" => {
                let entries = fs::read_dir(path)
                    .map_err(io_error)?
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect::<Vec<_>>();
                json!({"entries": entries})
            }
            "filesystem.metadata" | "filesystem.permissions" => {
                let metadata = fs::metadata(path).map_err(io_error)?;
                json!({"is_dir": metadata.is_dir(), "length": metadata.len(), "readonly": metadata.permissions().readonly()})
            }
            _ => {
                return Err(err(
                    CapabilityErrorCode::Unsupported,
                    format!("unsupported filesystem operation '{op}'"),
                ))
            }
        };
        Ok(CapabilityProviderResult {
            output,
            artifacts: vec![],
            side_effects: if op == "filesystem.read"
                || op == "filesystem.list"
                || op == "filesystem.metadata"
                || op == "filesystem.permissions"
            {
                vec![]
            } else {
                vec![op.into()]
            },
            metadata: json!({"root": self.root.display().to_string(), "real_io": true}),
        })
    }
}
