use async_trait::async_trait;
use tracing::info;

#[async_trait]
pub trait MacOSExecutionBackend: Send + Sync {
    async fn start_mac(&self) -> Result<(), String>;
    async fn stop_mac(&self) -> Result<(), String>;
    async fn execute(&self, command: &str) -> Result<String, String>;
    fn backend_type(&self) -> &'static str;
}

pub struct LocalMacBackend {
    pub is_apple_hardware: bool,
}

impl LocalMacBackend {
    pub fn new() -> Self {
        Self {
            is_apple_hardware: std::cfg!(target_os = "macos"),
        }
    }
}

impl Default for LocalMacBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MacOSExecutionBackend for LocalMacBackend {
    async fn start_mac(&self) -> Result<(), String> {
        if !self.is_apple_hardware {
            return Err(
                "Local macOS virtualization is legally & technically restricted to Apple hardware."
                    .to_string(),
            );
        }
        info!("Starting Local macOS VM via Virtualization.framework");
        Ok(())
    }

    async fn stop_mac(&self) -> Result<(), String> {
        info!("Stopping Local macOS VM");
        Ok(())
    }

    async fn execute(&self, command: &str) -> Result<String, String> {
        if !self.is_apple_hardware {
            return Err(
                "Local macOS virtualization unavailable on non-Apple hardware.".to_string(),
            );
        }
        Ok(format!("Local Mac executed: {}", command))
    }

    fn backend_type(&self) -> &'static str {
        "local-apple-hardware"
    }
}

pub struct RemoteMacBackend {
    pub remote_host: String,
}

impl RemoteMacBackend {
    pub fn new(remote_host: impl Into<String>) -> Self {
        Self {
            remote_host: remote_host.into(),
        }
    }
}

#[async_trait]
impl MacOSExecutionBackend for RemoteMacBackend {
    async fn start_mac(&self) -> Result<(), String> {
        info!(
            "Connecting to remote macOS worker host at '{}'",
            self.remote_host
        );
        Ok(())
    }

    async fn stop_mac(&self) -> Result<(), String> {
        info!(
            "Disconnecting from remote macOS worker host at '{}'",
            self.remote_host
        );
        Ok(())
    }

    async fn execute(&self, command: &str) -> Result<String, String> {
        info!(
            "Delegating command '{}' to remote Mac worker '{}'",
            command, self.remote_host
        );
        Ok(format!(
            "Remote Mac ({}) output for: {}",
            self.remote_host, command
        ))
    }

    fn backend_type(&self) -> &'static str {
        "remote-mac-worker"
    }
}
