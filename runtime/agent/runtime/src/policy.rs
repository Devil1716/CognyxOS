use serde::{Serialize, Deserialize};
use crate::identity::{AgentResourceLimits, AgentIdentity};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChildLimits {
    pub max_depth: usize,
    pub max_children: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPolicy {
    pub allowed_capabilities: Vec<String>,
    pub denied_capabilities: Vec<String>,
    pub approval_required: Vec<String>,
    pub network_policy: String,
    pub filesystem_scope: Vec<String>,
    pub resource_limits: AgentResourceLimits,
    pub child_agent_limits: AgentChildLimits,
    pub communication_policy: String,
}

pub fn evaluate_permission_inheritance(parent: &AgentIdentity, requested_capability: &str) -> bool {
    parent.capabilities.contains(&requested_capability.to_string()) || parent.capabilities.contains(&"*".to_string())
}
