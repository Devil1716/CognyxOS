//! CognyxOS shell (Phase 8 / Phase 12).
//!
//! The shell is the user-facing OS interface. It forwards natural-language
//! intents to the Agent Kernel and never executes capabilities itself.
//! Production wiring uses `AgentKernelAdapter` → `AgentKernelServer`.
//! `RecordingKernel` is TEST ONLY.

pub mod error;
pub mod kernel;
pub mod model;
pub mod shell;

pub use error::{ShellError, ShellResult};
pub use kernel::{AgentKernelAdapter, KernelClient, RecordingKernel};
pub use model::*;
pub use shell::CognyxShell;
