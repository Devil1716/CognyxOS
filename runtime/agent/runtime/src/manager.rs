use std::sync::Arc;
use crate::registry::AgentRegistry;
use crate::lifecycle::{AgentLifecycleManager, AgentLifecycleState};
use crate::bus::AgentCommunicationBus;
use crate::identity::AgentIdentity;
use crate::role::AgentRole;
use cognyx_resources::ResourceManager;

#[allow(dead_code)]
pub struct AgentManager {
    registry: Arc<AgentRegistry>,
    lifecycle_manager: Arc<AgentLifecycleManager>,
    communication_bus: Arc<AgentCommunicationBus>,
    resource_manager: Arc<ResourceManager>,
}

impl AgentManager {
    pub fn new(
        registry: Arc<AgentRegistry>,
        lifecycle_manager: Arc<AgentLifecycleManager>,
        communication_bus: Arc<AgentCommunicationBus>,
        resource_manager: Arc<ResourceManager>,
    ) -> Self {
        Self {
            registry,
            lifecycle_manager,
            communication_bus,
            resource_manager,
        }
    }

    pub fn create_agent(
        &self,
        agent_id: impl Into<String>,
        name: impl Into<String>,
        role: AgentRole,
        parent_agent_id: Option<String>,
        task_id: impl Into<String>,
    ) -> Result<Arc<AgentIdentity>, String> {
        let aid = agent_id.into();
        let display_name = name.into();
        let agent = AgentIdentity::new(&aid, &display_name, &display_name, role, parent_agent_id, task_id);

        if let Some(ref parent) = agent.parent_agent_id {
            let children = self.registry.get_children(parent);
            if children.len() >= 8 {
                return Err("Max children exceeded".into());
            }
        }
        let arc = Arc::new(agent);
        self.registry.register(arc.clone());
        Ok(arc)
    }

    pub fn spawn_child_agent(
        &self,
        parent_id: &str,
        child_id: impl Into<String>,
        name: impl Into<String>,
        role: AgentRole,
    ) -> Result<Arc<AgentIdentity>, String> {
        let parent = self.get_agent(parent_id).ok_or_else(|| "Parent agent not found".to_string())?;
        self.create_agent(child_id, name, role, Some(parent_id.to_string()), &parent.root_agent_id)
    }

    pub fn start_agent(&self, agent_id: &str) -> Result<(), String> {
        if let Some(agent) = self.registry.get(agent_id) {
            let mut updated = (*agent).clone();
            updated.status = AgentLifecycleState::Running;
            self.registry.register(Arc::new(updated));
            Ok(())
        } else {
            Err("Agent not found".into())
        }
    }

    pub fn pause_agent(&self, agent_id: &str) -> Result<(), String> {
        if let Some(agent) = self.registry.get(agent_id) {
            let mut updated = (*agent).clone();
            updated.status = AgentLifecycleState::Paused;
            self.registry.register(Arc::new(updated));
            Ok(())
        } else {
            Err("Agent not found".into())
        }
    }

    pub fn resume_agent(&self, agent_id: &str) -> Result<(), String> {
        self.start_agent(agent_id)
    }

    pub fn stop_agent(&self, agent_id: &str) -> Result<(), String> {
        if let Some(agent) = self.registry.get(agent_id) {
            let mut updated = (*agent).clone();
            updated.status = AgentLifecycleState::Stopped;
            self.registry.register(Arc::new(updated));
            Ok(())
        } else {
            Err("Agent not found".into())
        }
    }

    pub fn fail_agent(&self, agent_id: &str, error_message: impl Into<String>) -> Result<(), String> {
        if let Some(agent) = self.registry.get(agent_id) {
            let mut updated = (*agent).clone();
            updated.status = AgentLifecycleState::Failed(error_message.into());
            self.registry.register(Arc::new(updated));
            Ok(())
        } else {
            Err("Agent not found".into())
        }
    }

    pub fn recover_agent(&self, agent_id: &str) -> Result<(), String> {
        if let Some(agent) = self.registry.get(agent_id) {
            let mut updated = (*agent).clone();
            updated.status = AgentLifecycleState::Ready;
            self.registry.register(Arc::new(updated));
            Ok(())
        } else {
            Err("Agent not found".into())
        }
    }

    pub fn terminate_agent(&self, agent_id: &str) -> Result<(), String> {
        if let Some(agent) = self.registry.get(agent_id) {
            let mut updated = (*agent).clone();
            updated.status = AgentLifecycleState::Terminated;
            self.registry.register(Arc::new(updated));
            Ok(())
        } else {
            Err("Agent not found".into())
        }
    }

    pub fn cancel_tree(&self, root_id: &str) -> Result<(), String> {
        let tree = self.registry.get_tree(root_id);
        for agent in tree {
            let _ = self.terminate_agent(&agent.agent_id);
        }
        let _ = self.terminate_agent(root_id);
        Ok(())
    }

    pub fn get_agent(&self, agent_id: &str) -> Option<Arc<AgentIdentity>> {
        self.registry.get(agent_id)
    }

    pub fn list_agents(&self) -> Vec<Arc<AgentIdentity>> {
        self.registry.list_all()
    }

    pub fn inspect_agent(&self, agent_id: &str) -> Option<Arc<AgentIdentity>> {
        self.get_agent(agent_id)
    }

    pub fn get_agent_tree(&self, root_id: &str) -> Vec<Arc<AgentIdentity>> {
        self.registry.get_tree(root_id)
    }
}
