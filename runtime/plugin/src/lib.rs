//! Plugin runtime. IN-PROCESS only. WASM NOT IMPLEMENTED. Do not treat execution as sandboxed Wasm.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::Mutex;

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum PluginError {
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("quota exceeded: {0}")]
    Quota(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid manifest: {0}")]
    Manifest(String),
    #[error("disabled: {0}")]
    Disabled(String),
}

pub type PluginResult<T> = Result<T, PluginError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginPermission {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginCapability {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub permissions: Vec<PluginPermission>,
    pub capabilities: Vec<PluginCapability>,
    pub resources: ResourceQuota,
    pub network_access: Vec<String>,
    pub filesystem_scopes: Vec<String>,
    pub runtime_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceQuota {
    pub cpu_millis: u32,
    pub ram_mb: u32,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            cpu_millis: 100,
            ram_mb: 64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginLifecycle {
    Created,
    Installed,
    Verified,
    Enabled,
    Disabled,
    RolledBack,
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Plugin {
    pub id: String,
    pub manifest: PluginManifest,
    pub lifecycle: PluginLifecycle,
    pub checksum: String,
    pub previous_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub plugin_id: String,
    pub action: String,
    pub detail: String,
}

pub struct PluginRegistry {
    plugins: DashMap<String, Plugin>,
    audit: Mutex<Vec<AuditEvent>>,
    /// Plugins never inherit the full user grant set.
    user_grants: HashSet<String>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: DashMap::new(),
            audit: Mutex::new(Vec::new()),
            user_grants: HashSet::from([
                "filesystem.write".into(),
                "terminal.execute".into(),
                "network.request".into(),
            ]),
        }
    }

    fn audit(&self, plugin_id: &str, action: &str, detail: &str) {
        self.audit.lock().unwrap().push(AuditEvent {
            plugin_id: plugin_id.into(),
            action: action.into(),
            detail: detail.into(),
        });
    }

    pub fn audit_log(&self) -> Vec<AuditEvent> {
        self.audit.lock().unwrap().clone()
    }

    pub fn create(name: &str) -> PluginManifest {
        PluginManifest {
            name: name.into(),
            version: "0.1.0".into(),
            api_version: "1".into(),
            permissions: vec![PluginPermission {
                name: "workspace.read".into(),
            }],
            capabilities: vec![PluginCapability {
                name: "echo.say".into(),
            }],
            resources: ResourceQuota::default(),
            network_access: vec![],
            filesystem_scopes: vec!["/Workspace/Artifacts".into()],
            runtime_requirements: vec!["linux".into()],
        }
    }

    pub fn build(manifest: &PluginManifest) -> String {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(manifest).unwrap_or_default())
        )
    }

    pub fn install(&self, manifest: PluginManifest) -> PluginResult<Plugin> {
        if manifest.name.is_empty() {
            return Err(PluginError::Manifest("name required".into()));
        }
        for perm in &manifest.permissions {
            if self.user_grants.contains(&perm.name)
                && !matches!(perm.name.as_str(), "workspace.read" | "echo.say")
            {
                return Err(PluginError::PermissionDenied(format!(
                    "plugin cannot inherit user permission {}",
                    perm.name
                )));
            }
        }
        let checksum = Self::build(&manifest);
        let plugin = Plugin {
            id: format!("plug-{}", manifest.name),
            manifest,
            lifecycle: PluginLifecycle::Installed,
            checksum,
            previous_version: None,
        };
        self.plugins.insert(plugin.id.clone(), plugin.clone());
        self.audit(&plugin.id, "install", &plugin.manifest.version);
        Ok(plugin)
    }

    pub fn verify(&self, plugin_id: &str) -> PluginResult<()> {
        let mut p = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.into()))?;
        let expected = Self::build(&p.manifest);
        if expected != p.checksum {
            return Err(PluginError::Manifest("checksum mismatch".into()));
        }
        p.lifecycle = PluginLifecycle::Verified;
        self.audit(plugin_id, "verify", "ok");
        Ok(())
    }

    pub fn enable(&self, plugin_id: &str) -> PluginResult<()> {
        let mut p = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.into()))?;
        p.lifecycle = PluginLifecycle::Enabled;
        self.audit(plugin_id, "enable", "ok");
        Ok(())
    }

    pub fn disable(&self, plugin_id: &str) -> PluginResult<()> {
        let mut p = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.into()))?;
        p.lifecycle = PluginLifecycle::Disabled;
        self.audit(plugin_id, "disable", "ok");
        Ok(())
    }

    pub fn execute(
        &self,
        plugin_id: &str,
        capability: &str,
        cpu_millis: u32,
        path: Option<&str>,
        network_host: Option<&str>,
    ) -> PluginResult<String> {
        let p = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.into()))?;
        if p.lifecycle != PluginLifecycle::Enabled {
            return Err(PluginError::Disabled(plugin_id.into()));
        }
        if !p.manifest.capabilities.iter().any(|c| c.name == capability) {
            return Err(PluginError::PermissionDenied(capability.into()));
        }
        if cpu_millis > p.manifest.resources.cpu_millis {
            return Err(PluginError::Quota("cpu".into()));
        }
        if let Some(path) = path {
            if !p
                .manifest
                .filesystem_scopes
                .iter()
                .any(|s| path.starts_with(s))
            {
                return Err(PluginError::PermissionDenied(path.into()));
            }
        }
        if let Some(host) = network_host {
            if !p.manifest.network_access.iter().any(|h| h == host) {
                return Err(PluginError::PermissionDenied(host.into()));
            }
        }
        self.audit(plugin_id, "execute", capability);
        Ok(format!("{capability} ok"))
    }

    pub fn update(&self, plugin_id: &str, new_version: &str) -> PluginResult<Plugin> {
        let mut p = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.into()))?;
        p.previous_version = Some(p.manifest.version.clone());
        p.manifest.version = new_version.into();
        p.checksum = Self::build(&p.manifest);
        self.audit(plugin_id, "update", new_version);
        Ok(p.clone())
    }

    pub fn rollback(&self, plugin_id: &str) -> PluginResult<Plugin> {
        let mut p = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.into()))?;
        let prev = p
            .previous_version
            .clone()
            .ok_or_else(|| PluginError::Manifest("no previous version".into()))?;
        p.manifest.version = prev.clone();
        p.previous_version = None;
        p.lifecycle = PluginLifecycle::RolledBack;
        p.checksum = Self::build(&p.manifest);
        self.audit(plugin_id, "rollback", &prev);
        Ok(p.clone())
    }

    pub fn remove(&self, plugin_id: &str) -> PluginResult<()> {
        self.plugins
            .remove(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.into()))?;
        self.audit(plugin_id, "remove", "ok");
        Ok(())
    }

    pub fn inspect(&self, plugin_id: &str) -> PluginResult<Plugin> {
        self.plugins
            .get(plugin_id)
            .map(|p| p.clone())
            .ok_or_else(|| PluginError::NotFound(plugin_id.into()))
    }

    pub fn enabled_capabilities(&self) -> Vec<String> {
        let mut caps = Vec::new();
        for p in self.plugins.iter() {
            if p.lifecycle == PluginLifecycle::Enabled {
                for c in &p.manifest.capabilities {
                    caps.push(c.name.clone());
                }
            }
        }
        caps
    }

    pub fn cli(args: &[&str]) -> String {
        format!("cognyx plugin {}", args.join(" "))
    }
}

/// Sample plugin: one capability, one agent role, one workspace integration.
pub fn sample_echo_plugin() -> PluginManifest {
    let mut m = PluginRegistry::create("sample-echo");
    m.capabilities = vec![PluginCapability {
        name: "echo.say".into(),
    }];
    m.permissions = vec![
        PluginPermission {
            name: "workspace.read".into(),
        },
        PluginPermission {
            name: "agent.role.echo".into(),
        },
    ];
    m
}
