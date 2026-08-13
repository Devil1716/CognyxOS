use crate::checkpoint::{CheckpointEngine, CheckpointState};
use crate::store::AgentStateStore;
use cognyx_intent::{IntentEngine, ParsedIntent};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Created,
    Planning,
    Ready,
    Running,
    Waiting,
    Blocked,
    Paused,
    Failed(String),
    Recovering,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentTask {
    pub task_id: String,
    pub intent_id: String,
    pub parent_task_id: Option<String>,
    pub status: TaskStatus,
    pub priority: u32,
    pub constraints: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub plan_id: Option<String>,
    pub execution_graph_id: Option<String>,
    pub assigned_runtime: Option<String>,
    pub checkpoint: Option<CheckpointState>,
    pub result: Option<String>,
    pub error: Option<String>,
    pub prompt: String,
    pub intent: ParsedIntent,
    pub retry_count: u32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Task not found: {0}")]
    NotFound(String),
}

pub struct AgentTaskManager {
    intent_engine: IntentEngine,
    store: Arc<AgentStateStore>,
}

impl Default for AgentTaskManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentTaskManager {
    pub fn new() -> Self {
        Self {
            intent_engine: IntentEngine::default(),
            store: Arc::new(AgentStateStore::default()),
        }
    }

    pub async fn submit_task(&self, prompt: &str) -> AgentTask {
        let task_id = format!("task-{}", uuid::Uuid::now_v7());
        info!(
            "Submitting agent task '{}' for prompt: '{}'",
            task_id, prompt
        );

        let intent = self.intent_engine.parse_prompt(prompt).await;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let task = AgentTask {
            task_id: task_id.clone(),
            intent_id: intent.intent_id.clone(),
            parent_task_id: None,
            status: TaskStatus::Created,
            priority: 5,
            constraints: vec![],
            required_capabilities: intent.required_capabilities.clone(),
            plan_id: None,
            execution_graph_id: None,
            assigned_runtime: None,
            checkpoint: None,
            result: None,
            error: None,
            prompt: prompt.to_string(),
            intent,
            retry_count: 0,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        };

        self.store.save_task(&task);
        task
    }

    pub fn update_status(&self, task_id: &str, status: TaskStatus) -> Result<AgentTask, TaskError> {
        let mut task = self
            .store
            .get_task(task_id)
            .ok_or_else(|| TaskError::NotFound(task_id.to_string()))?;
        info!(
            "Updating status of agent task '{}' to {:?}",
            task_id, status
        );
        task.status = status;
        task.updated_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.store.save_task(&task);
        Ok(task)
    }

    pub fn create_checkpoint(
        &self,
        task_id: &str,
        graph_id: &str,
        completed: Vec<String>,
        pending: Vec<String>,
        outputs: std::collections::HashMap<String, String>,
    ) -> Result<AgentTask, TaskError> {
        let mut task = self
            .store
            .get_task(task_id)
            .ok_or_else(|| TaskError::NotFound(task_id.to_string()))?;
        let chk = CheckpointEngine::create_checkpoint(
            task_id,
            &format!("{:?}", task.status),
            graph_id,
            completed,
            pending,
            None,
            task.assigned_runtime.clone(),
            outputs,
        );
        task.checkpoint = Some(chk);
        self.store.save_task(&task);
        Ok(task)
    }

    pub fn get_task(&self, task_id: &str) -> Result<AgentTask, TaskError> {
        self.store
            .get_task(task_id)
            .ok_or_else(|| TaskError::NotFound(task_id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_manager_lifecycle() {
        let manager = AgentTaskManager::new();

        let task = manager.submit_task("Install Photoshop").await;
        assert_eq!(task.status, TaskStatus::Created);

        let planned = manager
            .update_status(&task.task_id, TaskStatus::Planning)
            .unwrap();
        assert_eq!(planned.status, TaskStatus::Planning);

        let running = manager
            .update_status(&task.task_id, TaskStatus::Running)
            .unwrap();
        assert_eq!(running.status, TaskStatus::Running);

        let completed = manager
            .update_status(&task.task_id, TaskStatus::Completed)
            .unwrap();
        assert_eq!(completed.status, TaskStatus::Completed);
    }
}
