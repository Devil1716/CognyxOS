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

pub struct CheckpointRequest {
    pub task_id: String,
    pub task_state: String,
    pub graph_id: String,
    pub completed_nodes: Vec<String>,
    pub pending_nodes: Vec<String>,
    pub current_node: Option<String>,
    pub assigned_runtime: Option<String>,
    pub node_outputs: HashMap<String, String>,
}

pub struct CheckpointEngine;

impl CheckpointEngine {
    pub fn create_checkpoint(request: CheckpointRequest) -> CheckpointState {
        CheckpointState {
            checkpoint_id: format!("chk-{}", uuid::Uuid::now_v7()),
            task_id: request.task_id,
            task_state: request.task_state,
            execution_graph_id: request.graph_id,
            completed_node_ids: request.completed_nodes,
            pending_node_ids: request.pending_nodes,
            current_node_id: request.current_node,
            assigned_runtime: request.assigned_runtime,
            node_outputs: request.node_outputs,
            context_params: HashMap::new(),
            permissions_granted: vec![],
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}
