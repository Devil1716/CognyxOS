use async_trait::async_trait;
use cognyx_intent::ParsedIntent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanConstraint {
    pub key: String,
    pub value: String,
    pub is_hard_requirement: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanStep {
    pub step_id: String,
    pub description: String,
    pub target_runtime_kind: String,
    pub required_capabilities: Vec<String>,
    pub depends_on_step_ids: Vec<String>,
    pub parameters: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanValidationResult {
    pub is_valid: bool,
    pub validation_errors: Vec<String>,
    pub missing_capabilities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub plan_id: String,
    pub task_id: String,
    pub steps: Vec<PlanStep>,
    pub constraints: Vec<PlanConstraint>,
    pub created_at_ms: u64,
}

#[async_trait]
pub trait PlannerProvider: Send + Sync {
    fn provider_name(&self) -> &str;
    async fn create_plan(&self, task_id: &str, intent: &ParsedIntent) -> Result<Plan, String>;
}

pub struct DeterministicPlannerProvider;

#[async_trait]
impl PlannerProvider for DeterministicPlannerProvider {
    fn provider_name(&self) -> &str {
        "deterministic"
    }

    async fn create_plan(&self, task_id: &str, intent: &ParsedIntent) -> Result<Plan, String> {
        let plan_id = format!("plan-{}", uuid::Uuid::now_v7());
        let mut steps = vec![];

        match intent.domain {
            cognyx_intent::IntentDomain::AppInstallation => {
                steps.push(PlanStep {
                    step_id: "step-1".to_string(),
                    description: "Verify Windows Runtime Environment".to_string(),
                    target_runtime_kind: "WindowsVm".to_string(),
                    required_capabilities: vec!["win32.powershell".to_string()],
                    depends_on_step_ids: vec![],
                    parameters: HashMap::new(),
                });

                steps.push(PlanStep {
                    step_id: "step-2".to_string(),
                    description: format!("Execute Winget Install for {}", intent.target_object),
                    target_runtime_kind: "WindowsVm".to_string(),
                    required_capabilities: vec![
                        "package.install".to_string(),
                        "win32.powershell".to_string(),
                    ],
                    depends_on_step_ids: vec!["step-1".to_string()],
                    parameters: HashMap::new(),
                });
            }
            cognyx_intent::IntentDomain::ApplicationExecution => {
                if intent
                    .required_capabilities
                    .contains(&"application.open".to_string())
                {
                    steps.push(PlanStep {
                        step_id: "step-1".to_string(),
                        description: format!("Open Windows Application {}", intent.target_object),
                        target_runtime_kind: "WindowsVm".to_string(),
                        required_capabilities: vec![
                            "application.open".to_string(),
                            "gui".to_string(),
                        ],
                        depends_on_step_ids: vec![],
                        parameters: HashMap::new(),
                    });
                } else {
                    steps.push(PlanStep {
                        step_id: "step-1".to_string(),
                        description: format!("Execute Script {}", intent.target_object),
                        target_runtime_kind: "NativeLinux".to_string(),
                        required_capabilities: vec![
                            "terminal.execute".to_string(),
                            "bash".to_string(),
                        ],
                        depends_on_step_ids: vec![],
                        parameters: HashMap::new(),
                    });
                }
            }
            _ => {
                steps.push(PlanStep {
                    step_id: "step-1".to_string(),
                    description: format!("Execute Native Task {}", intent.target_object),
                    target_runtime_kind: "NativeLinux".to_string(),
                    required_capabilities: vec!["bash".to_string()],
                    depends_on_step_ids: vec![],
                    parameters: HashMap::new(),
                });
            }
        }

        Ok(Plan {
            plan_id,
            task_id: task_id.to_string(),
            steps,
            constraints: vec![],
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }
}
