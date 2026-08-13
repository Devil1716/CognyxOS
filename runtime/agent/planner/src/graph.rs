use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TargetEnvironment {
    NativeLinux,
    WindowsVm,
    MacOsVm,
    Container,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeState {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionNode {
    pub node_id: String,
    pub task_id: String,
    pub name: String,
    pub target_env: TargetEnvironment,
    pub command: String,
    pub args: Vec<String>,
    pub depends_on: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub constraints: HashMap<String, String>,
    pub state: NodeState,
    pub runtime_requirements: Vec<String>,
    pub timeout_seconds: u32,
    pub retry_policy_max_retries: u32,
    pub env_vars: HashMap<String, String>,
    pub input_payload: String,
    pub output_result: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionGraph {
    pub graph_id: String,
    pub task_id: String,
    pub nodes: Vec<ExecutionNode>,
}

impl ExecutionGraph {
    pub fn new(graph_id: impl Into<String>, task_id: impl Into<String>) -> Self {
        Self {
            graph_id: graph_id.into(),
            task_id: task_id.into(),
            nodes: vec![],
        }
    }

    pub fn add_node(&mut self, node: ExecutionNode) {
        self.nodes.push(node);
    }

    pub fn mark_node_state(&mut self, node_id: &str, state: NodeState, output: Option<String>) {
        if let Some(n) = self.nodes.iter_mut().find(|n| n.node_id == node_id) {
            n.state = state;
            if output.is_some() {
                n.output_result = output;
            }
        }
    }
}
