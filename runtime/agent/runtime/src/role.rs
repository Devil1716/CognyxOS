use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AgentRole {
    Manager,
    Planner,
    Researcher,
    ComputerOperator,
    FileOperator,
    BrowserOperator,
    Analyst,
    Writer,
    Validator,
    Worker,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolePolicy {
    pub default_capability_scopes: Vec<String>,
    pub default_permissions: Vec<String>,
}

impl AgentRole {
    pub fn allowed_capabilities(&self) -> Vec<String> {
        self.default_role_policy().default_capability_scopes
    }

    pub fn default_role_policy(&self) -> RolePolicy {
        match self {
            AgentRole::Manager => RolePolicy {
                default_capability_scopes: vec!["*".to_string()],
                default_permissions: vec!["*".to_string()],
            },
            _ => RolePolicy {
                default_capability_scopes: vec![],
                default_permissions: vec![],
            }
        }
    }
}
