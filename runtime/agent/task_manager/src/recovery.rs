use crate::manager::AgentTask;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecoveryAction {
    RetryWithBackoff { delay_ms: u64 },
    RuntimeSwitch { new_runtime_kind: String },
    Replan,
    PauseForUserIntervention,
    FailSafe,
}

pub struct RecoveryEngine;

impl RecoveryEngine {
    pub fn evaluate_failure(
        task: &AgentTask,
        failed_runtime: &str,
        error_msg: &str,
    ) -> RecoveryAction {
        info!(
            "RecoveryEngine evaluating failure for task '{}' (runtime: '{}'): {}",
            task.task_id, failed_runtime, error_msg
        );

        if task.retry_count < 3 {
            let delay_ms = 100 * (2u64.pow(task.retry_count));
            RecoveryAction::RetryWithBackoff { delay_ms }
        } else if failed_runtime.contains("Windows") {
            info!(
                "Windows runtime failure detected. Triggering dynamic runtime switch / replanning"
            );
            RecoveryAction::RuntimeSwitch {
                new_runtime_kind: "RemoteWindowsWorker".to_string(),
            }
        } else {
            RecoveryAction::Replan
        }
    }
}
