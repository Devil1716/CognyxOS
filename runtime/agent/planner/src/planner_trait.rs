use async_trait::async_trait;
use cognyx_intent::{IntentDomain, ParsedIntent};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
    /// One capability per node makes permission, runtime selection, and data
    /// contracts unambiguous.
    pub required_capabilities: Vec<String>,
    pub depends_on_step_ids: Vec<String>,
    pub parameters: HashMap<String, String>,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
}

impl PlanStep {
    fn with_depends_on(mut self, depends_on_step_ids: Vec<String>) -> Self {
        self.depends_on_step_ids = depends_on_step_ids;
        self
    }

    fn with_parameters(mut self, parameters: HashMap<String, String>) -> Self {
        self.parameters = parameters;
        self
    }

    fn with_preconditions(mut self, preconditions: Vec<String>) -> Self {
        self.preconditions = preconditions;
        self
    }

    fn with_postconditions(mut self, postconditions: Vec<String>) -> Self {
        self.postconditions = postconditions;
        self
    }
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

impl Plan {
    pub fn validate(&self) -> PlanValidationResult {
        let mut errors = Vec::new();
        let ids = self
            .steps
            .iter()
            .map(|step| step.step_id.as_str())
            .collect::<HashSet<_>>();
        for step in &self.steps {
            let required_parameter = match step.required_capabilities.as_slice() {
                [capability] => match capability.as_str() {
                    "application.search" => Some("query"),
                    "application.open" | "application.inspect" => Some("application_id"),
                    "keyboard.type" => Some("text"),
                    "window.focus" | "window.activate" | "window.close" => Some("window_id"),
                    "browser.navigate" => Some("url"),
                    "process.stop" => Some("process_id"),
                    "terminal.execute" => Some("command"),
                    "filesystem.write" => Some("destination"),
                    _ => None,
                },
                [] => {
                    errors.push(format!(
                        "PLAN_INVALID: step '{}' has no capability",
                        step.step_id
                    ));
                    None
                }
                _ => {
                    errors.push(format!(
                        "PLAN_INVALID: step '{}' must invoke exactly one capability",
                        step.step_id
                    ));
                    None
                }
            };
            if let Some(parameter) = required_parameter {
                if step
                    .parameters
                    .get(parameter)
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .is_none()
                {
                    errors.push(format!(
                        "PLAN_INVALID: {} requires '{}' in step '{}'",
                        step.required_capabilities[0], parameter, step.step_id
                    ));
                }
            }
            for value in step.parameters.values() {
                if let Some(reference) = value.strip_prefix("${").and_then(|v| v.strip_suffix('}'))
                {
                    let referenced_step = reference.split('.').next().unwrap_or_default();
                    if !ids.contains(referenced_step) {
                        errors.push(format!(
                            "PLAN_INVALID: step '{}' references unknown output '{}'",
                            step.step_id, reference
                        ));
                    } else if !step
                        .depends_on_step_ids
                        .iter()
                        .any(|dependency| dependency == referenced_step)
                    {
                        errors.push(format!(
                            "PLAN_INVALID: step '{}' must depend on referenced step '{}'",
                            step.step_id, referenced_step
                        ));
                    }
                }
            }
        }
        PlanValidationResult {
            is_valid: errors.is_empty(),
            validation_errors: errors,
            missing_capabilities: vec![],
        }
    }
}

#[async_trait]
pub trait PlannerProvider: Send + Sync {
    fn provider_name(&self) -> &str;
    async fn create_plan(&self, task_id: &str, intent: &ParsedIntent) -> Result<Plan, String>;
}

pub struct DeterministicPlannerProvider;

impl DeterministicPlannerProvider {
    fn step(id: &str, description: impl Into<String>, runtime: &str, capability: &str) -> PlanStep {
        PlanStep {
            step_id: id.into(),
            description: description.into(),
            target_runtime_kind: runtime.into(),
            required_capabilities: vec![capability.into()],
            depends_on_step_ids: vec![],
            parameters: HashMap::new(),
            preconditions: vec![],
            postconditions: vec![],
        }
    }
}

#[async_trait]
impl PlannerProvider for DeterministicPlannerProvider {
    fn provider_name(&self) -> &str {
        "deterministic"
    }

    async fn create_plan(&self, task_id: &str, intent: &ParsedIntent) -> Result<Plan, String> {
        let plan_id = format!("plan-{}", uuid::Uuid::now_v7());
        let mut steps = Vec::new();
        match intent.domain {
            IntentDomain::ApplicationExecution if intent.parameters.contains_key("application") => {
                if intent
                    .parameters
                    .get("application_ambiguous")
                    .map(String::as_str)
                    == Some("true")
                {
                    return Err(
                        "AMBIGUOUS_APPLICATION: no configured default browser is available".into(),
                    );
                }
                let application = intent.parameters["application"].clone();
                steps.push(
                    Self::step(
                        "step-1",
                        format!("Discover application {application}"),
                        "WindowsVm",
                        "application.search",
                    )
                    .with_parameters(HashMap::from([("query".into(), application.clone())]))
                    .with_postconditions(vec!["applications[0].application_id".into()]),
                );
                steps.push(
                    Self::step(
                        "step-2",
                        format!("Open dynamically discovered application {application}"),
                        "WindowsVm",
                        "application.open",
                    )
                    .with_depends_on(vec!["step-1".into()])
                    .with_parameters(HashMap::from([(
                        "application_id".into(),
                        "${step-1.applications[0].application_id}".into(),
                    )]))
                    .with_preconditions(
                        vec!["application.search returned an application_id".into()],
                    )
                    .with_postconditions(vec![
                        "application_id".into(),
                        "process_id".into(),
                        "window_id".into(),
                    ]),
                );
                let mut previous = "step-2".to_string();
                if let Some(text) = intent.parameters.get("text") {
                    steps.push(
                        Self::step(
                            "step-3",
                            "Type requested text into the application",
                            "WindowsVm",
                            "keyboard.type",
                        )
                        .with_depends_on(vec![previous.clone()])
                        .with_parameters(HashMap::from([
                            ("text".into(), text.clone()),
                            ("window_id".into(), "${step-2.window_id}".into()),
                        ]))
                        .with_preconditions(vec![
                            "application.open completed; provider may establish foreground input context".into(),
                        ])
                        .with_postconditions(vec!["text submitted to active input context".into()]),
                    );
                    previous = "step-3".into();
                }
                if let Some(url) = intent.parameters.get("url") {
                    let id = format!("step-{}", steps.len() + 1);
                    let mut close_dependencies = vec![previous];
                    if !close_dependencies
                        .iter()
                        .any(|dependency| dependency == "step-2")
                    {
                        close_dependencies.push("step-2".into());
                    }
                    steps.push(
                        Self::step(
                            &id,
                            "Navigate the opened browser",
                            "WindowsVm",
                            "browser.navigate",
                        )
                        .with_depends_on(close_dependencies)
                        .with_parameters(HashMap::from([("url".into(), url.clone())]))
                        .with_preconditions(vec!["browser session available".into()])
                        .with_postconditions(vec!["browser navigated".into()]),
                    );
                    previous = id;
                }
                if intent
                    .parameters
                    .get("actions")
                    .is_some_and(|actions| actions.split(',').any(|action| action == "close"))
                {
                    let id = format!("step-{}", steps.len() + 1);
                    let mut close_dependencies = vec![previous];
                    if !close_dependencies
                        .iter()
                        .any(|dependency| dependency == "step-2")
                    {
                        close_dependencies.push("step-2".into());
                    }
                    steps.push(
                        Self::step(
                            &id,
                            "Close the focused application window",
                            "WindowsVm",
                            "window.close",
                        )
                        .with_depends_on(close_dependencies)
                        .with_parameters(HashMap::from([(
                            "window_id".into(),
                            "${step-2.window_id}".into(),
                        )]))
                        .with_preconditions(vec!["application.open returned a window_id".into()])
                        .with_postconditions(vec!["application window closed".into()]),
                    );
                }
            }
            IntentDomain::SystemOperation
                if intent
                    .required_capabilities
                    .iter()
                    .any(|c| c == "terminal.execute") =>
            {
                let command = intent
                    .parameters
                    .get("command")
                    .cloned()
                    .unwrap_or_default();
                steps.push(
                    Self::step(
                        "step-1",
                        "Execute the requested terminal command",
                        "NativeLinux",
                        "terminal.execute",
                    )
                    .with_parameters(HashMap::from([("command".into(), command)])),
                );
            }
            IntentDomain::AppInstallation => {
                steps.push(
                    Self::step(
                        "step-1",
                        "Verify Windows Runtime Environment",
                        "WindowsVm",
                        "win32.powershell",
                    )
                    .with_postconditions(vec!["windows runtime available".into()]),
                );
                steps.push(
                    Self::step(
                        "step-2",
                        format!("Execute Winget Install for {}", intent.target_object),
                        "WindowsVm",
                        "package.install",
                    )
                    .with_depends_on(vec!["step-1".into()])
                    .with_preconditions(vec!["windows runtime available".into()])
                    .with_postconditions(vec!["package installed".into()]),
                );
            }
            _ => {
                return Err(
                    "PLAN_INVALID: no deterministic planner is available for this intent".into(),
                )
            }
        }
        Ok(Plan {
            plan_id,
            task_id: task_id.into(),
            steps,
            constraints: vec![],
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cognyx_intent::IntentEngine;

    #[tokio::test]
    async fn application_plan_resolves_then_opens_then_types() {
        let intent = IntentEngine::default()
            .parse_prompt("Open Notepad and type Hello CognyxOS")
            .await;
        let plan = DeterministicPlannerProvider
            .create_plan("task", &intent)
            .await
            .unwrap();
        assert!(plan.validate().is_valid);
        assert_eq!(
            plan.steps
                .iter()
                .map(|step| step.required_capabilities[0].as_str())
                .collect::<Vec<_>>(),
            vec!["application.search", "application.open", "keyboard.type"]
        );
        assert_eq!(
            plan.steps[1].parameters["application_id"],
            "${step-1.applications[0].application_id}"
        );
    }

    #[tokio::test]
    async fn open_only_plan_is_search_then_open() {
        let intent = IntentEngine::default()
            .parse_prompt("Open Calculator")
            .await;
        let plan = DeterministicPlannerProvider
            .create_plan("task", &intent)
            .await
            .unwrap();
        assert!(plan.validate().is_valid);
        assert_eq!(
            plan.steps
                .iter()
                .map(|step| step.required_capabilities[0].as_str())
                .collect::<Vec<_>>(),
            vec!["application.search", "application.open"]
        );
        assert!(!plan
            .steps
            .iter()
            .any(|step| step.required_capabilities.iter().any(|c| c == "bash")));
    }

    #[tokio::test]
    async fn multi_action_close_uses_window_from_open() {
        let intent = IntentEngine::default()
            .parse_prompt("Open Notepad, type Hello, and close it")
            .await;
        let plan = DeterministicPlannerProvider
            .create_plan("task", &intent)
            .await
            .unwrap();
        assert!(plan.validate().is_valid);
        assert_eq!(
            plan.steps
                .iter()
                .map(|step| step.required_capabilities[0].as_str())
                .collect::<Vec<_>>(),
            vec![
                "application.search",
                "application.open",
                "keyboard.type",
                "window.close"
            ]
        );
        assert_eq!(plan.steps[3].parameters["window_id"], "${step-2.window_id}");
        assert!(plan.steps[3]
            .depends_on_step_ids
            .iter()
            .any(|id| id == "step-2"));
    }

    #[test]
    fn missing_keyboard_text_is_plan_invalid() {
        let plan = Plan {
            plan_id: "plan".into(),
            task_id: "task".into(),
            steps: vec![DeterministicPlannerProvider::step(
                "step-1",
                "type",
                "WindowsVm",
                "keyboard.type",
            )],
            constraints: vec![],
            created_at_ms: 0,
        };
        assert!(plan
            .validate()
            .validation_errors
            .iter()
            .any(|error| error.contains("keyboard.type requires 'text'")));
    }

    #[tokio::test]
    async fn explicit_terminal_plan_is_not_an_application_graph() {
        let intent = IntentEngine::default()
            .parse_prompt("Run a Python script")
            .await;
        let plan = DeterministicPlannerProvider
            .create_plan("task", &intent)
            .await
            .unwrap();
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(
            plan.steps[0].required_capabilities,
            vec!["terminal.execute"]
        );
        assert_eq!(
            plan.steps[0].parameters.get("command"),
            Some(&"a Python script".to_string())
        );
        assert!(!plan
            .steps
            .iter()
            .any(|step| step.required_capabilities.iter().any(|c| c == "bash")));
    }

    #[test]
    fn malformed_capability_input_is_rejected_before_execution() {
        let plan = Plan {
            plan_id: "plan".into(),
            task_id: "task".into(),
            steps: vec![DeterministicPlannerProvider::step(
                "step-1",
                "bad open",
                "WindowsVm",
                "application.open",
            )],
            constraints: vec![],
            created_at_ms: 0,
        };
        assert!(plan
            .validate()
            .validation_errors
            .iter()
            .any(|error| error.contains("application.open requires 'application_id'")));
    }
}
