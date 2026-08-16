use cognyx_execution::RuntimeRegistry;
use cognyx_planner::{ExecutionGraph, ExecutionNode, NodeState};
use cognyx_resources::ResourceManager;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::info;

pub struct ScheduledNodeAssignment {
    pub node: ExecutionNode,
    pub assigned_runtime_id: String,
}

pub struct GraphScheduler {
    registry: Arc<RuntimeRegistry>,
    _resource_manager: Arc<ResourceManager>,
}

impl GraphScheduler {
    pub fn new(registry: Arc<RuntimeRegistry>, resource_manager: Arc<ResourceManager>) -> Self {
        Self {
            registry,
            _resource_manager: resource_manager,
        }
    }

    pub fn get_ready_nodes(
        &self,
        graph: &ExecutionGraph,
        completed_node_ids: &HashSet<String>,
    ) -> Vec<ExecutionNode> {
        let mut ready = vec![];

        for node in &graph.nodes {
            if completed_node_ids.contains(&node.node_id) || node.state == NodeState::Completed {
                continue;
            }

            let dependencies_met = node
                .depends_on
                .iter()
                .all(|dep_id| completed_node_ids.contains(dep_id));

            if dependencies_met {
                ready.push(node.clone());
            }
        }

        info!(
            "Scheduler identified {} ready nodes for graph '{}'",
            ready.len(),
            graph.graph_id
        );
        ready
    }

    pub async fn schedule_node(
        &self,
        node: &ExecutionNode,
    ) -> Result<ScheduledNodeAssignment, String> {
        info!(
            "Scheduling node '{}' requiring capabilities: {:?}",
            node.node_id, node.required_capabilities
        );

        let required_cap = node
            .required_capabilities
            .first()
            .cloned()
            .unwrap_or_else(|| "bash".to_string());

        let matched = self
            .registry
            .find_runtime_for_capability(&required_cap)
            .await;

        let runtime_id = matched.unwrap_or_else(|| {
            info!(
                "No registered runtime explicitly match cap '{}', returning fallback simulator",
                required_cap
            );
            format!("sim-runtime-{:?}", node.target_env)
        });

        Ok(ScheduledNodeAssignment {
            node: node.clone(),
            assigned_runtime_id: runtime_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cognyx_execution::LinuxRuntime;
    use cognyx_planner::TargetEnvironment;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_scheduler_runtime_discovery() {
        let registry = Arc::new(RuntimeRegistry::new());
        let linux = Box::new(LinuxRuntime::new("linux-host-1", "Local Host"));
        registry.register(linux);

        let res_mgr = Arc::new(ResourceManager::default());
        let scheduler = GraphScheduler::new(registry, res_mgr);

        let node = ExecutionNode {
            node_id: "node-1".to_string(),
            task_id: "task-1".to_string(),
            name: "Run bash command".to_string(),
            target_env: TargetEnvironment::NativeLinux,
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            depends_on: vec![],
            required_capabilities: vec!["bash".to_string()],
            constraints: HashMap::new(),
            state: NodeState::Pending,
            runtime_requirements: vec![],
            timeout_seconds: 30,
            retry_policy_max_retries: 3,
            env_vars: HashMap::new(),
            input_payload: String::new(),
            output_result: None,
        };

        let assignment = scheduler.schedule_node(&node).await.unwrap();
        assert_eq!(assignment.assigned_runtime_id, "linux-host-1");
    }
}
