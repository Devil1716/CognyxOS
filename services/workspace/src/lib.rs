//! Phase 7 unified workspace.
//!
//! Logical files, apps, tasks, and artifacts live here. Physical bytes live on
//! a runtime filesystem. Permission checks go through the Phase 3
//! `PermissionEngine`. Runtime presence goes through `RuntimeRegistry`.

pub mod backend;
pub mod error;
pub mod events;
pub mod manager;
pub mod model;
pub mod security;

pub use backend::{dedicated_host_root, HostFilesystem, InMemoryFilesystem, RuntimeFilesystem};
pub use cognyx_agent_core::{PermissionContext, PermissionDecision, PermissionEngine};
pub use cognyx_execution::RuntimeRegistry;
pub use error::{WorkspaceError, WorkspaceResult};
pub use events::WorkspaceEvent;
pub use manager::{WorkspaceCheckpoint, WorkspaceManager, WorkspaceStateSnapshot};
pub use model::*;
pub use security::{ctx_for, WorkspaceSecurity};

pub struct WorkspaceService;

impl WorkspaceService {
    pub fn manager(
        registry: std::sync::Arc<cognyx_execution::RuntimeRegistry>,
    ) -> WorkspaceManager {
        WorkspaceManager::new(registry)
    }
}
