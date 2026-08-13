use crate::model::*;
use crate::provider::*;
use crate::registry::CapabilityRegistry;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};

#[derive(Clone, Default)]
pub struct UniversalCapabilityLayer {
    registry: Arc<CapabilityRegistry>,
}
impl UniversalCapabilityLayer {
    pub fn new(registry: Arc<CapabilityRegistry>) -> Self {
        Self { registry }
    }
    pub fn registry(&self) -> Arc<CapabilityRegistry> {
        self.registry.clone()
    }
    pub fn register_provider(&self, provider: Arc<dyn CapabilityProvider>) -> Result<(), String> {
        let id = provider.provider_id().to_string();
        self.registry.register_provider(provider)?;
        info!(event = "capability.registered", provider_id = %id);
        Ok(())
    }
    pub async fn execute(&self, request: CapabilityRequest) -> CapabilityResult {
        info!(event = "capability.requested", request_id = %request.request_id, capability_id = %request.capability_id, task_id = %request.task_id);
        let definition = match self
            .registry
            .lookup(&request.capability_id, request.requested_version.as_ref())
        {
            Some(d) => d,
            None => {
                return CapabilityResult::failed(
                    &request,
                    request
                        .runtime_hint
                        .clone()
                        .unwrap_or_else(|| "unresolved".into()),
                    CapabilityErrorCode::CapabilityUnavailable,
                    "No compatible capability definition is registered",
                )
            }
        };
        let candidates = self
            .registry
            .provider_candidates(&definition.capability_id, request.runtime_hint.as_deref());
        if candidates.is_empty() {
            return CapabilityResult::failed(
                &request,
                request
                    .runtime_hint
                    .clone()
                    .unwrap_or_else(|| "unresolved".into()),
                CapabilityErrorCode::RuntimeUnavailable,
                "No healthy provider is available for this capability",
            );
        }
        let started = now_ms();
        let deadline = request.timeout_ms.unwrap_or(definition.metadata.timeout_ms);
        for provider in candidates {
            let runtime_id = provider.runtime_id().to_string();
            let provider_id = provider.provider_id().to_string();
            info!(event = "capability.started", request_id = %request.request_id, provider_id = %provider_id, runtime_id = %runtime_id);
            let context = CapabilityProviderContext {
                request: request.clone(),
                runtime_id: runtime_id.clone(),
            };
            match timeout(Duration::from_millis(deadline), provider.execute(context)).await {
                Ok(Ok(value)) => {
                    info!(event = "capability.completed", request_id = %request.request_id, provider_id = %provider_id);
                    return CapabilityResult {
                        request_id: request.request_id,
                        capability_id: request.capability_id,
                        runtime_id,
                        status: CapabilityStatus::Completed,
                        output: value.output,
                        error: None,
                        metadata: value.metadata,
                        execution_time_ms: now_ms().saturating_sub(started),
                        artifacts: value.artifacts,
                        side_effects: value.side_effects,
                        provider_id: Some(provider_id),
                    };
                }
                Ok(Err(error)) if error.retryable => {
                    warn!(event = "capability.provider_unhealthy", provider_id = %provider_id, error = %error.message);
                    continue;
                }
                Ok(Err(error)) => {
                    return CapabilityResult {
                        request_id: request.request_id,
                        capability_id: request.capability_id,
                        runtime_id,
                        status: CapabilityStatus::Failed,
                        output: serde_json::Value::Null,
                        error: Some(error),
                        metadata: serde_json::Value::Null,
                        execution_time_ms: now_ms().saturating_sub(started),
                        artifacts: vec![],
                        side_effects: vec![],
                        provider_id: Some(provider_id),
                    }
                }
                Err(_) => {
                    return CapabilityResult {
                        request_id: request.request_id,
                        capability_id: request.capability_id,
                        runtime_id,
                        status: CapabilityStatus::Timeout,
                        output: serde_json::Value::Null,
                        error: Some(CapabilityError {
                            code: CapabilityErrorCode::Timeout,
                            message: "Capability execution timed out".into(),
                            retryable: true,
                        }),
                        metadata: serde_json::Value::Null,
                        execution_time_ms: now_ms().saturating_sub(started),
                        artifacts: vec![],
                        side_effects: vec![],
                        provider_id: Some(provider_id),
                    }
                }
            }
        }
        CapabilityResult::failed(
            &request,
            "unresolved",
            CapabilityErrorCode::ProviderUnavailable,
            "All providers failed",
        )
    }
}
