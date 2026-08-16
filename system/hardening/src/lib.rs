use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum HardeningError {
    #[error("invalid config: {0}")]
    Config(String),
    #[error("secret leaked into {0}")]
    SecretLeak(String),
    #[error("update failed: {0}")]
    Update(String),
    #[error("backup: {0}")]
    Backup(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Environment {
    Development,
    Testing,
    Staging,
    Production,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemConfig {
    pub environment: Environment,
    pub release_channel: ReleaseChannel,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReleaseChannel {
    Development,
    Nightly,
    Beta,
    Stable,
}

impl SystemConfig {
    pub fn validate(&self) -> Result<(), HardeningError> {
        if self.version.is_empty() {
            return Err(HardeningError::Config("version required".into()));
        }
        if self.environment == Environment::Production
            && matches!(
                self.release_channel,
                ReleaseChannel::Development | ReleaseChannel::Nightly
            )
        {
            return Err(HardeningError::Config(
                "production cannot use development/nightly channel".into(),
            ));
        }
        Ok(())
    }
}

pub struct SecretStore {
    secrets: DashMap<String, Vec<u8>>,
}

impl Default for SecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore {
    pub fn new() -> Self {
        Self {
            secrets: DashMap::new(),
        }
    }

    pub fn put(&self, name: &str, value: &[u8]) {
        self.secrets.insert(name.into(), value.to_vec());
    }

    pub fn get(&self, name: &str) -> Option<Vec<u8>> {
        self.secrets.get(name).map(|v| v.clone())
    }

    pub fn redact(&self, text: &str) -> Result<String, HardeningError> {
        let mut out = text.to_string();
        for entry in self.secrets.iter() {
            let needle = String::from_utf8_lossy(entry.value());
            if !needle.is_empty() && out.contains(needle.as_ref()) {
                return Err(HardeningError::SecretLeak("log/history".into()));
            }
            out = out.replace(needle.as_ref(), "[REDACTED]");
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Backup {
    pub id: String,
    pub includes: Vec<String>,
    pub checksum: String,
}

pub struct BackupEngine {
    pub secrets: SecretStore,
    snapshots: Mutex<HashMap<String, Backup>>,
}

impl BackupEngine {
    pub fn new(secrets: SecretStore) -> Self {
        Self {
            secrets,
            snapshots: Mutex::new(HashMap::new()),
        }
    }

    pub fn backup(&self, payload: &str, includes: Vec<String>) -> Result<Backup, HardeningError> {
        if self.secrets.redact(payload).is_err() {
            return Err(HardeningError::Backup(
                "refusing to backup plaintext secrets".into(),
            ));
        }
        let b = Backup {
            id: format!("bak-{}", uuid::Uuid::now_v7()),
            includes,
            checksum: format!("{:x}", Sha256::digest(payload.as_bytes())),
        };
        self.snapshots
            .lock()
            .unwrap()
            .insert(b.id.clone(), b.clone());
        Ok(b)
    }

    pub fn restore(&self, id: &str) -> Result<Backup, HardeningError> {
        self.snapshots
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| HardeningError::Backup("missing snapshot".into()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HealthStatus {
    Available,
    Healthy,
    Degraded,
    Unavailable,
    NotVerified,
    PermissionDenied,
    NotInstalled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub component: String,
    pub ok: bool,
    pub detail: String,
    pub status: HealthStatus,
}

impl Diagnostic {
    pub fn new(
        component: impl Into<String>,
        ok: bool,
        status: HealthStatus,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            component: component.into(),
            ok,
            detail: detail.into(),
            status,
        }
    }
}

pub struct Doctor;

impl Doctor {
    fn probe_cmd(program: &str, args: &[&str]) -> Result<std::process::Output, String> {
        std::process::Command::new(program)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .map_err(|e| e.to_string())
    }

    fn docker_diag() -> Diagnostic {
        match Self::probe_cmd("docker", &["info"]) {
            Err(e) if e.to_lowercase().contains("cannot find") || e.contains("os error 2") => {
                Diagnostic::new(
                    "virtualization.docker",
                    false,
                    HealthStatus::NotInstalled,
                    format!("NOT_INSTALLED: docker binary missing ({e})"),
                )
            }
            Err(e) => Diagnostic::new(
                "virtualization.docker",
                false,
                HealthStatus::Unavailable,
                format!("UNAVAILABLE: docker probe failed ({e})"),
            ),
            Ok(out) if out.status.success() => Diagnostic::new(
                "virtualization.docker",
                true,
                HealthStatus::Healthy,
                "docker daemon responded to docker info",
            ),
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr);
                Diagnostic::new(
                    "virtualization.docker",
                    false,
                    HealthStatus::Unavailable,
                    format!("UNAVAILABLE: docker daemon down: {err}"),
                )
            }
        }
    }

    fn hyperv_diag() -> Diagnostic {
        if std::env::consts::OS != "windows" {
            return Diagnostic::new(
                "virtualization.hyperv",
                false,
                HealthStatus::NotInstalled,
                "Hyper-V is a Windows feature; not present on this OS",
            );
        }
        let cmd = "Get-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V | Select-Object -ExpandProperty State";
        match Self::probe_cmd(
            "powershell",
            &["-NoProfile", "-NonInteractive", "-Command", cmd],
        ) {
            Err(e) => Diagnostic::new(
                "virtualization.hyperv",
                false,
                HealthStatus::NotVerified,
                format!("NOT_VERIFIED: host probe failed ({e})"),
            ),
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{stdout}\n{stderr}").to_lowercase();
                if combined.contains("access is denied")
                    || combined.contains("elevation")
                    || combined.contains("administrator")
                    || (!out.status.success()
                        && (combined.contains("unauthorized") || combined.contains("permission")))
                {
                    Diagnostic::new(
                        "virtualization.hyperv",
                        false,
                        HealthStatus::PermissionDenied,
                        format!("PERMISSION_DENIED: feature query elevation failure: {stderr}"),
                    )
                } else if !out.status.success() {
                    Diagnostic::new(
                        "virtualization.hyperv",
                        false,
                        HealthStatus::NotVerified,
                        format!("NOT_VERIFIED: feature query failed: {stderr}"),
                    )
                } else if stdout.to_ascii_lowercase().contains("enabled") {
                    Diagnostic::new(
                        "virtualization.hyperv",
                        true,
                        HealthStatus::Available,
                        format!("Hyper-V state: {}", stdout.trim()),
                    )
                } else {
                    Diagnostic::new(
                        "virtualization.hyperv",
                        false,
                        HealthStatus::Unavailable,
                        format!("UNAVAILABLE: Hyper-V state: {}", stdout.trim()),
                    )
                }
            }
        }
    }

    fn playwright_diag() -> Diagnostic {
        Diagnostic::new(
            "browser",
            false,
            HealthStatus::Unavailable,
            "BROWSER=UNAVAILABLE",
        )
    }

    pub fn run() -> Vec<Diagnostic> {
        let docker = Self::docker_diag();
        let hyperv = Self::hyperv_diag();
        let virt_ok = docker.ok || hyperv.ok;
        let virt_status = if virt_ok {
            HealthStatus::Healthy
        } else if hyperv.status == HealthStatus::PermissionDenied {
            HealthStatus::PermissionDenied
        } else if docker.status == HealthStatus::Unavailable
            || hyperv.status == HealthStatus::Unavailable
        {
            HealthStatus::Unavailable
        } else {
            HealthStatus::NotVerified
        };
        let virt_detail = format!(
            "docker={} hyperv={}; never ok unless a backend was actually probed healthy",
            docker.detail, hyperv.detail
        );
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(0);
        let cpu_ok = cpus > 0;
        let ws_root = if cfg!(windows) {
            std::path::PathBuf::from(r"C:\CognyxOSTestWorkspace")
        } else {
            std::env::temp_dir().join("CognyxOSTestWorkspace")
        };
        let ws = match std::fs::create_dir_all(&ws_root) {
            Ok(()) => Diagnostic::new(
                "workspace",
                true,
                HealthStatus::Available,
                format!("dedicated root {} created/present", ws_root.display()),
            ),
            Err(e) => Diagnostic::new(
                "workspace",
                false,
                HealthStatus::Unavailable,
                format!("UNAVAILABLE: cannot create {}: {e}", ws_root.display()),
            ),
        };
        vec![
            Diagnostic::new("host", true, HealthStatus::Available, format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)),
            Diagnostic::new("cpu", cpu_ok, if cpu_ok { HealthStatus::Available } else { HealthStatus::NotVerified }, format!("{cpus} logical CPUs via available_parallelism")),
            Diagnostic::new("virtualization", virt_ok, virt_status, virt_detail),
            docker,
            hyperv,
            ws,
            Diagnostic::new("memory", true, HealthStatus::Available, "IN-PROCESS working + long-term modules; optional JSON persist under dedicated workspace/memory"),
            Diagnostic::new("plugins", true, HealthStatus::Available, "IN-PROCESS, WASM NOT IMPLEMENTED"),
            Diagnostic::new("workers", true, HealthStatus::Available, "local registry only; WAN NOT VERIFIED"),
            Self::playwright_diag(),
            Diagnostic::new("security", true, HealthStatus::Available, "permission engine required; no host bypass"),
            Diagnostic::new("cargo_audit", false, HealthStatus::NotInstalled, "CARGO_AUDIT=NOT_AVAILABLE"),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateState {
    pub current: String,
    pub pending: Option<String>,
    pub previous: Option<String>,
}

pub struct UpdateManager {
    state: Mutex<UpdateState>,
}

impl UpdateManager {
    pub fn new(current: impl Into<String>) -> Self {
        Self {
            state: Mutex::new(UpdateState {
                current: current.into(),
                pending: None,
                previous: None,
            }),
        }
    }

    pub fn apply(&self, next: &str, healthy: bool) -> Result<UpdateState, HardeningError> {
        let mut st = self.state.lock().unwrap();
        if !healthy {
            return Err(HardeningError::Update(
                "health verification failed; leaving current version".into(),
            ));
        }
        st.previous = Some(st.current.clone());
        st.current = next.into();
        st.pending = None;
        Ok(st.clone())
    }

    pub fn rollback(&self) -> Result<UpdateState, HardeningError> {
        let mut st = self.state.lock().unwrap();
        let prev = st
            .previous
            .clone()
            .ok_or_else(|| HardeningError::Update("nothing to rollback".into()))?;
        st.current = prev;
        st.previous = None;
        st.pending = None;
        Ok(st.clone())
    }

    pub fn state(&self) -> UpdateState {
        self.state.lock().unwrap().clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Health {
    pub system: bool,
    pub runtime: bool,
    pub worker: bool,
    pub agent: bool,
    pub capability: bool,
    pub memory: bool,
    pub storage: bool,
}

impl Health {
    pub fn all_ok() -> Self {
        Self {
            system: true,
            runtime: true,
            worker: true,
            agent: true,
            capability: true,
            memory: true,
            storage: true,
        }
    }
}

pub fn first_boot_steps() -> Vec<&'static str> {
    vec![
        "initialize system",
        "detect hardware",
        "detect virtualization",
        "detect available runtimes",
        "detect capabilities",
        "create workspace",
        "initialize permissions",
        "initialize storage",
        "verify system health",
        "present setup UI",
    ]
}
