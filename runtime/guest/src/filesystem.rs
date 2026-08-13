use tracing::info;

pub struct GuestFileSystem {
    pub vm_id: String,
}

impl GuestFileSystem {
    pub fn new(vm_id: impl Into<String>) -> Self {
        Self {
            vm_id: vm_id.into(),
        }
    }

    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        info!("Reading file '{}' inside guest VM '{}'", path, self.vm_id);
        Ok(format!("Content of {}", path).into_bytes())
    }

    pub async fn write_file(&self, path: &str, content: &[u8]) -> Result<(), String> {
        info!(
            "Writing {} bytes to file '{}' inside guest VM '{}'",
            content.len(),
            path,
            self.vm_id
        );
        Ok(())
    }
}
