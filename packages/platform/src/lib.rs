//! Native platform abstraction boundary for future Tauri commands.

pub trait SystemInformationProvider: Send + Sync {
    fn platform_name(&self) -> &'static str;
}

pub struct WindowsSystemInformation;

impl SystemInformationProvider for WindowsSystemInformation {
    fn platform_name(&self) -> &'static str {
        "windows"
    }
}

#[cfg(test)]
mod tests {
    use super::{SystemInformationProvider, WindowsSystemInformation};

    #[test]
    fn windows_adapter_is_available() {
        assert_eq!(WindowsSystemInformation.platform_name(), "windows");
    }
}
