use dashmap::DashMap;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

use cognyx_proto::cognyx::bus::v1::target::Target as TargetTarget;
use cognyx_proto::cognyx::bus::v1::{MessageEnvelope, MessageType, Target};

pub type EnvelopeSender = mpsc::Sender<Result<MessageEnvelope, tonic::Status>>;

#[derive(Clone, Debug)]
pub struct CommandRecord {
    pub command_id: String,
    pub sender_id: String,
    pub target_id: String,
    pub envelope: MessageEnvelope,
    pub status: CommandStatusEnum,
    pub created_at: std::time::Instant,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CommandStatusEnum {
    Pending,
    Processing,
    Completed,
    Failed(String),
}

pub type BusRouter = MessageRouter;

pub struct MessageRouter {
    // Topic subscriptions: topic_name -> list of subscriber tx channels
    subscriptions: Arc<RwLock<HashMap<String, Vec<(String, EnvelopeSender)>>>>,
    // Unicast module streams: module_identity -> tx channel
    module_channels: DashMap<String, EnvelopeSender>,
    // In-flight commands: command_id -> CommandRecord
    commands: Arc<RwLock<HashMap<String, CommandRecord>>>,
    // Dead Letter Queue
    dead_letter_queue: Arc<RwLock<VecDeque<MessageEnvelope>>>,
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            module_channels: DashMap::new(),
            commands: Arc::new(RwLock::new(HashMap::new())),
            dead_letter_queue: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
        }
    }

    pub fn register_module_channel(&self, identity: String, sender: EnvelopeSender) {
        info!("Registering active message stream for module: {}", identity);
        self.module_channels.insert(identity, sender);
    }

    pub fn unregister_module_channel(&self, identity: &str) {
        info!("Unregistering message stream for module: {}", identity);
        self.module_channels.remove(identity);
    }

    pub async fn subscribe_topic(
        &self,
        subscriber_id: String,
        topic: String,
        sender: EnvelopeSender,
    ) {
        let mut lock = self.subscriptions.write().await;
        info!("Module '{}' subscribed to topic '{}'", subscriber_id, topic);
        lock.entry(topic).or_default().push((subscriber_id, sender));
    }

    pub async fn unsubscribe_topic(&self, subscriber_id: &str, topic: &str) {
        let mut lock = self.subscriptions.write().await;
        if let Some(subscribers) = lock.get_mut(topic) {
            subscribers.retain(|(id, _)| id != subscriber_id);
            info!(
                "Module '{}' unsubscribed from topic '{}'",
                subscriber_id, topic
            );
        }
    }

    pub async fn route_envelope(&self, envelope: MessageEnvelope) -> Result<(), String> {
        let msg_type = envelope.r#type;

        if let Some(target) = envelope.target.clone() {
            match target.target {
                Some(TargetTarget::Module(identity_id)) => {
                    let target_module = identity_id.value;
                    if let Some(sender) = self.module_channels.get(&target_module) {
                        sender.send(Ok(envelope)).await.map_err(|e| e.to_string())?;
                        return Ok(());
                    } else {
                        warn!(
                            "Target module '{}' not connected. Stashing in DLQ.",
                            target_module
                        );
                        self.push_dead_letter(envelope).await;
                        return Err(format!("Target module '{}' unavailable", target_module));
                    }
                }
                Some(TargetTarget::Topic(topic)) => {
                    self.publish_event(&topic, envelope).await;
                    return Ok(());
                }

                Some(TargetTarget::BroadcastAll(_)) => {
                    for entry in self.module_channels.iter() {
                        let _ = entry.value().send(Ok(envelope.clone())).await;
                    }
                    return Ok(());
                }
                Some(TargetTarget::WorkspaceBroadcast(_ws_id)) => {
                    for entry in self.module_channels.iter() {
                        let _ = entry.value().send(Ok(envelope.clone())).await;
                    }
                    return Ok(());
                }
                None => {
                    self.push_dead_letter(envelope).await;
                    return Err("Target specifier missing in envelope".to_string());
                }
            }
        } else {
            self.push_dead_letter(envelope).await;
            Err("No target provided in message envelope".to_string())
        }
    }

    pub async fn publish_event(&self, topic: &str, envelope: MessageEnvelope) {
        let lock = self.subscriptions.read().await;
        if let Some(subscribers) = lock.get(topic) {
            info!(
                "Publishing event to topic '{}' ({} subscribers)",
                topic,
                subscribers.len()
            );
            for (_sub_id, sender) in subscribers {
                let _ = sender.send(Ok(envelope.clone())).await;
            }
        } else {
            info!("No active subscribers for topic '{}'", topic);
        }
    }

    pub async fn submit_command(&self, envelope: MessageEnvelope) -> String {
        let command_id = envelope
            .message_id
            .as_ref()
            .map(|u| u.value.clone())
            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());

        let sender_id = envelope
            .sender
            .as_ref()
            .map(|s| s.value.clone())
            .unwrap_or_default();

        let target_id = match &envelope.target {
            Some(Target {
                target: Some(TargetTarget::Module(id)),
            }) => id.value.clone(),
            _ => String::new(),
        };

        let record = CommandRecord {
            command_id: command_id.clone(),
            sender_id,
            target_id,
            envelope: envelope.clone(),
            status: CommandStatusEnum::Pending,
            created_at: std::time::Instant::now(),
        };

        {
            let mut lock = self.commands.write().await;
            lock.insert(command_id.clone(), record);
        }

        let _ = self.route_envelope(envelope).await;
        command_id
    }

    pub async fn get_command_status(&self, command_id: &str) -> Option<CommandStatusEnum> {
        let lock = self.commands.read().await;
        lock.get(command_id).map(|r| r.status.clone())
    }

    pub async fn push_dead_letter(&self, envelope: MessageEnvelope) {
        let mut lock = self.dead_letter_queue.write().await;
        if lock.len() >= 1000 {
            lock.pop_front();
        }
        lock.push_back(envelope);
    }

    pub async fn get_dead_letters(&self) -> Vec<MessageEnvelope> {
        let lock = self.dead_letter_queue.read().await;
        lock.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cognyx_proto::cognyx::common::v1::IdentityId;

    #[tokio::test]
    async fn test_pub_sub_routing() {
        let router = MessageRouter::new();
        let (tx, mut rx) = mpsc::channel(10);

        router
            .subscribe_topic("sub1".to_string(), "workspace.created".to_string(), tx)
            .await;

        let envelope = MessageEnvelope {
            message_id: Some(cognyx_proto::cognyx::common::v1::Uuid {
                value: "test-msg-1".to_string(),
            }),
            r#type: MessageType::Event as i32,
            timestamp: None,
            sender: Some(IdentityId {
                value: "workspace-mgr".to_string(),
            }),
            target: Some(Target {
                target: Some(TargetTarget::Topic("workspace.created".to_string())),
            }),
            capability: None,
            correlation_id: None,
            causation_id: None,
            priority: 2,
            deadline: None,
            retry_policy: None,
            hop_count: 0,
            sender_signature: vec![],
            payload_size: 0,
            payload_encoding: 1,
            payload_checksum_sha256: vec![],
            payload: None,
            memfd_payload: None,
            w3c_traceparent: String::new(),
            w3c_tracestate: String::new(),
        };

        router.route_envelope(envelope).await.unwrap();

        let received = rx.recv().await.unwrap().unwrap();
        assert_eq!(received.message_id.unwrap().value, "test-msg-1");
    }

    #[tokio::test]
    async fn test_dead_letter_on_missing_target() {
        let router = MessageRouter::new();

        let envelope = MessageEnvelope {
            message_id: Some(cognyx_proto::cognyx::common::v1::Uuid {
                value: "test-msg-dlq".to_string(),
            }),
            r#type: MessageType::Command as i32,
            timestamp: None,
            sender: Some(IdentityId {
                value: "cli".to_string(),
            }),
            target: Some(Target {
                target: Some(TargetTarget::Module(IdentityId {
                    value: "nonexistent-module".to_string(),
                })),
            }),
            capability: None,
            correlation_id: None,
            causation_id: None,
            priority: 2,
            deadline: None,
            retry_policy: None,
            hop_count: 0,
            sender_signature: vec![],
            payload_size: 0,
            payload_encoding: 1,
            payload_checksum_sha256: vec![],
            payload: None,
            memfd_payload: None,
            w3c_traceparent: String::new(),
            w3c_tracestate: String::new(),
        };

        let result = router.route_envelope(envelope).await;
        assert!(result.is_err());
        assert_eq!(router.dead_letter_queue.read().await.len(), 1);
    }
}
