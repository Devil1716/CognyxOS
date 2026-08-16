use crate::graph::{ExecutionGraph, ExecutionNode, NodeState, TargetEnvironment};
use crate::planner_trait::{DeterministicPlannerProvider, Plan, PlannerProvider};
use cognyx_intent::ParsedIntent;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub struct AgentPlanner {
    provider: Arc<dyn PlannerProvider>,
}

impl Default for AgentPlanner {
    fn default() -> Self {
        Self::new(Arc::new(DeterministicPlannerProvider))
    }
}

impl AgentPlanner {
    pub fn new(provider: Arc<dyn PlannerProvider>) -> Self {
        Self { provider }
    }

    pub async fn create_plan(&self, task_id: &str, intent: &ParsedIntent) -> Result<Plan, String> {
        self.provider.create_plan(task_id, intent).await
    }

    pub fn compile_plan_to_graph(&self, plan: &Plan) -> ExecutionGraph {
        info!(
            "Compiling Plan '{}' into ExecutionGraph DAG for task '{}'",
            plan.plan_id, plan.task_id
        );

        let graph_id = format!("graph-{}", uuid::Uuid::now_v7());
        let mut graph = ExecutionGraph::new(graph_id, &plan.task_id);

        for step in &plan.steps {
            let target_env = match step.target_runtime_kind.as_str() {
                "WindowsVm" => TargetEnvironment::WindowsVm,
                "MacOsVm" => TargetEnvironment::MacOsVm,
                "Container" => TargetEnvironment::Container,
                _ => TargetEnvironment::NativeLinux,
            };

            // A graph node carries the original capability inputs.  `command`
            // remains populated for compatibility with older node consumers,
            // but is never a synthetic shell command for capability nodes.
            let capability = step
                .required_capabilities
                .first()
                .cloned()
                .unwrap_or_default();
            let command = match capability.as_str() {
                "application.search" => step.parameters.get("query"),
                "application.open" | "application.inspect" => step.parameters.get("application_id"),
                "keyboard.type" => step.parameters.get("text"),
                "window.focus" | "window.activate" | "window.close" => {
                    step.parameters.get("window_id")
                }
                "browser.navigate" => step.parameters.get("url"),
                _ => None,
            }
            .cloned()
            .unwrap_or_default();

            graph.add_node(ExecutionNode {
                node_id: step.step_id.clone(),
                task_id: plan.task_id.clone(),
                name: step.description.clone(),
                target_env,
                command,
                args: vec![],
                depends_on: step.depends_on_step_ids.clone(),
                required_capabilities: step.required_capabilities.clone(),
                constraints: step.parameters.clone(),
                state: NodeState::Pending,
                runtime_requirements: vec![],
                timeout_seconds: 60,
                retry_policy_max_retries: 3,
                env_vars: HashMap::new(),
                input_payload: String::new(),
                output_result: None,
            });
        }

        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cognyx_intent::IntentEngine;

    #[tokio::test]
    async fn test_planner_graph_compilation() {
        let planner = AgentPlanner::default();
        let engine = IntentEngine::default();

        let intent = engine.parse_prompt("Install Photoshop").await;
        let plan = planner.create_plan("task-123", &intent).await.unwrap();
        let graph = planner.compile_plan_to_graph(&plan);

        assert_eq!(graph.task_id, "task-123");
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.nodes[0].target_env, TargetEnvironment::WindowsVm);
    }
}
