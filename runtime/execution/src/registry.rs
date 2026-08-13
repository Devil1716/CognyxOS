use crate::traits::ExecutionRuntime;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::info;

#[derive(Default)]
pub struct RuntimeRegistry {
    runtimes:
        RwLock<HashMap<String, Arc<tokio::sync::RwLock<Box<dyn ExecutionRuntime + Send + Sync>>>>>,
}

impl RuntimeRegistry {
    pub fn new() -> Self {
        Self {
            runtimes: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, runtime: Box<dyn ExecutionRuntime + Send + Sync>) -> String {
        let id = runtime.runtime_id().to_string();
        info!(
            "Registering runtime '{}' (type: {:?})",
            id,
            runtime.runtime_type()
        );
        let mut map = self.runtimes.write().unwrap();
        map.insert(id.clone(), Arc::new(tokio::sync::RwLock::new(runtime)));
        id
    }

    pub fn unregister(&self, id: &str) {
        info!("Unregistering runtime '{}'", id);
        let mut map = self.runtimes.write().unwrap();
        map.remove(id);
    }

    pub async fn find_runtime_for_capability(&self, capability: &str) -> Option<String> {
        let runtimes: Vec<Arc<tokio::sync::RwLock<Box<dyn ExecutionRuntime + Send + Sync>>>> = {
            let map = self.runtimes.read().unwrap();
            map.values().cloned().collect()
        };

        for rt in runtimes {
            let lock = rt.read().await;
            if lock.can_perform(capability) {
                return Some(lock.runtime_id().to_string());
            }
        }
        None
    }

    pub fn list_runtime_ids(&self) -> Vec<String> {
        let map = self.runtimes.read().unwrap();
        map.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtimes::LinuxRuntime;

    #[tokio::test]
    async fn test_runtime_registry_capability_lookup() {
        let registry = RuntimeRegistry::new();
        let linux = Box::new(LinuxRuntime::new("linux-host-1", "Local Host"));

        let id = registry.register(linux);
        assert_eq!(id, "linux-host-1");

        let found = registry.find_runtime_for_capability("bash").await;
        assert_eq!(found, Some("linux-host-1".to_string()));
    }
}
