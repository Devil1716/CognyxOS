use crate::model::*;
use crate::provider::CapabilityProvider;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct CapabilityRegistry {
    definitions: DashMap<String, Vec<CapabilityDefinition>>,
    providers: DashMap<String, Arc<dyn CapabilityProvider>>,
}
impl CapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&self, definition: CapabilityDefinition) -> Result<(), String> {
        let mut versions = self
            .definitions
            .entry(definition.capability_id.clone())
            .or_default();
        if versions.iter().any(|d| d.version == definition.version) {
            return Err(format!(
                "capability '{}' {} is already registered",
                definition.capability_id, definition.version
            ));
        }
        versions.push(definition);
        versions.sort_by(|a, b| a.version.cmp(&b.version));
        Ok(())
    }
    pub fn unregister(&self, id: &str, version: &CapabilityVersion) -> bool {
        if let Some(mut versions) = self.definitions.get_mut(id) {
            let old = versions.len();
            versions.retain(|d| &d.version != version);
            return old != versions.len();
        }
        false
    }
    pub fn lookup(
        &self,
        id: &str,
        requested: Option<&CapabilityVersion>,
    ) -> Option<CapabilityDefinition> {
        self.definitions.get(id).and_then(|versions| {
            versions
                .iter()
                .filter(|d| {
                    requested
                        .map(|v| d.version.compatible_with(v))
                        .unwrap_or(true)
                })
                .max_by_key(|d| d.version.clone())
                .cloned()
        })
    }
    pub fn list(&self) -> Vec<CapabilityDefinition> {
        self.definitions
            .iter()
            .flat_map(|v| v.value().clone())
            .collect()
    }
    pub fn search(&self, query: &str) -> Vec<CapabilityDefinition> {
        let needle = query.to_ascii_lowercase();
        self.list()
            .into_iter()
            .filter(|d| {
                d.capability_id.to_ascii_lowercase().contains(&needle)
                    || d.description.to_ascii_lowercase().contains(&needle)
            })
            .collect()
    }
    pub fn register_provider(&self, provider: Arc<dyn CapabilityProvider>) -> Result<(), String> {
        let id = provider.provider_id().to_string();
        if self.providers.contains_key(&id) {
            return Err(format!("provider '{id}' is already registered"));
        }
        for d in provider.definitions() {
            if self.lookup(&d.capability_id, Some(&d.version)).is_none() {
                self.register(d)?;
            }
        }
        self.providers.insert(id, provider);
        Ok(())
    }
    pub fn unregister_provider(&self, id: &str) -> bool {
        self.providers.remove(id).is_some()
    }
    pub fn provider_ids_for(&self, capability_id: &str) -> Vec<String> {
        let mut matched: Vec<_> = self
            .providers
            .iter()
            .filter(|p| {
                p.value()
                    .definitions()
                    .iter()
                    .any(|d| d.capability_id == capability_id)
            })
            .map(|p| p.key().clone())
            .collect();
        matched.sort();
        matched
    }
    pub fn provider_candidates(
        &self,
        capability_id: &str,
        runtime_hint: Option<&str>,
    ) -> Vec<Arc<dyn CapabilityProvider>> {
        let mut providers: Vec<_> = self
            .providers
            .iter()
            .filter(|p| {
                runtime_hint
                    .map(|r| p.value().runtime_id() == r)
                    .unwrap_or(true)
                    && p.value()
                        .definitions()
                        .iter()
                        .any(|d| d.capability_id == capability_id)
                    && p.value().health().availability != ProviderAvailability::Unavailable
            })
            .map(|p| p.value().clone())
            .collect();
        providers.sort_by_key(|p| p.priority());
        providers
    }
    pub fn provider_health(&self) -> Vec<(String, CapabilityProviderHealth)> {
        self.providers
            .iter()
            .map(|p| (p.key().clone(), p.value().health()))
            .collect()
    }
}
