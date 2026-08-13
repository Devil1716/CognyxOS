use std::path::PathBuf;
use tracing::info;

pub struct WindowsFilesystemBridge {
    pub vm_id: String,
    pub shared_folders: Vec<(String, PathBuf)>,
}

impl WindowsFilesystemBridge {
    pub fn new(vm_id: impl Into<String>) -> Self {
        Self {
            vm_id: vm_id.into(),
            shared_folders: vec![],
        }
    }

    pub fn mount_shared_folder(&mut self, tag: impl Into<String>, host_path: PathBuf) {
        let tag_str = tag.into();
        info!(
            "Mounting virtio-fs shared folder '{}' ({:?}) to Windows VM '{}'",
            tag_str, host_path, self.vm_id
        );
        self.shared_folders.push((tag_str, host_path));
    }
}
