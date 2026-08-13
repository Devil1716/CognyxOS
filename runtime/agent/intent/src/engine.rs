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
        } else if lower.contains("windows") || lower.contains("application") {
            let target = lower
                .replace("open", "")
                .replace("windows", "")
                .trim()
                .to_string();
            Ok(ParsedIntent {
                intent_id,
                original_prompt: intent.raw_prompt.clone(),
                domain: IntentDomain::ApplicationExecution,
                target_object: if target.is_empty() {
                    "Windows App".to_string()
                } else {
                    target
                },
                required_capabilities: vec![
                    "application.open".to_string(),
                    "gui".to_string(),
                    "win32.powershell".to_string(),
                ],
                constraints: vec![],
                parameters,
                confidence: 0.93,
            })
        } else if lower.contains("python") || lower.contains("script") || lower.contains("run") {
            let target = lower
                .replace("run", "")
                .replace("python", "")
                .trim()
                .to_string();
            Ok(ParsedIntent {
                intent_id,
                original_prompt: intent.raw_prompt.clone(),
                domain: IntentDomain::ApplicationExecution,
                target_object: if target.is_empty() {
                    "script.py".to_string()
                } else {
                    target
                },
                required_capabilities: vec!["terminal.execute".to_string(), "bash".to_string()],
                constraints: vec![],
                parameters,
                confidence: 0.94,
            })
        } else {
            Ok(ParsedIntent {
                intent_id,
                original_prompt: intent.raw_prompt.clone(),
                domain: IntentDomain::SystemOperation,
                target_object: intent.raw_prompt.clone(),
                required_capabilities: vec!["bash".to_string()],
                constraints: vec![],
                parameters,
                confidence: 0.70,
            })
        }
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
        assert_eq!(intent2.domain, IntentDomain::ApplicationExecution);

        let intent3 = engine.parse_prompt("Open a Windows application").await;
        assert_eq!(intent3.domain, IntentDomain::ApplicationExecution);
    }
}
