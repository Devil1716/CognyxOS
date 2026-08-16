use crate::backend::{dedicated_host_root, HostFilesystem, InMemoryFilesystem, RuntimeFilesystem};
use crate::error::{WorkspaceError, WorkspaceResult};
use crate::events::WorkspaceEvent;
use crate::model::*;
use crate::security::WorkspaceSecurity;
use cognyx_agent_core::PermissionContext;
use cognyx_execution::RuntimeRegistry;
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

const RECENT_LIMIT: usize = 32;

#[derive(Debug, Clone)]
pub struct WorkspaceStateSnapshot {
    pub active_workspace: Option<String>,
    pub open_applications: Vec<String>,
    pub active_tasks: Vec<String>,
    pub running_agents: Vec<String>,
    pub recent_files: Vec<String>,
    pub recent_artifacts: Vec<String>,
    pub active_runtimes: Vec<String>,
    pub session_id: String,
}

struct SessionState {
    active_workspace: Option<String>,
    open_applications: Vec<String>,
    active_tasks: Vec<String>,
    running_agents: Vec<String>,
    recent_files: VecDeque<String>,
    recent_artifacts: VecDeque<String>,
    session_id: String,
}

/// Snapshot used by workspace recovery. Secrets are never included.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceCheckpoint {
    pub workspaces: Vec<Workspace>,
    pub items: Vec<WorkspaceItem>,
    pub artifacts: Vec<WorkspaceArtifact>,
}

pub struct WorkspaceManager {
    security: WorkspaceSecurity,
    registry: Arc<RuntimeRegistry>,
    workspaces: DashMap<String, Workspace>,
    items: DashMap<String, WorkspaceItem>,
    artifacts: DashMap<String, WorkspaceArtifact>,
    versions: DashMap<(String, u64), Vec<u8>>,
    filesystems: DashMap<String, Arc<dyn RuntimeFilesystem>>,
    events: Mutex<Vec<WorkspaceEvent>>,
    state: RwLock<SessionState>,
}

impl Default for WorkspaceManager {
    fn default() -> Self {
        Self::new(Arc::new(RuntimeRegistry::new()))
    }
}

impl WorkspaceManager {
    pub fn new(registry: Arc<RuntimeRegistry>) -> Self {
        Self {
            security: WorkspaceSecurity::new(),
            registry,
            workspaces: DashMap::new(),
            items: DashMap::new(),
            artifacts: DashMap::new(),
            versions: DashMap::new(),
            filesystems: DashMap::new(),
            events: Mutex::new(Vec::new()),
            state: RwLock::new(SessionState {
                active_workspace: None,
                open_applications: Vec::new(),
                active_tasks: Vec::new(),
                running_agents: Vec::new(),
                recent_files: VecDeque::new(),
                recent_artifacts: VecDeque::new(),
                session_id: new_id("sess"),
            }),
        }
    }

    pub fn registry(&self) -> Arc<RuntimeRegistry> {
        Arc::clone(&self.registry)
    }

    pub fn attach_runtime_fs(&self, fs: Arc<dyn RuntimeFilesystem>) {
        self.filesystems.insert(fs.runtime_id().to_string(), fs);
    }

    pub fn attach_in_memory_runtime(
        &self,
        runtime_id: impl Into<String>,
    ) -> Arc<InMemoryFilesystem> {
        let fs = Arc::new(InMemoryFilesystem::new(runtime_id));
        self.attach_runtime_fs(fs.clone());
        fs
    }

    pub fn attach_host_filesystem(
        &self,
        runtime_id: impl Into<String>,
        root: impl Into<std::path::PathBuf>,
    ) -> WorkspaceResult<Arc<HostFilesystem>> {
        let fs = Arc::new(HostFilesystem::open(runtime_id, root)?);
        self.attach_runtime_fs(fs.clone());
        Ok(fs)
    }

    pub fn attach_dedicated_host_filesystem(
        &self,
        runtime_id: impl Into<String>,
    ) -> WorkspaceResult<Arc<HostFilesystem>> {
        self.attach_host_filesystem(runtime_id, dedicated_host_root())
    }

    pub fn set_runtime_available(&self, runtime_id: &str, available: bool) -> WorkspaceResult<()> {
        let fs = self
            .filesystems
            .get(runtime_id)
            .ok_or_else(|| WorkspaceError::RuntimeUnavailable(runtime_id.to_string()))?;
        fs.set_available(available);
        Ok(())
    }

    fn emit(&self, event_type: &str, workspace_id: &str, payload: &str) {
        let event = WorkspaceEvent::new(event_type, workspace_id, payload);
        self.events.lock().unwrap().push(event);
    }

    pub fn events(&self) -> Vec<WorkspaceEvent> {
        self.events.lock().unwrap().clone()
    }

    fn ensure_runtime(&self, runtime_id: &str) -> WorkspaceResult<Arc<dyn RuntimeFilesystem>> {
        let registered = self.registry.list_runtime_ids();
        if !registered.iter().any(|id| id == runtime_id) {
            return Err(WorkspaceError::RuntimeUnavailable(runtime_id.to_string()));
        }
        let fs = self
            .filesystems
            .get(runtime_id)
            .ok_or_else(|| WorkspaceError::RuntimeUnavailable(runtime_id.to_string()))?;
        if !fs.is_available() {
            return Err(WorkspaceError::RuntimeUnavailable(runtime_id.to_string()));
        }
        Ok(Arc::clone(fs.value()))
    }

    fn authorize_write(
        &self,
        ctx: &PermissionContext,
        perms: &WorkspacePermission,
        capability: &str,
    ) -> WorkspaceResult<()> {
        self.security.check(capability, ctx)?;
        if !perms.allows_write(&ctx.user_id) && !ctx.is_administrator {
            return Err(WorkspaceError::PermissionDenied(format!(
                "workspace acl denied write for {}",
                ctx.user_id
            )));
        }
        Ok(())
    }

    fn authorize_read(
        &self,
        ctx: &PermissionContext,
        perms: &WorkspacePermission,
    ) -> WorkspaceResult<()> {
        self.security.check("filesystem.read", ctx)?;
        if !perms.allows_read(&ctx.user_id) && !ctx.is_administrator {
            return Err(WorkspaceError::PermissionDenied(format!(
                "workspace acl denied read for {}",
                ctx.user_id
            )));
        }
        Ok(())
    }

    pub fn create_workspace(
        &self,
        name: impl Into<String>,
        owner: impl Into<String>,
        default_runtime_id: &str,
        ctx: &PermissionContext,
    ) -> WorkspaceResult<Workspace> {
        self.security.check("filesystem.write", ctx)?;
        self.ensure_runtime(default_runtime_id)?;

        let owner = owner.into();
        let now = now_secs();
        let ws = Workspace {
            id: new_id("ws"),
            name: name.into(),
            owner: owner.clone(),
            permissions: WorkspacePermission::owner_only(&owner),
            created_at: now,
            modified_at: now,
            metadata: WorkspaceMetadata::default(),
        };

        for logical in default_logical_layout() {
            let folder = WorkspaceItem {
                id: new_id("item"),
                kind: WorkspaceItemKind::Folder,
                name: logical
                    .rsplit('/')
                    .next()
                    .unwrap_or("Workspace")
                    .to_string(),
                location: logical.to_string(),
                runtime_id: default_runtime_id.to_string(),
                owner: owner.clone(),
                permissions: WorkspacePermission::owner_only(&owner),
                created_at: now,
                modified_at: now,
                metadata: WorkspaceMetadata::default(),
                parent_id: None,
                checksum: None,
                version: 1,
            };
            self.items.insert(folder.id.clone(), folder);
        }

        self.workspaces.insert(ws.id.clone(), ws.clone());
        {
            let mut state = self.state.write().unwrap();
            state.active_workspace = Some(ws.id.clone());
        }
        self.emit("workspace.created", &ws.id, &ws.name);
        Ok(ws)
    }

    pub fn get_workspace(&self, id: &str) -> WorkspaceResult<Workspace> {
        self.workspaces
            .get(id)
            .map(|w| w.clone())
            .ok_or_else(|| WorkspaceError::WorkspaceNotFound(id.to_string()))
    }

    fn require_workspace(&self, workspace_id: &str) -> WorkspaceResult<Workspace> {
        self.get_workspace(workspace_id)
    }

    fn parent_id_for(&self, logical: &str) -> Option<String> {
        let parent = logical.rsplit_once('/').map(|(p, _)| p)?;
        self.items.iter().find_map(|e| {
            let item = e.value();
            if item.location == parent && item.kind == WorkspaceItemKind::Folder {
                Some(item.id.clone())
            } else {
                None
            }
        })
    }

    pub fn create_folder(
        &self,
        workspace_id: &str,
        logical: &str,
        runtime_id: &str,
        ctx: &PermissionContext,
    ) -> WorkspaceResult<WorkspaceItem> {
        let ws = self.require_workspace(workspace_id)?;
        validate_logical_path(logical)?;
        self.authorize_write(ctx, &ws.permissions, "filesystem.write")?;
        self.ensure_runtime(runtime_id)?;

        if self.find_item_at(logical, runtime_id).is_some() {
            return Err(WorkspaceError::AlreadyExists(logical.to_string()));
        }

        let now = now_secs();
        let name = logical.rsplit('/').next().unwrap_or(logical).to_string();
        let item = WorkspaceItem {
            id: new_id("item"),
            kind: WorkspaceItemKind::Folder,
            name,
            location: logical.to_string(),
            runtime_id: runtime_id.to_string(),
            owner: ctx.user_id.clone(),
            permissions: ws.permissions.clone(),
            created_at: now,
            modified_at: now,
            metadata: WorkspaceMetadata::default(),
            parent_id: self.parent_id_for(logical),
            checksum: None,
            version: 1,
        };
        self.items.insert(item.id.clone(), item.clone());
        self.emit("workspace.folder.created", workspace_id, logical);
        Ok(item)
    }

    pub async fn create_file(
        &self,
        workspace_id: &str,
        logical: &str,
        runtime_id: &str,
        data: &[u8],
        ctx: &PermissionContext,
    ) -> WorkspaceResult<WorkspaceItem> {
        let ws = self.require_workspace(workspace_id)?;
        validate_logical_path(logical)?;
        self.authorize_write(ctx, &ws.permissions, "filesystem.write")?;
        let fs = self.ensure_runtime(runtime_id)?;

        if let Some(existing) = self.find_item_at(logical, runtime_id) {
            return Err(WorkspaceError::AlreadyExists(existing.id));
        }

        let physical = physical_location(runtime_id, logical);
        fs.write(&physical, data).await?;
        let checksum = checksum_bytes(data);
        let now = now_secs();
        let name = logical.rsplit('/').next().unwrap_or(logical).to_string();
        let item = WorkspaceItem {
            id: new_id("item"),
            kind: WorkspaceItemKind::File,
            name,
            location: logical.to_string(),
            runtime_id: runtime_id.to_string(),
            owner: ctx.user_id.clone(),
            permissions: ws.permissions.clone(),
            created_at: now,
            modified_at: now,
            metadata: WorkspaceMetadata::default(),
            parent_id: self.parent_id_for(logical),
            checksum: Some(checksum),
            version: 1,
        };
        self.versions.insert((item.id.clone(), 1), data.to_vec());
        self.items.insert(item.id.clone(), item.clone());
        self.touch_recent_file(&item.id);
        self.emit("workspace.file.created", workspace_id, logical);
        Ok(item)
    }

    pub async fn write_file(
        &self,
        item_id: &str,
        data: &[u8],
        ctx: &PermissionContext,
    ) -> WorkspaceResult<WorkspaceItem> {
        let mut item = self
            .items
            .get(item_id)
            .map(|i| i.clone())
            .ok_or_else(|| WorkspaceError::ItemNotFound(item_id.to_string()))?;
        self.authorize_write(ctx, &item.permissions, "filesystem.write")?;
        let fs = self.ensure_runtime(&item.runtime_id)?;
        let physical = physical_location(&item.runtime_id, &item.location);
        fs.write(&physical, data).await?;
        item.version += 1;
        item.checksum = Some(checksum_bytes(data));
        item.modified_at = now_secs();
        self.versions
            .insert((item.id.clone(), item.version), data.to_vec());
        self.items.insert(item.id.clone(), item.clone());
        self.touch_recent_file(&item.id);
        Ok(item)
    }

    pub async fn read_file(
        &self,
        item_id: &str,
        ctx: &PermissionContext,
    ) -> WorkspaceResult<Vec<u8>> {
        let item = self
            .items
            .get(item_id)
            .map(|i| i.clone())
            .ok_or_else(|| WorkspaceError::ItemNotFound(item_id.to_string()))?;
        self.authorize_read(ctx, &item.permissions)?;
        let fs = self.ensure_runtime(&item.runtime_id)?;
        let physical = physical_location(&item.runtime_id, &item.location);
        let data = fs.read(&physical).await?;
        if let Some(expected) = &item.checksum {
            let actual = checksum_bytes(&data);
            if &actual != expected {
                return Err(WorkspaceError::ChecksumMismatch(item_id.to_string()));
            }
        }
        self.touch_recent_file(&item.id);
        Ok(data)
    }

    pub async fn copy_file(
        &self,
        workspace_id: &str,
        source_item_id: &str,
        dest_runtime_id: &str,
        dest_logical: Option<&str>,
        ctx: &PermissionContext,
    ) -> WorkspaceResult<WorkspaceItem> {
        self.security.check("filesystem.copy", ctx)?;
        let source = self
            .items
            .get(source_item_id)
            .map(|i| i.clone())
            .ok_or_else(|| WorkspaceError::ItemNotFound(source_item_id.to_string()))?;
        self.authorize_read(ctx, &source.permissions)?;

        let dest_logical = dest_logical.unwrap_or(&source.location).to_string();
        validate_logical_path(&dest_logical)?;

        let src_fs = self.ensure_runtime(&source.runtime_id)?;
        let dst_fs = self.ensure_runtime(dest_runtime_id)?;
        let src_physical = physical_location(&source.runtime_id, &source.location);
        let data = src_fs.read(&src_physical).await?;
        let checksum = checksum_bytes(&data);

        if let Some(existing) = self.find_item_at(&dest_logical, dest_runtime_id) {
            if existing.checksum.as_deref() != Some(&checksum) {
                self.emit(
                    "workspace.conflict",
                    workspace_id,
                    &format!("{} vs {}", existing.id, source.id),
                );
                return Err(WorkspaceError::Conflict(format!(
                    "refusing to overwrite {} (checksum mismatch)",
                    existing.id
                )));
            }
            return Ok(existing);
        }

        let dest_physical = physical_location(dest_runtime_id, &dest_logical);
        dst_fs.write(&dest_physical, &data).await?;

        let now = now_secs();
        let copied = WorkspaceItem {
            id: new_id("item"),
            kind: WorkspaceItemKind::File,
            name: dest_logical
                .rsplit('/')
                .next()
                .unwrap_or(&dest_logical)
                .to_string(),
            location: dest_logical,
            runtime_id: dest_runtime_id.to_string(),
            owner: source.owner.clone(),
            permissions: source.permissions.clone(),
            created_at: now,
            modified_at: now,
            metadata: source.metadata.clone(),
            parent_id: self.parent_id_for(&source.location),
            checksum: Some(checksum),
            version: 1,
        };
        self.versions.insert((copied.id.clone(), 1), data);
        self.items.insert(copied.id.clone(), copied.clone());
        self.emit(
            "workspace.file.copied",
            workspace_id,
            &format!("{} -> {}", source.runtime_id, dest_runtime_id),
        );
        Ok(copied)
    }

    pub async fn move_file(
        &self,
        workspace_id: &str,
        source_item_id: &str,
        dest_runtime_id: &str,
        dest_logical: &str,
        ctx: &PermissionContext,
    ) -> WorkspaceResult<WorkspaceItem> {
        self.security.check("filesystem.move", ctx)?;
        let copied = self
            .copy_file(
                workspace_id,
                source_item_id,
                dest_runtime_id,
                Some(dest_logical),
                ctx,
            )
            .await?;
        self.delete_item(source_item_id, ctx).await?;
        Ok(copied)
    }

    pub async fn sync_file(
        &self,
        workspace_id: &str,
        left_item_id: &str,
        right_runtime_id: &str,
        ctx: &PermissionContext,
    ) -> WorkspaceResult<WorkspaceItem> {
        let left = self
            .items
            .get(left_item_id)
            .map(|i| i.clone())
            .ok_or_else(|| WorkspaceError::ItemNotFound(left_item_id.to_string()))?;
        if let Some(right) = self.find_item_at(&left.location, right_runtime_id) {
            if right.checksum != left.checksum {
                self.emit(
                    "workspace.conflict",
                    workspace_id,
                    &format!("sync {} {}", left.id, right.id),
                );
                return Err(WorkspaceError::Conflict(
                    "sync refused: checksums differ; resolve explicitly".into(),
                ));
            }
            return Ok(right);
        }
        self.copy_file(workspace_id, left_item_id, right_runtime_id, None, ctx)
            .await
    }

    pub fn search(&self, query: &str) -> Vec<WorkspaceItem> {
        let q = query.to_lowercase();
        self.items
            .iter()
            .map(|e| e.value().clone())
            .filter(|item| {
                item.name.to_lowercase().contains(&q)
                    || item.location.to_lowercase().contains(&q)
                    || item
                        .metadata
                        .labels
                        .values()
                        .any(|v| v.to_lowercase().contains(&q))
                    || format!("{:?}", item.kind).to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn reference_for(
        &self,
        workspace_id: &str,
        item_id: &str,
    ) -> WorkspaceResult<WorkspaceReference> {
        let item = self
            .items
            .get(item_id)
            .map(|i| i.clone())
            .ok_or_else(|| WorkspaceError::ItemNotFound(item_id.to_string()))?;
        Ok(WorkspaceReference {
            workspace_id: workspace_id.to_string(),
            item_id: item.id,
            runtime_id: item.runtime_id.clone(),
            physical_location: physical_location(&item.runtime_id, &item.location),
            logical_location: item.location,
            permissions: item.permissions,
            checksum: item.checksum.unwrap_or_default(),
            version: item.version,
        })
    }

    pub async fn restore(
        &self,
        item_id: &str,
        version: u64,
        ctx: &PermissionContext,
    ) -> WorkspaceResult<WorkspaceItem> {
        let data = self
            .versions
            .get(&(item_id.to_string(), version))
            .map(|v| v.clone())
            .ok_or_else(|| WorkspaceError::VersionNotFound(format!("{item_id}@{version}")))?;
        let mut item = self.write_file(item_id, &data, ctx).await?;
        item.metadata
            .labels
            .insert("restored_from".into(), version.to_string());
        self.items.insert(item.id.clone(), item.clone());
        Ok(item)
    }

    pub async fn ingest_artifact(
        &self,
        request: ArtifactIngestRequest<'_>,
        ctx: &PermissionContext,
    ) -> WorkspaceResult<WorkspaceArtifact> {
        let logical = format!("{LOGICAL_ARTIFACTS}/{}", request.name);
        let mut item = self
            .create_file(
                request.workspace_id,
                &logical,
                request.runtime_id,
                request.data,
                ctx,
            )
            .await?;
        item.kind = WorkspaceItemKind::Artifact;
        self.items.insert(item.id.clone(), item.clone());
        let artifact = WorkspaceArtifact {
            item: item.clone(),
            source_artifact_id: request.source_artifact_id.to_string(),
            source_task_id: request.source_task_id.to_string(),
            source_agent_id: request.source_agent_id.to_string(),
            shared_with: Vec::new(),
        };
        self.artifacts.insert(item.id.clone(), artifact.clone());
        self.touch_recent_artifact(&item.id);
        self.emit(
            "workspace.artifact.created",
            request.workspace_id,
            request.name,
        );
        Ok(artifact)
    }

    pub fn share_artifact(
        &self,
        artifact_item_id: &str,
        principal: &str,
        ctx: &PermissionContext,
    ) -> WorkspaceResult<WorkspaceArtifact> {
        let mut artifact = self
            .artifacts
            .get(artifact_item_id)
            .map(|a| a.clone())
            .ok_or_else(|| WorkspaceError::ItemNotFound(artifact_item_id.to_string()))?;
        self.authorize_write(ctx, &artifact.item.permissions, "filesystem.write")?;
        if !artifact.shared_with.iter().any(|p| p == principal) {
            artifact.shared_with.push(principal.to_string());
        }
        artifact.item.permissions.read.push(principal.to_string());
        self.items
            .insert(artifact.item.id.clone(), artifact.item.clone());
        self.artifacts
            .insert(artifact.item.id.clone(), artifact.clone());
        Ok(artifact)
    }

    pub fn get_artifact(
        &self,
        artifact_item_id: &str,
        ctx: &PermissionContext,
    ) -> WorkspaceResult<WorkspaceArtifact> {
        let artifact = self
            .artifacts
            .get(artifact_item_id)
            .map(|a| a.clone())
            .ok_or_else(|| WorkspaceError::ItemNotFound(artifact_item_id.to_string()))?;
        self.authorize_read(ctx, &artifact.item.permissions)?;
        Ok(artifact)
    }

    pub async fn delete_item(&self, item_id: &str, ctx: &PermissionContext) -> WorkspaceResult<()> {
        let item = self
            .items
            .get(item_id)
            .map(|i| i.clone())
            .ok_or_else(|| WorkspaceError::ItemNotFound(item_id.to_string()))?;
        self.authorize_write(ctx, &item.permissions, "filesystem.delete")?;
        if item.kind == WorkspaceItemKind::File || item.kind == WorkspaceItemKind::Artifact {
            if let Ok(fs) = self.ensure_runtime(&item.runtime_id) {
                let physical = physical_location(&item.runtime_id, &item.location);
                let _ = fs.delete(&physical).await;
            }
        }
        self.items.remove(item_id);
        self.artifacts.remove(item_id);
        Ok(())
    }

    pub fn snapshot(&self) -> WorkspaceCheckpoint {
        WorkspaceCheckpoint {
            workspaces: self.workspaces.iter().map(|e| e.value().clone()).collect(),
            items: self.items.iter().map(|e| e.value().clone()).collect(),
            artifacts: self.artifacts.iter().map(|e| e.value().clone()).collect(),
        }
    }

    pub fn restore_checkpoint(&self, checkpoint: WorkspaceCheckpoint) {
        self.workspaces.clear();
        self.items.clear();
        self.artifacts.clear();
        for ws in checkpoint.workspaces {
            self.workspaces.insert(ws.id.clone(), ws);
        }
        for item in checkpoint.items {
            self.items.insert(item.id.clone(), item);
        }
        for artifact in checkpoint.artifacts {
            self.artifacts.insert(artifact.item.id.clone(), artifact);
        }
        self.emit("workspace.recovered", "system", "checkpoint restored");
    }

    pub fn state(&self) -> WorkspaceStateSnapshot {
        let state = self.state.read().unwrap();
        WorkspaceStateSnapshot {
            active_workspace: state.active_workspace.clone(),
            open_applications: state.open_applications.clone(),
            active_tasks: state.active_tasks.clone(),
            running_agents: state.running_agents.clone(),
            recent_files: state.recent_files.iter().cloned().collect(),
            recent_artifacts: state.recent_artifacts.iter().cloned().collect(),
            active_runtimes: self.registry.list_runtime_ids(),
            session_id: state.session_id.clone(),
        }
    }

    pub fn record_open_application(&self, app_id: impl Into<String>) {
        self.state
            .write()
            .unwrap()
            .open_applications
            .push(app_id.into());
    }

    pub fn record_active_task(&self, task_id: impl Into<String>) {
        self.state
            .write()
            .unwrap()
            .active_tasks
            .push(task_id.into());
    }

    pub fn record_running_agent(&self, agent_id: impl Into<String>) {
        self.state
            .write()
            .unwrap()
            .running_agents
            .push(agent_id.into());
    }

    fn touch_recent_file(&self, item_id: &str) {
        let mut state = self.state.write().unwrap();
        state.recent_files.push_front(item_id.to_string());
        state.recent_files.truncate(RECENT_LIMIT);
    }

    fn touch_recent_artifact(&self, item_id: &str) {
        let mut state = self.state.write().unwrap();
        state.recent_artifacts.push_front(item_id.to_string());
        state.recent_artifacts.truncate(RECENT_LIMIT);
    }

    fn find_item_at(&self, logical: &str, runtime_id: &str) -> Option<WorkspaceItem> {
        self.items.iter().find_map(|e| {
            let item = e.value();
            if item.location == logical && item.runtime_id == runtime_id {
                Some(item.clone())
            } else {
                None
            }
        })
    }
}
