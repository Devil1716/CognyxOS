pub struct WindowsCapabilityAdapter;

impl WindowsCapabilityAdapter {
    pub fn translate_capability(cognyx_cap: &str) -> Option<&'static str> {
        match cognyx_cap {
            "win32.powershell" => Some("SePowerShellPrivilege"),
            "win32.filesystem" => Some("SeFileReadWritePrivilege"),
            "win32.automation" => Some("SeUiAutomationPrivilege"),
            _ => None,
        }
    }
}
