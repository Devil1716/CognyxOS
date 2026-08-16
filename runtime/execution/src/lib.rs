pub mod registry;
pub mod runtimes;
pub mod traits;

pub use registry::RuntimeRegistry;
pub use runtimes::LinuxRuntime;
pub use traits::{ExecutionRuntime, RuntimeInfo, RuntimeKind, RuntimeStatus};

/// Runtime identity of the native host OS. Providers and registries must use
/// this (or a RuntimeRegistry id that matches it) instead of a hardcoded
/// `host-linux-1` label on Windows.
pub fn native_host_runtime_id() -> &'static str {
    match std::env::consts::OS {
        "windows" => "windows-host",
        "macos" => "macos-host",
        _ => "linux-host-1",
    }
}

pub fn native_host_runtime_name() -> &'static str {
    match std::env::consts::OS {
        "windows" => "Host Windows",
        "macos" => "Host macOS",
        _ => "Host Linux",
    }
}
