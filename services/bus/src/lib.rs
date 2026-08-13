use async_trait::async_trait;
use cognyx_proto::cognyx::bus::v1::message_bus_service_server::{
    MessageBusService, MessageBusServiceServer,
};
use cognyx_proto::cognyx::bus::v1::*;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::info;

pub mod auth;
pub mod router;

pub use auth::BusAuthenticator;
pub use router::BusRouter;

pub struct MessageBusDaemon {
    pub router: Arc<BusRouter>,
    pub auth: Arc<BusAuthenticator>,
}

impl Default for MessageBusDaemon {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageBusDaemon {
    pub fn new() -> Self {
        Self {
            router: Arc::new(BusRouter::new()),
            auth: Arc::new(BusAuthenticator::new()),
        }
    }
}

#[async_trait]
impl MessageBusService for MessageBusDaemon {
    type StreamMessagesStream = ReceiverStream<Result<MessageEnvelope, Status>>;
    type SubscribeStream = ReceiverStream<Result<MessageEnvelope, Status>>;
    type WatchCommandStream = ReceiverStream<Result<MessageEnvelope, Status>>;

    async fn register_module(
        &self,
        request: Request<RegisterModuleRequest>,
    ) -> Result<Response<RegisterModuleResponse>, Status> {
        let req = request.into_inner();
        let identity = req
            .identity
            .ok_or_else(|| Status::invalid_argument("Identity missing"))?;

        info!("Registering module '{}' on Message Bus", identity.value);
        let session_id = self.auth.register(&identity.value).await;

        Ok(Response::new(RegisterModuleResponse {
            session_id,
            bootstrap_caps: vec![],
            server_time: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            heartbeat_interval_ms: 5000,
        }))
    }

    async fn send_message(
        &self,
        request: Request<MessageEnvelope>,
    ) -> Result<Response<SendReceipt>, Status> {
        let envelope = request.into_inner();
        let msg_id = envelope
            .message_id
            .as_ref()
            .map(|u| u.value.clone())
            .unwrap_or_default();

        match self.router.route_envelope(envelope).await {
            Ok(_) => Ok(Response::new(SendReceipt {
                receipt_id: Some(cognyx_proto::cognyx::common::v1::Uuid { value: msg_id }),
                bus_offset: 0,
                received_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            })),
            Err(e) => Err(Status::internal(e)),
        }
    }

    async fn stream_messages(
        &self,
        request: Request<tonic::Streaming<MessageEnvelope>>,
    ) -> Result<Response<Self::StreamMessagesStream>, Status> {
        let mut in_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(100);

        let router = self.router.clone();

        tokio::spawn(async move {
            while let Ok(Some(envelope)) = in_stream.message().await {
                if let Some(sender) = &envelope.sender {
                    router.register_module_channel(sender.value.clone(), tx.clone());
                }
                let _ = router.route_envelope(envelope).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(100);
        let subscriber_id = format!("sub-{}", uuid::Uuid::now_v7());

        for topic in req.topic_patterns {
            self.router
                .subscribe_topic(subscriber_id.clone(), topic, tx.clone())
                .await;
        }

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn unsubscribe(
        &self,
        request: Request<UnsubscribeRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        let subscriber_id = format!("sub-{}", uuid::Uuid::now_v7());

        for topic in req.topic_patterns {
            self.router.unsubscribe_topic(&subscriber_id, &topic).await;
        }

        Ok(Response::new(()))
    }

    async fn submit_command(
        &self,
        request: Request<MessageEnvelope>,
    ) -> Result<Response<CommandHandle>, Status> {
        let envelope = request.into_inner();
        let cmd_id = self.router.submit_command(envelope).await;

        Ok(Response::new(CommandHandle {
            command_id: Some(cognyx_proto::cognyx::common::v1::Uuid { value: cmd_id }),
        }))
    }

    async fn get_command_status(
        &self,
        request: Request<CommandHandle>,
    ) -> Result<Response<CommandStatus>, Status> {
        let handle = request.into_inner();
        let cmd_id = handle
            .command_id
            .ok_or_else(|| Status::invalid_argument("Command ID missing"))?
            .value;

        let status_enum = self
            .router
            .get_command_status(&cmd_id)
            .await
            .unwrap_or(router::CommandStatusEnum::Pending);

        let code = match status_enum {
            router::CommandStatusEnum::Pending => CommandState::Queued as i32,
            router::CommandStatusEnum::Processing => CommandState::Executing as i32,
            router::CommandStatusEnum::Completed => CommandState::Completed as i32,
            router::CommandStatusEnum::Failed(_) => CommandState::Failed as i32,
        };

        Ok(Response::new(CommandStatus {
            command_id: Some(cognyx_proto::cognyx::common::v1::Uuid { value: cmd_id }),
            state: code,
            error: None,
            result: None,
            progress: vec![],
            submitted_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            completed_at: None,
        }))
    }

    async fn cancel_command(
        &self,
        _request: Request<CommandHandle>,
    ) -> Result<Response<()>, Status> {
        Ok(Response::new(()))
    }

    async fn watch_command(
        &self,
        _request: Request<CommandHandle>,
    ) -> Result<Response<Self::WatchCommandStream>, Status> {
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn ping(&self, request: Request<PingRequest>) -> Result<Response<PongResponse>, Status> {
        let req = request.into_inner();

        Ok(Response::new(PongResponse {
            nonce_echo: req.nonce,
            server_time: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        }))
    }

    async fn list_dead_letter_messages(
        &self,
        _request: Request<DlqListRequest>,
    ) -> Result<Response<DlqListResponse>, Status> {
        let dead_letters = self.router.get_dead_letters().await;

        let messages = dead_letters
            .into_iter()
            .enumerate()
            .map(|(i, env)| DeadLetterMessage {
                dlq_id: (i + 1) as u64,
                envelope: Some(env),
                last_error: None,
                attempts: 1,
                first_failed_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            })
            .collect();

        Ok(Response::new(DlqListResponse {
            messages,
            pagination: None,
        }))
    }

    async fn replay_dead_letter_message(
        &self,
        request: Request<DlqReplayRequest>,
    ) -> Result<Response<()>, Status> {
        let dlq_id = request.into_inner().dlq_id;
        info!("Replaying Dead Letter message ID {}", dlq_id);
        Ok(Response::new(()))
    }
}
