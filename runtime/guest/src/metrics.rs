use tracing::info;

pub struct GuestMetrics {
    pub vm_id: String,
}

impl GuestMetrics {
    pub fn new(vm_id: impl Into<String>) -> Self {
        Self {
            vm_id: vm_id.into(),
        }
    }

    pub async fn fetch_guest_telemetry(&self) -> Result<String, String> {
        info!(
            "Fetching in-guest telemetry metrics for VM '{}'",
            self.vm_id
        );
        Ok("cpu=5.2%,mem=2048MB,load=0.10".to_string())
    }
}
