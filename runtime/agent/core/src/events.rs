use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentBusEvent {
    pub event_id: String,
    pub event_type: String,
    pub task_id: String,
    pub payload: String,
    pub timestamp_ms: u64,
}

pub struct AgentEventPublisher;

impl AgentEventPublisher {
    pub fn publish(event_type: &str, task_id: &str, payload: &str) -> AgentBusEvent {
        let event_id = format!("evt-{}", uuid::Uuid::now_v7());
        info!(
            "Emitting Agent Bus Event [{}] for task '{}': {}",
            event_type, task_id, payload
        );

        AgentBusEvent {
            event_id,
            event_type: event_type.to_string(),
            task_id: task_id.to_string(),
            payload: payload.to_string(),
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}
