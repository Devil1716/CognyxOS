use tracing::info;

pub struct GuestAutomation {
    pub vm_id: String,
}

impl GuestAutomation {
    pub fn new(vm_id: impl Into<String>) -> Self {
        Self {
            vm_id: vm_id.into(),
        }
    }

    pub async fn send_keystrokes(&self, text: &str) -> Result<(), String> {
        info!("Sending keystrokes '{}' to guest VM '{}'", text, self.vm_id);
        Ok(())
    }
}
