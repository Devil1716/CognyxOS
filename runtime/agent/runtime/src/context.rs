use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextVisibility {
    AgentPrivate,
    TaskShared,
    System,
}

#[derive(Debug, Clone)]
pub struct AgentTaskContext {
    pub private_context: serde_json::Value,
    pub task_shared_context: serde_json::Value,
    pub system_info: serde_json::Value,
}

impl AgentTaskContext {
    pub fn new() -> Self {
        Self {
            private_context: serde_json::Value::Null,
            task_shared_context: serde_json::Value::Null,
            system_info: serde_json::Value::Null,
        }
    }
}
