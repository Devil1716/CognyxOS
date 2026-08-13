use cognyx_execution::RuntimeRegistry;
use std::sync::Arc;
use tracing::info;

pub struct CapabilityResolver {
    registry: Arc<RuntimeRegistry>,
}

impl CapabilityResolver {
    pub fn new(registry: Arc<RuntimeRegistry>) -> Self {
        Self { registry }
    }

    pub async fn resolve_runtime_for_capability(&self, capability: &str) -> Option<String> {
        info!(
            "Resolving runtime via RuntimeRegistry for capability '{}'",
            capability
        );
        self.registry.find_runtime_for_capability(capability).await
    }
}
