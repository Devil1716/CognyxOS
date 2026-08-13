use crate::role::AgentRole;
use cognyx_intent::Intent;

#[derive(Debug, Clone)]
pub struct SubtaskAssignment {
    pub subtask_id: String,
    pub description: String,
    pub assigned_role: AgentRole,
    pub required_capabilities: Vec<String>,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MultiAgentPlan {
    pub task_id: String,
    pub subtasks: Vec<SubtaskAssignment>,
}

#[derive(Default)]
pub struct MultiAgentPlanner;

impl MultiAgentPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn create_multi_agent_plan(&self, task_id: impl Into<String>, _intent: &Intent) -> MultiAgentPlan {
        let tid = task_id.into();
        MultiAgentPlan {
            task_id: tid,
            subtasks: vec![
                SubtaskAssignment {
                    subtask_id: "subtask-file".into(),
                    description: "Discover and read test documents".into(),
                    assigned_role: AgentRole::FileOperator,
                    required_capabilities: vec!["filesystem.read".into()],
                    depends_on: vec![],
                },
                SubtaskAssignment {
                    subtask_id: "subtask-research".into(),
                    description: "Research topic context".into(),
                    assigned_role: AgentRole::Researcher,
                    required_capabilities: vec!["browser.read".into()],
                    depends_on: vec![],
                },
                SubtaskAssignment {
                    subtask_id: "subtask-writer".into(),
                    description: "Synthesize findings into final report".into(),
                    assigned_role: AgentRole::Writer,
                    required_capabilities: vec!["filesystem.write".into()],
                    depends_on: vec!["subtask-file".into(), "subtask-research".into()],
                },
            ],
        }
    }
}
