use crate::error::{WorkspaceError, WorkspaceResult};
use cognyx_agent_core::{PermissionContext, PermissionDecision, PermissionEngine};

/// Workspace security is a facade over the Phase 3 PermissionEngine.
/// This layer must never grant access the engine would deny.
pub struct WorkspaceSecurity {
    engine: PermissionEngine,
}

impl Default for WorkspaceSecurity {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceSecurity {
    pub fn new() -> Self {
        Self {
            engine: PermissionEngine::new(),
        }
    }

    pub fn check(&self, capability: &str, ctx: &PermissionContext) -> WorkspaceResult<()> {
        match self.engine.authorize(capability, ctx) {
            PermissionDecision::Allow => Ok(()),
            PermissionDecision::Deny => {
                Err(WorkspaceError::PermissionDenied(capability.to_string()))
            }
            PermissionDecision::UserApprovalRequired => {
                Err(WorkspaceError::ApprovalRequired(capability.to_string()))
            }
        }
    }
}

pub fn ctx_for(user_id: &str, granted: &[&str]) -> PermissionContext {
    PermissionContext {
        user_id: user_id.to_string(),
        session_id: "workspace".to_string(),
        granted_capabilities: granted.iter().map(|s| s.to_string()).collect(),
        is_administrator: false,
    }
}
