use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct ModuleSession {
    pub session_id: Vec<u8>,
    pub identity: String,
    pub verifying_key: Option<VerifyingKey>,
    pub registered_at: std::time::Instant,
    pub last_ping: std::time::Instant,
}

#[derive(Default)]
pub struct AuthManager {
    sessions: Arc<RwLock<HashMap<String, ModuleSession>>>,
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn generate_nonce() -> Vec<u8> {
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce);
        nonce.to_vec()
    }

    pub fn verify_signature(
        public_key_bytes: &[u8],
        nonce: &[u8],
        signature_bytes: &[u8],
    ) -> Result<VerifyingKey, String> {
        let key_bytes: [u8; 32] = public_key_bytes
            .try_into()
            .map_err(|_| "Invalid public key length; expected 32 bytes")?;

        let verifying_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| format!("Invalid public key: {}", e))?;

        let sig_bytes: [u8; 64] = signature_bytes
            .try_into()
            .map_err(|_| "Invalid signature length; expected 64 bytes")?;
        let signature = Signature::from_bytes(&sig_bytes);

        verifying_key
            .verify(nonce, &signature)
            .map_err(|e| format!("Signature verification failed: {}", e))?;

        Ok(verifying_key)
    }

    pub async fn register_session(
        &self,
        identity: String,
        verifying_key: Option<VerifyingKey>,
    ) -> Vec<u8> {
        let session_id = Self::generate_nonce();
        let session = ModuleSession {
            session_id: session_id.clone(),
            identity: identity.clone(),
            verifying_key,
            registered_at: std::time::Instant::now(),
            last_ping: std::time::Instant::now(),
        };

        let mut lock = self.sessions.write().await;
        lock.insert(identity, session);
        session_id
    }

    pub async fn is_session_active(&self, identity: &str) -> bool {
        let lock = self.sessions.read().await;
        lock.contains_key(identity)
    }

    pub async fn update_ping(&self, identity: &str) -> bool {
        let mut lock = self.sessions.write().await;
        if let Some(session) = lock.get_mut(identity) {
            session.last_ping = std::time::Instant::now();
            true
        } else {
            false
        }
    }

    pub async fn register(&self, identity: &str) -> Vec<u8> {
        self.register_session(identity.to_string(), None).await
    }
}

pub type BusAuthenticator = AuthManager;
