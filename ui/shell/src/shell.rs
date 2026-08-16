use crate::error::{ShellError, ShellResult};
use crate::kernel::KernelClient;
use crate::model::*;
use cognyx_service_workspace::{WorkspaceManager, WorkspaceResult};
use dashmap::DashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct CognyxShell<K: KernelClient> {
    kernel: Arc<K>,
    workspace: Arc<WorkspaceManager>,
    windows: DashMap<String, Window>,
    notifications: Mutex<Vec<Notification>>,
    approvals: DashMap<String, ApprovalRequest>,
    task_grants: DashMap<String, Vec<String>>,
    frames: Mutex<Vec<ComputerUseFrame>>,
    desktop: Mutex<Desktop>,
    dock: Mutex<Vec<String>>,
}

impl<K: KernelClient> CognyxShell<K> {
    pub fn new(kernel: Arc<K>, workspace: Arc<WorkspaceManager>, workspace_id: String) -> Self {
        Self {
            kernel,
            workspace,
            windows: DashMap::new(),
            notifications: Mutex::new(Vec::new()),
            approvals: DashMap::new(),
            task_grants: DashMap::new(),
            frames: Mutex::new(Vec::new()),
            desktop: Mutex::new(Desktop {
                workspace_id,
                focused_window: None,
            }),
            dock: Mutex::new(vec![
                "launcher".into(),
                "command-bar".into(),
                "agent-panel".into(),
            ]),
        }
    }

    pub fn workspace(&self) -> Arc<WorkspaceManager> {
        Arc::clone(&self.workspace)
    }

    pub fn desktop(&self) -> Desktop {
        self.desktop.lock().unwrap().clone()
    }

    pub fn dock(&self) -> Vec<String> {
        self.dock.lock().unwrap().clone()
    }

    pub fn launcher_apps(&self) -> Vec<String> {
        self.windows
            .iter()
            .map(|w| w.application_id.clone())
            .collect()
    }

    /// Natural-language command bar. Forwards to the Agent Kernel.
    pub async fn submit_intent(&self, prompt: &str) -> ShellResult<TaskView> {
        let task = self.kernel.submit_intent(prompt).await?;
        if task.status == "failed" {
            self.notify(
                NotificationKind::AgentFailed,
                "Agent failed",
                task.error.as_deref().unwrap_or("task failed"),
                &task.task_id,
            );
        }
        Ok(task)
    }

    pub async fn inspect_task(&self, task_id: &str) -> ShellResult<TaskView> {
        self.kernel.inspect_task(task_id).await
    }

    pub async fn inspect_agent(&self, agent_id: &str) -> ShellResult<AgentNode> {
        self.kernel.inspect_agent(agent_id).await
    }

    pub async fn agent_tree(&self, task_id: &str) -> ShellResult<AgentNode> {
        self.kernel.agent_tree(task_id).await
    }

    pub async fn recover_task(&self, task_id: &str) -> ShellResult<TaskView> {
        let task = self.kernel.recover_task(task_id).await?;
        self.notify(
            NotificationKind::SystemWarning,
            "Task recovered",
            "Kernel resumed the task",
            task_id,
        );
        Ok(task)
    }

    pub fn request_approval(
        &self,
        task_id: &str,
        capability: &str,
        reason: &str,
        resource: &str,
        risk: RiskLevel,
    ) -> ApprovalRequest {
        let req = ApprovalRequest {
            id: format!("appr-{}", uuid::Uuid::now_v7()),
            task_id: task_id.to_string(),
            capability: capability.to_string(),
            reason: reason.to_string(),
            resource: resource.to_string(),
            risk,
            decided: None,
        };
        self.approvals.insert(req.id.clone(), req.clone());
        self.notify(
            NotificationKind::ApprovalRequired,
            "Approval required",
            reason,
            &req.id,
        );
        req
    }

    pub fn decide_approval(
        &self,
        approval_id: &str,
        decision: ApprovalDecision,
    ) -> ShellResult<ApprovalRequest> {
        let mut req = self
            .approvals
            .get(approval_id)
            .map(|r| r.clone())
            .ok_or_else(|| ShellError::NotFound(approval_id.to_string()))?;
        if req.decided.is_some() {
            return Ok(req);
        }
        req.decided = Some(decision.clone());
        match decision {
            ApprovalDecision::Deny => {
                self.approvals.insert(req.id.clone(), req.clone());
                return Err(ShellError::Denied(req.capability.clone()));
            }
            ApprovalDecision::AllowOnce | ApprovalDecision::AllowForTask => {
                if decision == ApprovalDecision::AllowForTask {
                    self.task_grants
                        .entry(req.task_id.clone())
                        .or_default()
                        .push(req.capability.clone());
                }
            }
        }
        self.approvals.insert(req.id.clone(), req.clone());
        Ok(req)
    }

    pub fn pending_approvals(&self) -> Vec<ApprovalRequest> {
        self.approvals
            .iter()
            .map(|e| e.value().clone())
            .filter(|a| a.decided.is_none())
            .collect()
    }

    pub fn open_window(&self, application_id: &str, runtime_id: &str, title: &str) -> Window {
        let workspace = self.desktop.lock().unwrap().workspace_id.clone();
        let window = Window {
            window_id: format!("win-{}", uuid::Uuid::now_v7()),
            application_id: application_id.to_string(),
            runtime_id: runtime_id.to_string(),
            title: title.to_string(),
            bounds: (40, 40, 1280, 720),
            state: WindowState::Normal,
            focus: true,
            workspace,
        };
        for mut w in self.windows.iter_mut() {
            w.focus = false;
        }
        self.windows
            .insert(window.window_id.clone(), window.clone());
        self.desktop.lock().unwrap().focused_window = Some(window.window_id.clone());
        window
    }

    pub fn focus_window(&self, window_id: &str) -> ShellResult<Window> {
        if !self.windows.contains_key(window_id) {
            return Err(ShellError::NotFound(window_id.to_string()));
        }
        for mut w in self.windows.iter_mut() {
            w.focus = w.window_id == window_id;
        }
        self.desktop.lock().unwrap().focused_window = Some(window_id.to_string());
        Ok(self.windows.get(window_id).unwrap().clone())
    }

    pub fn close_window(&self, window_id: &str) -> ShellResult<()> {
        self.windows
            .remove(window_id)
            .ok_or_else(|| ShellError::NotFound(window_id.to_string()))?;
        Ok(())
    }

    pub fn windows(&self) -> Vec<Window> {
        self.windows.iter().map(|w| w.clone()).collect()
    }

    pub fn observe_frame(&self, frame: ComputerUseFrame) {
        self.frames.lock().unwrap().push(frame);
    }

    pub fn computer_use_frames(&self) -> Vec<ComputerUseFrame> {
        self.frames.lock().unwrap().clone()
    }

    pub fn switch_workspace(&self, workspace_id: &str) -> WorkspaceResult<()> {
        self.workspace.get_workspace(workspace_id)?;
        self.desktop.lock().unwrap().workspace_id = workspace_id.to_string();
        Ok(())
    }

    pub fn search_workspace(&self, query: &str) -> Vec<cognyx_service_workspace::WorkspaceItem> {
        self.workspace.search(query)
    }

    pub fn notify(
        &self,
        kind: NotificationKind,
        title: &str,
        body: &str,
        subject: &str,
    ) -> Option<Notification> {
        let mut notes = self.notifications.lock().unwrap();
        if notes.iter().any(|n| n.kind == kind && n.subject == subject) {
            return None;
        }
        let n = Notification {
            id: format!("ntf-{}", uuid::Uuid::now_v7()),
            kind,
            title: title.to_string(),
            body: body.to_string(),
            subject: subject.to_string(),
        };
        notes.push(n.clone());
        Some(n)
    }

    pub fn notifications(&self) -> Vec<Notification> {
        self.notifications.lock().unwrap().clone()
    }

    pub fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}
