use crate::model::*;
use async_trait::async_trait;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct CapabilityProviderContext {
    pub request: CapabilityRequest,
    pub runtime_id: String,
}
#[derive(Clone, Debug)]
pub struct CapabilityProviderResult {
    pub output: Value,
    pub artifacts: Vec<String>,
    pub side_effects: Vec<String>,
    pub metadata: Value,
}
#[async_trait]
pub trait CapabilityProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn runtime_id(&self) -> &str;
    fn priority(&self) -> u8 {
        100
    }
    fn definitions(&self) -> Vec<CapabilityDefinition>;
    fn health(&self) -> CapabilityProviderHealth {
        CapabilityProviderHealth::default()
    }
    async fn execute(
        &self,
        context: CapabilityProviderContext,
    ) -> Result<CapabilityProviderResult, CapabilityError>;
}
