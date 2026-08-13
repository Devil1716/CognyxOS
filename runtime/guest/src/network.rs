use tracing::info;

pub struct GuestNetwork {
    pub vm_id: String,
}

impl GuestNetwork {
    pub fn new(vm_id: impl Into<String>) -> Self {
        Self {
            vm_id: vm_id.into(),
        }
    }

    pub async fn configure_ip(&self, ip: &str, netmask: &str) -> Result<(), String> {
        info!(
            "Setting IP {}/{} inside guest VM '{}'",
            ip, netmask, self.vm_id
        );
        Ok(())
    }
}
