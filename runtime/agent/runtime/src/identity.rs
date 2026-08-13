use serde::{Serialize, Deserialize};
use crate::role::AgentRole;
use crate::lifecycle::AgentLifecycleState;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentPriority {
    Critical,
    High,
    Normal,
    Low,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentModelConfig {
    pub model_provider: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,
    pub context_limit: usize,
    pub timeout_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResourceLimits {
    pub cpu_quota_pct: f32,
    pub memory_bytes: u64,
    pub max_child_agents: usize,
    pub max_concurrent_tasks: usize,
    pub max_retries: u32,
    pub max_message_rate: u32,
    pub timeout_seconds: u32,
}

impl Default for AgentResourceLimits {
    fn default() -> Self {
        Self {
            cpu_quota_pct: 100.0,
            memory_bytes: 1024 * 1024 * 1024, // 1GB
            max_child_agents: 8,
            max_concurrent_tasks: 10,
            max_retries: 3,
            max_message_rate: 100,
            timeout_seconds: 3600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub agent_id: String,
    pub parent_agent_id: Option<String>,
    pub root_agent_id: String,
    pub name: String,
    pub display_name: String,
    pub role: AgentRole,
    pub status: AgentLifecycleState,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub stopped_at: Option<u64>,
    pub permissions: Vec<String>,
    pub capabilities: Vec<String>,
    pub resource_limits: AgentResourceLimits,
    pub metadata: serde_json::Value,
}

impl AgentIdentity {
    pub fn new(
        agent_id: impl Into<String>,
        name: impl Into<String>,
        display_name: impl Into<String>,
        role: AgentRole,
        parent_agent_id: Option<String>,
        root_agent_id: impl Into<String>,
    ) -> Self {
        let aid = agent_id.into();
        let rid = root_agent_id.into();
        let root = match parent_agent_id {
            Some(_) => if rid.is_empty() { aid.clone() } else { rid },
            None => aid.clone(),
        };
        let caps = role.allowed_capabilities();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            agent_id: aid,
            parent_agent_id,
            root_agent_id: root,
            name: name.into(),
            display_name: display_name.into(),
            role,
            status: AgentLifecycleState::Created,
            created_at: now,
            started_at: None,
            stopped_at: None,
            permissions: caps.clone(),
            capabilities: caps,
            resource_limits: AgentResourceLimits::default(),
            metadata: serde_json::json!({}),
        }
    }
}
