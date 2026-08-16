use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Window {
    pub window_id: String,
    pub application_id: String,
    pub runtime_id: String,
    pub title: String,
    pub bounds: (i32, i32, u32, u32),
    pub state: WindowState,
    pub focus: bool,
    pub workspace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Hidden,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationKind {
    TaskCompleted,
    ApprovalRequired,
    AgentFailed,
    RuntimeUnavailable,
    DownloadComplete,
    WorkspaceConflict,
    SecurityWarning,
    SystemWarning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Notification {
    pub id: String,
    pub kind: NotificationKind,
    pub title: String,
    pub body: String,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalDecision {
    AllowOnce,
    AllowForTask,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalRequest {
    pub id: String,
    pub task_id: String,
    pub capability: String,
    pub reason: String,
    pub resource: String,
    pub risk: RiskLevel,
    pub decided: Option<ApprovalDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskView {
    pub task_id: String,
    pub prompt: String,
    pub status: String,
    pub runtime_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentNode {
    pub agent_id: String,
    pub role: String,
    pub status: String,
    pub runtime_id: Option<String>,
    pub operation: Option<String>,
    pub children: Vec<AgentNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputerUseFrame {
    pub runtime_id: String,
    pub application: String,
    pub kind: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Desktop {
    pub workspace_id: String,
    pub focused_window: Option<String>,
}
