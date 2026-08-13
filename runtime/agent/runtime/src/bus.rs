use serde::{Serialize, Deserialize};
use dashmap::DashMap;
use tokio::sync::broadcast;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMessageType {
    TaskAssignment,
    TaskResult,
    InformationRequest,
    InformationResponse,
    CapabilityRequest,
    StatusUpdate,
    ProgressUpdate,
    Error,
    ApprovalRequest,
    Cancel,
    Pause,
    Resume,
    Heartbeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub message_id: String,
    pub sender_agent_id: String,
    pub recipient_agent_id: String,
    pub task_id: String,
    pub timestamp: u64,
    pub message_type: AgentMessageType,
    pub payload: serde_json::Value,
    pub authorization_context: serde_json::Value,
    pub trace_id: String,
}

pub struct AgentCommunicationBus {
    routers: DashMap<String, broadcast::Sender<AgentMessage>>,
    log: Mutex<Vec<AgentMessage>>,
}

impl AgentCommunicationBus {
    pub fn new() -> Self {
        Self {
            routers: DashMap::new(),
            log: Mutex::new(Vec::new()),
        }
    }

    pub fn send_message(&self, message: AgentMessage) -> Result<(), String> {
        self.log.lock().unwrap().push(message.clone());
        if let Some(sender) = self.routers.get(&message.recipient_agent_id) {
            let _ = sender.send(message);
        }
        Ok(())
    }

    pub fn subscribe(&self, agent_id: &str) -> broadcast::Receiver<AgentMessage> {
        let sender = self.routers.entry(agent_id.to_string()).or_insert_with(|| {
            let (tx, _) = broadcast::channel(100);
            tx
        });
        sender.subscribe()
    }
}
