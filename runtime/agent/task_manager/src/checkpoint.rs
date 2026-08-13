use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckpointState {
    pub checkpoint_id: String,
    pub task_id: String,
    pub task_state: String,
    pub execution_graph_id: String,
    pub completed_node_ids: Vec<String>,
    pub pending_node_ids: Vec<String>,
    pub current_node_id: Option<String>,
    pub assigned_runtime: Option<String>,
    pub node_outputs: HashMap<String, String>,
    pub context_params: HashMap<String, String>,
    pub permissions_granted: Vec<String>,
    pub timestamp_ms: u64,
}

pub struct CheckpointEngine;

impl CheckpointEngine {
    pub fn create_checkpoint(
        task_id: &str,
        task_state: &str,
        graph_id: &str,
        completed_nodes: Vec<String>,
        pending_nodes: Vec<String>,
        current_node: Option<String>,
        assigned_runtime: Option<String>,
        node_outputs: HashMap<String, String>,
    ) -> CheckpointState {
        CheckpointState {
            checkpoint_id: format!("chk-{}", uuid::Uuid::now_v7()),
            task_id: task_id.to_string(),
            task_state: task_state.to_string(),
            execution_graph_id: graph_id.to_string(),
            completed_node_ids: completed_nodes,
            pending_node_ids: pending_nodes,
            current_node_id: current_node,
            assigned_runtime,
            node_outputs,
            context_params: HashMap::new(),
            permissions_granted: vec![],
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}
