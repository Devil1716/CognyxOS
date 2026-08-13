use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum NetworkMode {
    Nat,
    Bridge,
    Isolated,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkRule {
    pub rule_id: String,
    pub source_runtime_id: String,
    pub target_runtime_id: String,
    pub allowed_port: Option<u32>,
    pub protocol: String, // "tcp", "udp", "*"
    pub allow: bool,
}

#[derive(Clone, Debug)]
pub struct RuntimeNetworkConfig {
    pub runtime_id: String,
    pub mode: NetworkMode,
    pub ip_address: String,
    pub mac_address: String,
}

#[derive(Default)]
pub struct VirtualNetworkManager {
    runtimes: Arc<RwLock<HashMap<String, RuntimeNetworkConfig>>>,
    rules: Arc<RwLock<Vec<NetworkRule>>>,
}

impl VirtualNetworkManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register_runtime_network(&self, config: RuntimeNetworkConfig) {
        info!(
            "Registering network for runtime '{}' (IP: {}, mode: {:?})",
            config.runtime_id, config.ip_address, config.mode
        );
        let mut lock = self.runtimes.write().await;
        lock.insert(config.runtime_id.clone(), config);
    }

    pub async fn add_firewall_rule(&self, rule: NetworkRule) {
        info!(
            "Adding firewall rule: {} -> {} (port: {:?}, allow: {})",
            rule.source_runtime_id, rule.target_runtime_id, rule.allowed_port, rule.allow
        );
        let mut lock = self.rules.write().await;
        lock.push(rule);
    }

    pub async fn can_communicate(
        &self,
        source_id: &str,
        target_id: &str,
        port: u32,
        protocol: &str,
    ) -> (bool, String) {
        let lock_runtimes = self.runtimes.read().await;
        let source_cfg = lock_runtimes.get(source_id);
        let target_cfg = lock_runtimes.get(target_id);

        if let (Some(src), Some(dst)) = (source_cfg, target_cfg) {
            if src.mode == NetworkMode::Isolated || dst.mode == NetworkMode::Isolated {
                return (
                    false,
                    "One or both runtimes are in ISOLATED network mode.".to_string(),
                );
            }
        }

        let lock_rules = self.rules.read().await;
        for rule in lock_rules.iter() {
            if (rule.source_runtime_id == source_id || rule.source_runtime_id == "*")
                && (rule.target_runtime_id == target_id || rule.target_runtime_id == "*")
            {
                if let Some(p) = rule.allowed_port {
                    if p != port {
                        continue;
                    }
                }
                if rule.protocol != "*" && rule.protocol != protocol {
                    continue;
                }
                return (rule.allow, format!("Rule '{}' applied", rule.rule_id));
            }
        }

        // Default allow intra-host NAT traffic
        (
            true,
            "Default host network policy allowed communication".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_policy_evaluation() {
        let mgr = VirtualNetworkManager::new();

        mgr.register_runtime_network(RuntimeNetworkConfig {
            runtime_id: "rt-a".to_string(),
            mode: NetworkMode::Nat,
            ip_address: "192.168.122.10".to_string(),
            mac_address: "52:54:00:00:00:01".to_string(),
        })
        .await;

        mgr.register_runtime_network(RuntimeNetworkConfig {
            runtime_id: "rt-b".to_string(),
            mode: NetworkMode::Nat,
            ip_address: "192.168.122.20".to_string(),
            mac_address: "52:54:00:00:00:02".to_string(),
        })
        .await;

        let (allowed, _) = mgr.can_communicate("rt-a", "rt-b", 8080, "tcp").await;
        assert!(allowed);

        // Add explicit deny rule
        mgr.add_firewall_rule(NetworkRule {
            rule_id: "deny-rt-a-to-rt-b".to_string(),
            source_runtime_id: "rt-a".to_string(),
            target_runtime_id: "rt-b".to_string(),
            allowed_port: Some(8080),
            protocol: "tcp".to_string(),
            allow: false,
        })
        .await;

        let (allowed_after, _) = mgr.can_communicate("rt-a", "rt-b", 8080, "tcp").await;
        assert!(!allowed_after);
    }
}
