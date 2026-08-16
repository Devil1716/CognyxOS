//! Phase 9 distributed execution.
//!
//! Workers are local-registry only. WAN NOT VERIFIED.
//!
//! Workers advertise OS/compute. `WorkerRegistry` registers them with the
//! existing `RuntimeRegistry`. It does not replace it.

mod model;
mod registry;
mod remote;
mod transfer;

pub use model::*;
pub use registry::WorkerRegistry;
pub use remote::RemoteWorkerRuntime;
pub use transfer::{ArtifactBlob, ArtifactTransfer};

pub use cognyx_execution::{RuntimeKind, RuntimeRegistry, RuntimeStatus};
pub use cognyx_task_manager::{CheckpointEngine, CheckpointState};
