use crate::manager::AgentTask;
use dashmap::DashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

pub struct AgentStateStore {
    base_dir: PathBuf,
    memory_map: Arc<DashMap<String, AgentTask>>,
}

impl Default for AgentStateStore {
    fn default() -> Self {
        Self::new(PathBuf::from("/var/lib/cognyxos/agent_state"))
    }
}

impl AgentStateStore {
    pub fn new(base_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&base_dir);
        let store = Self {
            base_dir,
            memory_map: Arc::new(DashMap::new()),
        };
        store.load_all();
        store
    }

    fn load_all(&self) {
        if let Ok(entries) = fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(task) = serde_json::from_str::<AgentTask>(&content) {
                            self.memory_map.insert(task.task_id.clone(), task);
                        }
                    }
                }
            }
        }
    }

    pub fn save_task(&self, task: &AgentTask) {
        self.memory_map.insert(task.task_id.clone(), task.clone());
        let file_path = self.base_dir.join(format!("{}.json", task.task_id));
        if let Ok(json) = serde_json::to_string_pretty(task) {
            let _ = fs::write(file_path, json);
        }
        info!("Persisted AgentTask '{}' to disk state store", task.task_id);
    }

    pub fn get_task(&self, task_id: &str) -> Option<AgentTask> {
        self.memory_map.get(task_id).map(|e| e.value().clone())
    }

    pub fn list_tasks(&self) -> Vec<AgentTask> {
        self.memory_map.iter().map(|e| e.value().clone()).collect()
    }
}
