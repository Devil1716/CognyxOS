use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum AgentLifecycleState {
    Created,
    Initializing,
    Ready,
    Running,
    Waiting,
    Paused,
    Blocked,
    Failed(String),
    Recovering,
    Stopping,
    Stopped,
    Terminated,
}

pub struct AgentEventPublisher;

impl Default for AgentEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentEventPublisher {
    pub fn new() -> Self {
        Self
    }
    pub fn publish(&self, _event: &str) {}
}

pub struct AgentLifecycleManager {
    publisher: Arc<AgentEventPublisher>,
    states: Arc<Mutex<HashMap<String, AgentLifecycleState>>>,
}

impl Default for AgentLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentLifecycleManager {
    pub fn new() -> Self {
        Self {
            publisher: Arc::new(AgentEventPublisher::new()),
            states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_publisher(publisher: Arc<AgentEventPublisher>) -> Self {
        Self {
            publisher,
            states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn transition(
        &self,
        agent_id: &str,
        new_state: AgentLifecycleState,
    ) -> Result<(), String> {
        let mut states = self.states.lock().await;
        states.insert(agent_id.to_string(), new_state);
        self.publisher
            .publish(&format!("Agent {} state changed", agent_id));
        Ok(())
    }
}
