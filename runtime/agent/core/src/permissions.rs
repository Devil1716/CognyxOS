use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::RwLock;
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Deny,
    UserApprovalRequired,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PermissionContext {
    pub user_id: String,
    pub session_id: String,
    pub granted_capabilities: HashSet<String>,
    pub is_administrator: bool,
}

pub struct PermissionEngine {
    policy_overrides: RwLock<HashSet<String>>,
}

impl Default for PermissionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionEngine {
    pub fn new() -> Self {
        Self {
            policy_overrides: RwLock::new(HashSet::new()),
        }
    }

    pub fn authorize(&self, capability: &str, ctx: &PermissionContext) -> PermissionDecision {
        info!(
            "Evaluating permission for capability '{}' (user: '{}')",
            capability, ctx.user_id
        );

        if ctx.is_administrator {
            return PermissionDecision::Allow;
        }

        // Restricted / sensitive capability security rules
        match capability {
            "camera.capture" | "microphone.capture" | "browser.download" | "browser.upload" => PermissionDecision::UserApprovalRequired,
            "filesystem.write" | "filesystem.delete" | "filesystem.move" | "filesystem.copy"
            | "terminal.execute" | "process.start" | "process.stop" | "application.close"
            | "clipboard.read" | "clipboard.write" => {
                if ctx.granted_capabilities.contains(capability) {
                    PermissionDecision::Allow
                } else {
                    PermissionDecision::UserApprovalRequired
                }
            }
            "filesystem.read"
            | "filesystem.list"
            | "filesystem.metadata"
            | "filesystem.permissions"
            | "process.list"
            | "process.inspect"
            | "process.metrics"
            | "application.list"
            | "application.search"
            | "application.inspect"
            | "application.status"
            | "browser.list"
            | "browser.open"
            | "browser.navigate"
            | "browser.read"
            | "browser.screenshot"
            | "browser.tabs"
            | "browser.back"
            | "browser.forward"
            | "browser.reload"
            | "browser.click"
            | "browser.type"
            | "browser.select"
            | "browser.scroll"
            | "browser.close"
            | "screen.capture"
            | "screen.read"
            | "keyboard.type"
            | "keyboard.press"
            | "keyboard.hotkey"
            | "mouse.move"
            | "mouse.click"
            | "mouse.double_click"
            | "mouse.right_click"
            | "mouse.scroll"
            | "window.list"
            | "window.inspect"
            | "window.focus"
            | "window.activate"
            | "window.minimize"
            | "window.maximize"
            | "window.move"
            | "window.resize"
            | "network.request"
            | "application.open"
            | "package.install"
            | "win32.powershell"
            | "bash"
            | "doc.render"
            | "container.exec"
            | "data.process"
            | "file.write"
            | "memory.query"
            | "session.restore"
            | "process.spawn" => PermissionDecision::Allow,
            _ => {
                if ctx.granted_capabilities.contains(capability) {
                    PermissionDecision::Allow
                } else {
                    PermissionDecision::Deny
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_decisions() {
        let engine = PermissionEngine::new();

        let ctx = PermissionContext {
            user_id: "user-1".to_string(),
            session_id: "s-1".to_string(),
            granted_capabilities: HashSet::from(["filesystem.read".to_string()]),
            is_administrator: false,
        };

        assert_eq!(
            engine.authorize("filesystem.read", &ctx),
            PermissionDecision::Allow
        );
        assert_eq!(
            engine.authorize("camera.capture", &ctx),
            PermissionDecision::UserApprovalRequired
        );
        assert_eq!(
            engine.authorize("unknown.restricted", &ctx),
            PermissionDecision::Deny
        );
    }
}
