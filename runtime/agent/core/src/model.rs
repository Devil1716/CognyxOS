use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub reasoning: bool,
    pub code_generation: bool,
    pub function_calling: bool,
    pub vision: bool,
    pub max_context_tokens: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelRequest {
    pub model_id: String,
    pub prompt: String,
    pub temperature: f32,
    pub max_tokens: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelResponse {
    pub text: String,
    pub tokens_used: usize,
    pub finish_reason: String,
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn provider_name(&self) -> &str;
    fn capabilities(&self) -> ModelCapabilities;
    async fn generate(&self, req: ModelRequest) -> Result<ModelResponse, String>;
}

pub struct MockModelProvider {
    name: String,
}

impl Default for MockModelProvider {
    fn default() -> Self {
        Self::new("mock-llm-v1")
    }
}

impl MockModelProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl ModelProvider for MockModelProvider {
    fn provider_name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            reasoning: true,
            code_generation: true,
            function_calling: true,
            vision: false,
            max_context_tokens: 32768,
        }
    }

    async fn generate(&self, req: ModelRequest) -> Result<ModelResponse, String> {
        Ok(ModelResponse {
            text: format!("Mock LLM response for: '{}'", req.prompt),
            tokens_used: 42,
            finish_reason: "stop".to_string(),
        })
    }
}

pub struct ModelRegistry {
    providers: DashMap<String, Arc<dyn ModelProvider>>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        let registry = Self::new();
        registry.register_provider(Arc::new(MockModelProvider::default()));
        registry
    }
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            providers: DashMap::new(),
        }
    }

    pub fn register_provider(&self, provider: Arc<dyn ModelProvider>) {
        self.providers
            .insert(provider.provider_name().to_string(), provider);
    }

    pub fn get_provider(&self, name: &str) -> Option<Arc<dyn ModelProvider>> {
        self.providers.get(name).map(|entry| entry.value().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_model_provider() {
        let registry = ModelRegistry::default();
        let provider = registry.get_provider("mock-llm-v1").unwrap();

        let req = ModelRequest {
            model_id: "mock-llm-v1".to_string(),
            prompt: "Test prompt".to_string(),
            temperature: 0.7,
            max_tokens: 100,
        };

        let res = provider.generate(req).await.unwrap();
        assert!(res.text.contains("Mock LLM response"));
    }
}
