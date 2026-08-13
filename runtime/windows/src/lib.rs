pub mod automation;
pub mod capability;
pub mod fs_bridge;
pub mod guest_comm;
pub mod runtime;
pub mod vm_manager;

pub use automation::WindowsAppAutomation;
pub use capability::WindowsCapabilityAdapter;
pub use fs_bridge::WindowsFilesystemBridge;
pub use guest_comm::WindowsGuestCommunication;
pub use runtime::WindowsRuntime;
pub use vm_manager::WindowsVmManager;
