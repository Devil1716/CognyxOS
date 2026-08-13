use tracing::info;

pub struct GuestProcess {
    pub vm_id: String,
}

impl GuestProcess {
    pub fn new(vm_id: impl Into<String>) -> Self {
        Self {
            vm_id: vm_id.into(),
        }
    }

    pub async fn spawn_guest_process(&self, cmd: &str, args: &[&str]) -> Result<u32, String> {
        info!(
            "Spawning process '{} {:?}' inside guest VM '{}'",
            cmd, args, self.vm_id
        );
        Ok(1001) // Dummy PID inside guest
    }

    pub async fn kill_guest_process(&self, pid: u32) -> Result<(), String> {
        info!(
            "Killing process PID {} inside guest VM '{}'",
            pid, self.vm_id
        );
        Ok(())
    }
}
