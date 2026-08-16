use crate::memory::{ContextEngine, WorkingMemory};
use crate::store::{local_embed, LocalVectorStore, VectorStoreProvider};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryKind {
    ShortTerm,
    Working,
    Episodic,
    Semantic,
    Preference,
    Task,
    Artifact,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryPrivacy {
    pub owner: String,
    pub scope: String,
    pub retention_secs: u64,
    pub visibility: String,
    pub classification: String,
    pub consent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryRecord {
    pub id: String,
    pub kind: MemoryKind,
    pub text: String,
    pub created_at: u64,
    pub privacy: MemoryPrivacy,
    pub task_id: Option<String>,
    pub artifact_id: Option<String>,
    pub workspace_id: Option<String>,
}

pub struct LongTermMemory {
    pub working: Arc<ContextEngine>,
    records: DashMap<String, MemoryRecord>,
    store: Arc<dyn VectorStoreProvider>,
    enabled: AtomicBool,
    max_inject: usize,
    persist_dir: Mutex<Option<std::path::PathBuf>>,
}

impl LongTermMemory {
    pub fn new(working: Arc<ContextEngine>) -> Self {
        Self {
            working,
            records: DashMap::new(),
            store: Arc::new(LocalVectorStore::new()),
            enabled: AtomicBool::new(true),
            max_inject: 8,
            persist_dir: Mutex::new(None),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Optional JSON persistence. Default is IN-PROCESS (no disk).
    pub fn enable_disk_persist(&self, dir: impl Into<std::path::PathBuf>) {
        *self.persist_dir.lock().unwrap() = Some(dir.into());
        self.flush_disk();
    }

    fn flush_disk(&self) {
        let dir = self.persist_dir.lock().unwrap().clone();
        let Some(dir) = dir else { return };
        let _ = std::fs::create_dir_all(&dir);
        let rows: Vec<MemoryRecord> = self.records.iter().map(|e| e.value().clone()).collect();
        if let Ok(body) = serde_json::to_string_pretty(&rows) {
            let _ = std::fs::write(dir.join("records.json"), body);
        }
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub async fn ingest(
        &self,
        kind: MemoryKind,
        text: impl Into<String>,
        privacy: MemoryPrivacy,
        task_id: Option<String>,
        artifact_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<MemoryRecord, String> {
        if !privacy.consent {
            return Err("consent required".into());
        }
        if matches!(privacy.classification.as_str(), "secret" | "credential") {
            return Err("refusing to store sensitive classification".into());
        }
        let text = text.into();
        let rec = MemoryRecord {
            id: format!("mem-{}", uuid::Uuid::now_v7()),
            kind,
            text: text.clone(),
            created_at: Self::now(),
            privacy,
            task_id,
            artifact_id,
            workspace_id,
        };
        if self.enabled() {
            self.store
                .upsert(&rec.id, local_embed(&text, 32), &text)
                .await;
            self.records.insert(rec.id.clone(), rec.clone());
            self.flush_disk();
        }
        Ok(rec)
    }

    pub async fn retrieve(
        &self,
        query: &str,
        requester: &str,
        limit: usize,
    ) -> Vec<(MemoryRecord, f32)> {
        if !self.enabled() {
            return vec![];
        }
        let hits = self
            .store
            .search(&local_embed(query, 32), limit.max(1) * 4)
            .await;
        let mut out = Vec::new();
        for (id, score) in hits {
            if let Some(rec) = self.records.get(&id) {
                if rec.privacy.owner == requester
                    || rec.privacy.visibility == "shared"
                    || rec.privacy.scope == "task"
                {
                    out.push((rec.clone(), score));
                }
            }
        }
        out.truncate(limit.min(self.max_inject));
        out
    }

    pub fn delete(&self, id: &str, requester: &str) -> Result<(), String> {
        let rec = self
            .records
            .get(id)
            .ok_or_else(|| "not found".to_string())?;
        if rec.privacy.owner != requester {
            return Err("not owner".into());
        }
        drop(rec);
        self.records.remove(id);
        self.flush_disk();
        let store = Arc::clone(&self.store);
        let id = id.to_string();
        tokio::spawn(async move {
            store.delete(&id).await;
        });
        Ok(())
    }

    pub fn delete_category(&self, requester: &str, kind: MemoryKind) -> usize {
        let ids: Vec<String> = self
            .records
            .iter()
            .filter(|e| e.privacy.owner == requester && e.kind == kind)
            .map(|e| e.id.clone())
            .collect();
        let n = ids.len();
        for id in ids {
            self.records.remove(&id);
        }
        n
    }

    pub fn view(&self, requester: &str) -> Vec<MemoryRecord> {
        self.records
            .iter()
            .filter(|e| e.privacy.owner == requester)
            .map(|e| e.clone())
            .collect()
    }

    pub async fn consolidate(&self, requester: &str) -> usize {
        let short: Vec<MemoryRecord> = self
            .records
            .iter()
            .filter(|e| e.privacy.owner == requester && e.kind == MemoryKind::ShortTerm)
            .map(|e| e.clone())
            .collect();
        let mut n = 0;
        for rec in short {
            if rec.text.len() < 8 {
                self.records.remove(&rec.id);
                continue;
            }
            let mut next = rec.clone();
            next.kind = MemoryKind::Episodic;
            self.records.insert(next.id.clone(), next);
            n += 1;
        }
        n
    }

    pub fn remember_preference(&self, owner: &str, key: &str, value: &str) -> MemoryRecord {
        let rec = MemoryRecord {
            id: format!("pref-{}", uuid::Uuid::now_v7()),
            kind: MemoryKind::Preference,
            text: format!("{key}={value}"),
            created_at: Self::now(),
            privacy: MemoryPrivacy {
                owner: owner.into(),
                scope: "preference".into(),
                retention_secs: 365 * 24 * 3600,
                visibility: "private".into(),
                classification: "preference".into(),
                consent: true,
            },
            task_id: None,
            artifact_id: None,
            workspace_id: None,
        };
        self.records.insert(rec.id.clone(), rec.clone());
        rec
    }

    pub fn working_memory(&self, session_id: &str) -> Option<WorkingMemory> {
        self.working.get_working_memory(session_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reflection {
    pub task_id: String,
    pub worked: Vec<String>,
    pub failed: Vec<String>,
    pub change: Vec<String>,
    pub do_not_repeat: Vec<String>,
}

impl Reflection {
    pub fn from_task(task_id: &str, success: bool, note: &str) -> Self {
        if success {
            Self {
                task_id: task_id.into(),
                worked: vec![note.into()],
                failed: vec![],
                change: vec![],
                do_not_repeat: vec![],
            }
        } else {
            Self {
                task_id: task_id.into(),
                worked: vec![],
                failed: vec![note.into()],
                change: vec!["retry with different runtime".into()],
                do_not_repeat: vec![note.into()],
            }
        }
    }
}
