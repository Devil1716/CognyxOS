use async_trait::async_trait;
use tracing::info;

pub struct WindowsGuestCommunication {
    pub vm_id: String,
    pub vsock_port: u32,
}

impl WindowsGuestCommunication {
    pub fn new(vm_id: impl Into<String>, vsock_port: u32) -> Self {
        Self {
            vm_id: vm_id.into(),
            vsock_port,
        }
    }

    pub async fn ping_guest(&self) -> Result<bool, String> {
        info!(
            "Ping guest agent for VM '{}' over vsock port {}",
            self.vm_id, self.vsock_port
        );
        Ok(true)
    }

    pub async fn send_rpc_command(&self, method: &str, payload: &[u8]) -> Result<Vec<u8>, String> {
        info!(
            "RPC call '{}' ({} bytes) to Windows guest agent '{}'",
            method,
            payload.len(),
            self.vm_id
        );
        Ok(format!("RPC Response for {}", method).into_bytes())
    }
}
