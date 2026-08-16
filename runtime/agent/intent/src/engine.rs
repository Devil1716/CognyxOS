use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntentDomain {
    AppInstallation,
    DocumentGeneration,
    DataAnalysis,
    SessionResume,
    SystemOperation,
    ApplicationExecution,
    FileManagement,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentConstraint {
    pub key: String,
    pub value: String,
    pub is_hard_requirement: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentContext {
    pub session_id: String,
    pub current_workspace: String,
    pub environment_params: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Intent {
    pub intent_id: String,
    pub raw_prompt: String,
    pub context: IntentContext,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParsedIntent {
    pub intent_id: String,
    pub original_prompt: String,
    pub domain: IntentDomain,
    pub target_object: String,
    pub required_capabilities: Vec<String>,
    pub constraints: Vec<IntentConstraint>,
    pub parameters: HashMap<String, String>,
    pub confidence: f32,
}

#[async_trait]
pub trait IntentProvider: Send + Sync {
    fn provider_name(&self) -> &str;
    async fn parse(&self, intent: &Intent) -> Result<ParsedIntent, String>;
}

pub struct DeterministicIntentProvider;

#[async_trait]
impl IntentProvider for DeterministicIntentProvider {
    fn provider_name(&self) -> &str {
        "deterministic"
    }

    async fn parse(&self, intent: &Intent) -> Result<ParsedIntent, String> {
        let lower = intent.raw_prompt.to_lowercase();
        let intent_id = format!("intent-{}", uuid::Uuid::now_v7());
        let mut parameters = HashMap::new();

        // Application interaction is deliberately recognized before the generic
        // document/system branches.  It must never be reinterpreted as a shell
        // command merely because the requested application is not installed.
        if let Some((verb, application)) = Self::application_request(&intent.raw_prompt) {
            let actions = Self::application_actions(&intent.raw_prompt);
            parameters.insert("primary_action".into(), verb);
            parameters.insert("application".into(), application.clone());
            parameters.insert("actions".into(), actions.join(","));
            parameters.insert(
                "expected_outcome".into(),
                "application interaction completed".into(),
            );
            parameters.insert(
                "dependencies".into(),
                "application.search->application.open".into(),
            );
            if let Some(text) = Self::value_after(&intent.raw_prompt, "type") {
                parameters.insert("text".into(), text);
            }
            if !parameters.contains_key("text") {
                if let Some(expression) = Self::value_after(&intent.raw_prompt, "calculate") {
                    parameters.insert("text".into(), expression);
                }
            }
            if let Some(url) = Self::value_after(&intent.raw_prompt, "navigate to") {
                parameters.insert("url".into(), url);
            }
            if application.eq_ignore_ascii_case("browser") {
                parameters.insert("application_ambiguous".into(), "true".into());
            }

            let mut required_capabilities =
                vec!["application.search".into(), "application.open".into()];
            if actions.iter().any(|action| action == "type") {
                required_capabilities.push("keyboard.type".into());
            }
            if actions.iter().any(|action| action == "navigate") {
                required_capabilities.push("browser.navigate".into());
            }
            if actions.iter().any(|action| action == "close") {
                required_capabilities.push("window.close".into());
            }

            return Ok(ParsedIntent {
                intent_id,
                original_prompt: intent.raw_prompt.clone(),
                domain: IntentDomain::ApplicationExecution,
                target_object: application,
                required_capabilities,
                constraints: vec![],
                parameters,
                confidence: 0.96,
            });
        }

        if lower.contains("install") {
            let app_name = lower.replace("install", "").trim().to_string();
            parameters.insert("app_name".to_string(), app_name.clone());

            Ok(ParsedIntent {
                intent_id,
                original_prompt: intent.raw_prompt.clone(),
                domain: IntentDomain::AppInstallation,
                target_object: app_name,
                required_capabilities: vec![
                    "package.install".to_string(),
                    "win32.powershell".to_string(),
                ],
                constraints: vec![],
                parameters,
                confidence: 0.95,
            })
        } else if lower.contains("presentation")
            || lower.contains("document")
            || lower.contains("create")
        {
            parameters.insert("doc_type".to_string(), "presentation".to_string());

            Ok(ParsedIntent {
                intent_id,
                original_prompt: intent.raw_prompt.clone(),
                domain: IntentDomain::DocumentGeneration,
                target_object: "presentation.pptx".to_string(),
                required_capabilities: vec!["file.write".to_string(), "doc.render".to_string()],
                constraints: vec![],
                parameters,
                confidence: 0.90,
            })
        } else if lower.contains("yesterday")
            || lower.contains("continue")
            || lower.contains("resume")
        {
            Ok(ParsedIntent {
                intent_id,
                original_prompt: intent.raw_prompt.clone(),
                domain: IntentDomain::SessionResume,
                target_object: "last_working_session".to_string(),
                required_capabilities: vec![
                    "memory.query".to_string(),
                    "session.restore".to_string(),
                ],
                constraints: vec![],
                parameters,
                confidence: 0.92,
            })
        } else if lower.contains("analyze") || lower.contains("compare") {
            Ok(ParsedIntent {
                intent_id,
                original_prompt: intent.raw_prompt.clone(),
                domain: IntentDomain::DataAnalysis,
                target_object: "dataset".to_string(),
                required_capabilities: vec![
                    "container.exec".to_string(),
                    "data.process".to_string(),
                ],
                constraints: vec![],
                parameters,
                confidence: 0.88,
            })
        } else if Self::explicit_terminal_request(&lower) {
            if let Some(command) = Self::terminal_command(&intent.raw_prompt) {
                parameters.insert("command".into(), command);
            }
            Ok(ParsedIntent {
                intent_id,
                original_prompt: intent.raw_prompt.clone(),
                domain: IntentDomain::SystemOperation,
                target_object: intent.raw_prompt.clone(),
                required_capabilities: vec!["terminal.execute".to_string()],
                constraints: vec![],
                parameters,
                confidence: 0.9,
            })
        } else {
            Ok(ParsedIntent {
                intent_id,
                original_prompt: intent.raw_prompt.clone(),
                domain: IntentDomain::Unknown,
                target_object: intent.raw_prompt.clone(),
                required_capabilities: vec![],
                constraints: vec![],
                parameters,
                confidence: 0.70,
            })
        }
    }
}

impl DeterministicIntentProvider {
    fn application_request(prompt: &str) -> Option<(String, String)> {
        let lower = prompt.to_ascii_lowercase();
        let verb = if lower.starts_with("open ") {
            "open"
        } else if lower.starts_with("launch ") {
            "launch"
        } else {
            return None;
        };
        let rest = prompt[verb.len()..].trim_start();
        let end = [" and ", ",", ";"]
            .iter()
            .filter_map(|separator| rest.to_ascii_lowercase().find(separator))
            .min()
            .unwrap_or(rest.len());
        let application = rest[..end]
            .trim()
            .trim_end_matches('.')
            .strip_prefix("the ")
            .or_else(|| rest[..end].trim().trim_end_matches('.').strip_prefix("a "))
            .unwrap_or(rest[..end].trim().trim_end_matches('.'))
            .trim();
        (!application.is_empty()).then(|| (verb.to_string(), application.to_string()))
    }

    fn value_after(prompt: &str, marker: &str) -> Option<String> {
        let lower = prompt.to_ascii_lowercase();
        let start = lower.find(marker)? + marker.len();
        let mut value = prompt[start..].trim().trim_start_matches("to ").to_string();
        let value_lower = value.to_ascii_lowercase();
        if let Some(cut) = [
            " and close",
            ", and ",
            " then close",
            ", then ",
            " and then ",
        ]
        .iter()
        .filter_map(|separator| value_lower.find(separator))
        .min()
        {
            value.truncate(cut);
        }
        let value = value.trim().trim_end_matches(['.', ',']).trim().to_string();
        (!value.is_empty()).then_some(value)
    }

    fn explicit_terminal_request(lower: &str) -> bool {
        lower.contains("terminal")
            || lower.contains("shell command")
            || lower.starts_with("run command")
            || (lower.starts_with("run ") && (lower.contains("python") || lower.contains("script")))
    }

    fn terminal_command(prompt: &str) -> Option<String> {
        let lower = prompt.to_ascii_lowercase();
        if let Some(idx) = lower.find("run ") {
            let command = prompt[idx + 4..]
                .trim()
                .trim_start_matches("command ")
                .trim_start_matches("in the terminal ")
                .trim_end_matches('.')
                .trim();
            return (!command.is_empty()).then(|| command.to_string());
        }
        None
    }

    fn application_actions(prompt: &str) -> Vec<String> {
        let lower = prompt.to_ascii_lowercase();
        let mut actions = vec!["open".to_string()];
        for (needle, action) in [
            ("type", "type"),
            ("navigate", "navigate"),
            ("calculate", "calculate"),
            ("close", "close"),
        ] {
            if lower.contains(needle) {
                actions.push(action.into());
            }
        }
        actions
    }
}

pub struct MockIntentProvider;

#[async_trait]
impl IntentProvider for MockIntentProvider {
    fn provider_name(&self) -> &str {
        "mock"
    }

    async fn parse(&self, intent: &Intent) -> Result<ParsedIntent, String> {
        let provider = DeterministicIntentProvider;
        provider.parse(intent).await
    }
}

pub struct IntentEngine {
    provider: Arc<dyn IntentProvider>,
}

impl Default for IntentEngine {
    fn default() -> Self {
        Self::new(Arc::new(DeterministicIntentProvider))
    }
}

impl IntentEngine {
    pub fn new(provider: Arc<dyn IntentProvider>) -> Self {
        Self { provider }
    }

    pub async fn parse_prompt(&self, prompt: &str) -> ParsedIntent {
        info!("IntentEngine processing prompt: '{}'", prompt);

        let intent = Intent {
            intent_id: format!("intent-{}", uuid::Uuid::now_v7()),
            raw_prompt: prompt.to_string(),
            context: IntentContext {
                session_id: "default-session".to_string(),
                current_workspace: "/var/lib/cognyxos".to_string(),
                environment_params: HashMap::new(),
            },
        };

        self.provider
            .parse(&intent)
            .await
            .unwrap_or_else(|_| ParsedIntent {
                intent_id: intent.intent_id,
                original_prompt: prompt.to_string(),
                domain: IntentDomain::Unknown,
                target_object: prompt.to_string(),
                required_capabilities: vec![],
                constraints: vec![],
                parameters: HashMap::new(),
                confidence: 0.0,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_intent_engine_providers() {
        let engine = IntentEngine::default();

        let intent1 = engine.parse_prompt("Install Photoshop").await;
        assert_eq!(intent1.domain, IntentDomain::AppInstallation);

        let intent2 = engine.parse_prompt("Run a Python script").await;
        assert_eq!(intent2.domain, IntentDomain::SystemOperation);
        assert_eq!(intent2.required_capabilities, vec!["terminal.execute"]);

        let intent3 = engine.parse_prompt("Open a Windows application").await;
        assert_eq!(intent3.domain, IntentDomain::ApplicationExecution);
    }

    #[tokio::test]
    async fn application_prompt_has_structured_entities_and_actions() {
        let parsed = IntentEngine::default()
            .parse_prompt("Open Notepad and type Hello CognyxOS")
            .await;
        assert_eq!(parsed.domain, IntentDomain::ApplicationExecution);
        assert_eq!(
            parsed.parameters.get("application"),
            Some(&"Notepad".to_string())
        );
        assert_eq!(
            parsed.parameters.get("text"),
            Some(&"Hello CognyxOS".to_string())
        );
        assert_eq!(
            parsed.parameters.get("actions"),
            Some(&"open,type".to_string())
        );
        assert!(!parsed.required_capabilities.iter().any(|c| c == "bash"));
        assert!(parsed
            .required_capabilities
            .iter()
            .any(|c| c == "keyboard.type"));
    }

    #[tokio::test]
    async fn comma_separated_type_and_close_does_not_swallow_close_into_text() {
        let parsed = IntentEngine::default()
            .parse_prompt("Open Notepad, type Hello, and close it")
            .await;
        assert_eq!(parsed.parameters.get("text"), Some(&"Hello".to_string()));
        assert_eq!(
            parsed.parameters.get("actions"),
            Some(&"open,type,close".to_string())
        );
        assert!(parsed
            .required_capabilities
            .iter()
            .any(|c| c == "window.close"));
    }

    #[tokio::test]
    async fn explicit_terminal_request_is_not_an_application_open() {
        let parsed = IntentEngine::default()
            .parse_prompt("Run a Python script")
            .await;
        assert_eq!(parsed.domain, IntentDomain::SystemOperation);
        assert_eq!(parsed.required_capabilities, vec!["terminal.execute"]);
        assert!(!parsed.required_capabilities.iter().any(|c| c == "bash"));
    }

    #[tokio::test]
    async fn application_names_are_entities_not_executable_guesses() {
        let engine = IntentEngine::default();
        for (prompt, application) in [
            ("Open Calculator", "Calculator"),
            ("Launch Paint", "Paint"),
            ("Open VS Code", "VS Code"),
            ("Open Chrome", "Chrome"),
        ] {
            let parsed = engine.parse_prompt(prompt).await;
            assert_eq!(
                parsed.parameters.get("application"),
                Some(&application.to_string())
            );
            assert!(!parsed
                .required_capabilities
                .iter()
                .any(|capability| capability == "bash"));
        }
    }
}
