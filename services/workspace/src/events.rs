use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceEvent {
    pub event_id: String,
    pub event_type: String,
    pub workspace_id: String,
    pub payload: String,
    pub timestamp_secs: u64,
}

impl WorkspaceEvent {
    pub fn new(event_type: &str, workspace_id: &str, payload: &str) -> Self {
        let evt =
            cognyx_agent_core::AgentEventPublisher::publish(event_type, workspace_id, payload);
        Self {
            event_id: evt.event_id,
            event_type: event_type.to_string(),
            workspace_id: workspace_id.to_string(),
            payload: payload.to_string(),
            timestamp_secs: crate::model::now_secs(),
        }
    }
}
