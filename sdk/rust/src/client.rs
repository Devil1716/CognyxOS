use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{error, info};

use cognyx_proto::cognyx::bus::v1::{
    MessageEnvelope, MessageType, RegisterModuleRequest, RegisterModuleResponse, Target,
};
use cognyx_proto::cognyx::common::v1::{IdentityId, Priority, Uuid};

#[derive(Error, Debug)]
pub enum BusClientError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
}

pub struct BusClient {
    pub identity_id: String,
    pub signing_key: SigningKey,
    pub socket_path: PathBuf,
}

impl BusClient {
    pub fn new(identity_id: impl Into<String>, socket_path: impl Into<PathBuf>) -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        Self {
            identity_id: identity_id.into(),
            signing_key,
            socket_path: socket_path.into(),
        }
    }

    pub fn create_envelope(
        &self,
        message_type: MessageType,
        target: Target,
        payload_bytes: Vec<u8>,
    ) -> MessageEnvelope {
        let message_id = uuid::Uuid::now_v7().to_string();
        let correlation_id = uuid::Uuid::now_v7().to_string();
        let causation_id = correlation_id.clone();

        MessageEnvelope {
            message_id: Some(Uuid { value: message_id }),
            r#type: message_type as i32,
            timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            sender: Some(IdentityId {
                value: self.identity_id.clone(),
            }),
            target: Some(target),
            capability: None,
            correlation_id: Some(Uuid {
                value: correlation_id,
            }),
            causation_id: Some(Uuid {
                value: causation_id,
            }),
            priority: Priority::Normal as i32,

            deadline: None,
            retry_policy: None,
            hop_count: 0,
            sender_signature: vec![],
            payload_size: payload_bytes.len() as u64,
            payload_encoding: 1, // PROTOBUF
            payload_checksum_sha256: vec![],
            payload: Some(
                cognyx_proto::cognyx::bus::v1::message_envelope::Payload::RawPayload(payload_bytes),
            ),
            memfd_payload: None,
            w3c_traceparent: String::new(),
            w3c_tracestate: String::new(),
        }
    }

    pub fn sign_challenge(&self, challenge: &[u8]) -> Vec<u8> {
        let signature = self.signing_key.sign(challenge);
        signature.to_bytes().to_vec()
    }
}
