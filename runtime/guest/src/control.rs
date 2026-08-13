use tracing::info;

pub struct GuestControl {
    pub vm_id: String,
}

impl GuestControl {
    pub fn new(vm_id: impl Into<String>) -> Self {
        Self {
            vm_id: vm_id.into(),
        }
    }

    pub async fn shutdown_guest(&self) -> Result<(), String> {
        info!(
            "Sending shutdown command to guest agent for VM '{}'",
            self.vm_id
        );
        Ok(())
    }

    pub async fn reboot_guest(&self) -> Result<(), String> {
        info!(
            "Sending reboot command to guest agent for VM '{}'",
            self.vm_id
        );
        Ok(())
    }
}
