use crate::error::{WorkspaceError, WorkspaceResult};
use async_trait::async_trait;
use dashmap::DashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Filesystem backend for one execution runtime.
/// Physical bytes live here; the workspace graph stays logical.
#[async_trait]
pub trait RuntimeFilesystem: Send + Sync {
    fn runtime_id(&self) -> &str;
    fn is_available(&self) -> bool;
    fn set_available(&self, available: bool);
    async fn write(&self, physical: &str, data: &[u8]) -> WorkspaceResult<()>;
    async fn read(&self, physical: &str) -> WorkspaceResult<Vec<u8>>;
    async fn delete(&self, physical: &str) -> WorkspaceResult<()>;
    async fn exists(&self, physical: &str) -> bool;
}

pub struct InMemoryFilesystem {
    runtime_id: String,
    files: DashMap<String, Vec<u8>>,
    available: AtomicBool,
}

impl InMemoryFilesystem {
    pub fn new(runtime_id: impl Into<String>) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            files: DashMap::new(),
            available: AtomicBool::new(true),
        }
    }
}

#[async_trait]
impl RuntimeFilesystem for InMemoryFilesystem {
    fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    fn is_available(&self) -> bool {
        self.available.load(Ordering::SeqCst)
    }

    fn set_available(&self, available: bool) {
        self.available.store(available, Ordering::SeqCst);
    }

    async fn write(&self, physical: &str, data: &[u8]) -> WorkspaceResult<()> {
        if !self.is_available() {
            return Err(WorkspaceError::RuntimeUnavailable(self.runtime_id.clone()));
        }
        self.files.insert(physical.to_string(), data.to_vec());
        Ok(())
    }

    async fn read(&self, physical: &str) -> WorkspaceResult<Vec<u8>> {
        if !self.is_available() {
            return Err(WorkspaceError::RuntimeUnavailable(self.runtime_id.clone()));
        }
        self.files
            .get(physical)
            .map(|v| v.clone())
            .ok_or_else(|| WorkspaceError::ItemNotFound(physical.to_string()))
    }

    async fn delete(&self, physical: &str) -> WorkspaceResult<()> {
        if !self.is_available() {
            return Err(WorkspaceError::RuntimeUnavailable(self.runtime_id.clone()));
        }
        self.files.remove(physical);
        Ok(())
    }

    async fn exists(&self, physical: &str) -> bool {
        self.is_available() && self.files.contains_key(physical)
    }
}

/// Dedicated on-disk root. Workspace host backend must not write outside it.
pub fn dedicated_host_root() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\CognyxOSTestWorkspace")
    } else {
        std::env::temp_dir().join("CognyxOSTestWorkspace")
    }
}

/// Real host filesystem scoped to a dedicated root. Uses std::fs, not InMemoryFilesystem.
pub struct HostFilesystem {
    runtime_id: String,
    root: PathBuf,
    available: AtomicBool,
    lock: Mutex<()>,
}

impl HostFilesystem {
    pub fn open(runtime_id: impl Into<String>, root: impl Into<PathBuf>) -> WorkspaceResult<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root).map_err(|e| WorkspaceError::Io(e.to_string()))?;
        let root = std::fs::canonicalize(&root).unwrap_or(root);
        Ok(Self {
            runtime_id: runtime_id.into(),
            root,
            available: AtomicBool::new(true),
            lock: Mutex::new(()),
        })
    }

    pub fn dedicated(runtime_id: impl Into<String>) -> WorkspaceResult<Self> {
        Self::open(runtime_id, dedicated_host_root())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn resolve(&self, physical: &str) -> WorkspaceResult<PathBuf> {
        let logical = physical
            .split_once(':')
            .map(|(_, rest)| rest)
            .unwrap_or(physical);
        let rel = logical.trim_start_matches(['/', '\\']);
        let mut out = self.root.clone();
        for c in Path::new(rel).components() {
            match c {
                Component::CurDir => {}
                Component::Normal(p) => out.push(p),
                _ => {
                    return Err(WorkspaceError::InvalidPath(format!(
                        "refusing path outside dedicated root: {physical}"
                    )))
                }
            }
        }
        let canon_root = &self.root;
        if let (Ok(c_out), Ok(c_root)) = (out.canonicalize(), canon_root.canonicalize()) {
            if !c_out.starts_with(&c_root) {
                return Err(WorkspaceError::InvalidPath(physical.to_string()));
            }
        } else {
            // File may not exist yet; ensure prefix is under root.
            if !out.starts_with(&self.root) {
                return Err(WorkspaceError::InvalidPath(physical.to_string()));
            }
        }
        Ok(out)
    }
}

#[async_trait]
impl RuntimeFilesystem for HostFilesystem {
    fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    fn is_available(&self) -> bool {
        self.available.load(Ordering::SeqCst)
    }

    fn set_available(&self, available: bool) {
        self.available.store(available, Ordering::SeqCst);
    }

    async fn write(&self, physical: &str, data: &[u8]) -> WorkspaceResult<()> {
        if !self.is_available() {
            return Err(WorkspaceError::RuntimeUnavailable(self.runtime_id.clone()));
        }
        let _g = self.lock.lock().unwrap();
        let path = self.resolve(physical)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| WorkspaceError::Io(e.to_string()))?;
        }
        std::fs::write(&path, data).map_err(|e| WorkspaceError::Io(e.to_string()))
    }

    async fn read(&self, physical: &str) -> WorkspaceResult<Vec<u8>> {
        if !self.is_available() {
            return Err(WorkspaceError::RuntimeUnavailable(self.runtime_id.clone()));
        }
        let path = self.resolve(physical)?;
        std::fs::read(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                WorkspaceError::ItemNotFound(physical.to_string())
            } else {
                WorkspaceError::Io(e.to_string())
            }
        })
    }

    async fn delete(&self, physical: &str) -> WorkspaceResult<()> {
        if !self.is_available() {
            return Err(WorkspaceError::RuntimeUnavailable(self.runtime_id.clone()));
        }
        let path = self.resolve(physical)?;
        if path.is_dir() {
            std::fs::remove_dir(&path).map_err(|e| WorkspaceError::Io(e.to_string()))?;
        } else if path.exists() {
            std::fs::remove_file(&path).map_err(|e| WorkspaceError::Io(e.to_string()))?;
        }
        Ok(())
    }

    async fn exists(&self, physical: &str) -> bool {
        self.is_available() && self.resolve(physical).map(|p| p.exists()).unwrap_or(false)
    }
}
