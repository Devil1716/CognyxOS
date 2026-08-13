//! Phase 4 universal capability contracts.  This crate deliberately contains no
//! Agent Kernel or OS-specific planner logic.

pub mod adapters;
pub mod browser;
pub mod layer;
pub mod model;
pub mod native;
pub mod provider;
pub mod registry;
#[cfg(target_os = "windows")]
pub mod windows_providers;

pub use adapters::{
    AdapterProvider, CapabilityAdapter, ContainerCapabilityAdapter, LinuxCapabilityAdapter,
    LocalFilesystemProvider, MacOSCapabilityAdapter, WindowsCapabilityAdapter,
};
pub use browser::UniversalBrowserProvider;
pub use layer::UniversalCapabilityLayer;
pub use model::*;
#[cfg(target_os = "windows")]
pub use native::WindowsClipboardProvider;
pub use native::{
    ApplicationRegistry, Browser, BrowserElement, BrowserProvider, BrowserResult, BrowserSession,
    NativeApplicationProvider, NativeProcessProvider, NativeTerminalProvider, VisionElement,
    VisionRequest, VisionResult,
};
pub use provider::*;
pub use registry::CapabilityRegistry;
#[cfg(target_os = "windows")]
pub use windows_providers::{
    WindowsKeyboardProvider, WindowsMouseProvider, WindowsScreenCaptureProvider,
    WindowsWindowProvider,
};
