use dashmap::DashMap;
use std::sync::Arc;
use crate::identity::AgentIdentity;

pub struct AgentRegistry {
    agents: DashMap<String, Arc<AgentIdentity>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: DashMap::new(),
        }
    }

    pub fn register(&self, agent: Arc<AgentIdentity>) {
        self.agents.insert(agent.agent_id.clone(), agent);
    }

    pub fn get(&self, agent_id: &str) -> Option<Arc<AgentIdentity>> {
        self.agents.get(agent_id).map(|a| a.clone())
    }

    pub fn remove(&self, agent_id: &str) {
        self.agents.remove(agent_id);
    }

    pub fn list_all(&self) -> Vec<Arc<AgentIdentity>> {
        self.agents.iter().map(|a| a.clone()).collect()
    }

    pub fn get_children(&self, parent_id: &str) -> Vec<Arc<AgentIdentity>> {
        self.agents.iter().filter(|a| a.parent_agent_id.as_deref() == Some(parent_id)).map(|a| a.clone()).collect()
    }

    pub fn get_tree(&self, root_id: &str) -> Vec<Arc<AgentIdentity>> {
        self.get_descendants(root_id)
    }

    pub fn get_descendants(&self, root_id: &str) -> Vec<Arc<AgentIdentity>> {
        let mut descendants = Vec::new();
        let mut queue = vec![root_id.to_string()];
        
        while let Some(current_id) = queue.pop() {
            let children = self.get_children(&current_id);
            for child in children {
                queue.push(child.agent_id.clone());
                descendants.push(child);
            }
        }
        
        descendants
    }
}
