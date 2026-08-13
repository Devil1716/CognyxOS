use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionContext {
    pub session_id: String,
    pub user_id: String,
    pub active_task_ids: Vec<String>,
    pub environment_variables: HashMap<String, String>,
    pub created_at_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskHistoryRecord {
    pub record_id: String,
    pub session_id: String,
    pub task_id: String,
    pub action: String,
    pub result: String,
    pub timestamp_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkingMemory {
    pub session_id: String,
    pub current_task_id: Option<String>,
    pub current_plan_id: Option<String>,
    pub current_node_id: Option<String>,
    pub active_permissions: Vec<String>,
    pub recent_results: Vec<String>,
    pub working_variables: HashMap<String, String>,
}

pub struct ContextEngine {
    sessions: Arc<DashMap<String, SessionContext>>,
    working_memories: Arc<DashMap<String, WorkingMemory>>,
    task_histories: Arc<DashMap<String, Vec<TaskHistoryRecord>>>,
}

impl Default for ContextEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextEngine {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            working_memories: Arc::new(DashMap::new()),
            task_histories: Arc::new(DashMap::new()),
        }
    }

    pub fn get_or_create_session(&self, session_id: &str, user_id: &str) -> SessionContext {
        self.sessions
            .entry(session_id.to_string())
            .or_insert_with(|| {
                info!("Creating new SessionContext for session '{}'", session_id);
                SessionContext {
                    session_id: session_id.to_string(),
                    user_id: user_id.to_string(),
                    active_task_ids: vec![],
                    environment_variables: HashMap::new(),
                    created_at_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                }
            })
            .clone()
    }

    pub fn update_working_memory(&self, memory: WorkingMemory) {
        info!("Updating WorkingMemory for session '{}'", memory.session_id);
        self.working_memories
            .insert(memory.session_id.clone(), memory);
    }

    pub fn get_working_memory(&self, session_id: &str) -> Option<WorkingMemory> {
        self.working_memories
            .get(session_id)
            .map(|e| e.value().clone())
    }

    pub fn record_task_history(&self, record: TaskHistoryRecord) {
        let mut history = self
            .task_histories
            .entry(record.session_id.clone())
            .or_default();
        history.push(record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_engine_working_memory() {
        let engine = ContextEngine::new();
        let session = engine.get_or_create_session("sess-1", "user-1");
        assert_eq!(session.user_id, "user-1");

        let wm = WorkingMemory {
            session_id: "sess-1".to_string(),
            current_task_id: Some("task-1".to_string()),
            current_plan_id: Some("plan-1".to_string()),
            current_node_id: Some("node-1".to_string()),
            active_permissions: vec!["bash".to_string()],
            recent_results: vec!["success".to_string()],
            working_variables: HashMap::new(),
        };

        engine.update_working_memory(wm);
        let fetched = engine.get_working_memory("sess-1").unwrap();
        assert_eq!(fetched.current_task_id, Some("task-1".to_string()));
    }
}
