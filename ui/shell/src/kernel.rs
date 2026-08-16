use crate::error::{ShellError, ShellResult};
use crate::model::{AgentNode, TaskView};
use async_trait::async_trait;
use cognyx_agent_kernel::AgentKernelServer;
use cognyx_proto::cognyx::services::agent::v1::agent_kernel_service_server::AgentKernelService;
use cognyx_proto::cognyx::services::agent::v1::{SubmitTaskRequest, TaskHandle};
use cognyx_task_manager::TaskStatus;
use dashmap::DashMap;
use std::sync::{Arc, Mutex};
use tonic::Request;

/// The shell talks to the Agent Kernel through this trait.
/// It must not plan, schedule, or execute capabilities itself.
#[async_trait]
pub trait KernelClient: Send + Sync {
    async fn submit_intent(&self, prompt: &str) -> ShellResult<TaskView>;
    async fn inspect_task(&self, task_id: &str) -> ShellResult<TaskView>;
    async fn inspect_agent(&self, agent_id: &str) -> ShellResult<AgentNode>;
    async fn recover_task(&self, task_id: &str) -> ShellResult<TaskView>;
    async fn agent_tree(&self, task_id: &str) -> ShellResult<AgentNode>;
}

/// Production adapter. Reuses `AgentKernelServer` (does not duplicate it).
/// The shell submits intents; it never executes OS actions itself.
pub struct AgentKernelAdapter {
    inner: Arc<AgentKernelServer>,
}

impl Default for AgentKernelAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentKernelAdapter {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AgentKernelServer::new()),
        }
    }

    pub fn from_server(server: Arc<AgentKernelServer>) -> Self {
        Self { inner: server }
    }

    pub fn server(&self) -> Arc<AgentKernelServer> {
        Arc::clone(&self.inner)
    }
}

fn task_view(task: &cognyx_task_manager::AgentTask, prompt_fallback: Option<&str>) -> TaskView {
    let (status, error) = match &task.status {
        TaskStatus::Failed(e) => ("failed".to_string(), Some(e.clone())),
        TaskStatus::Completed => ("completed".to_string(), None),
        TaskStatus::Cancelled => ("cancelled".to_string(), None),
        TaskStatus::Paused => ("paused".to_string(), None),
        TaskStatus::Created | TaskStatus::Planning | TaskStatus::Ready => {
            ("pending".to_string(), None)
        }
        TaskStatus::Recovering => ("recovering".to_string(), None),
        TaskStatus::Waiting | TaskStatus::Blocked => ("waiting".to_string(), None),
        TaskStatus::Running => ("running".to_string(), task.error.clone()),
    };
    TaskView {
        task_id: task.task_id.clone(),
        prompt: if task.prompt.is_empty() {
            prompt_fallback.unwrap_or("").to_string()
        } else {
            task.prompt.clone()
        },
        status,
        runtime_id: task.assigned_runtime.clone(),
        error: error.or(task.error.clone()),
    }
}

#[async_trait]
impl KernelClient for AgentKernelAdapter {
    async fn submit_intent(&self, prompt: &str) -> ShellResult<TaskView> {
        let resp = AgentKernelService::submit_task(
            self.inner.as_ref(),
            Request::new(SubmitTaskRequest {
                meta: None,
                cap: None,
                prompt: prompt.to_string(),
                priority: 1,
            }),
        )
        .await
        .map_err(|e| ShellError::Kernel(e.to_string()))?;
        let handle = resp.into_inner();
        match self.inner.task_manager.get_task(&handle.task_id) {
            Ok(task) => Ok(task_view(&task, Some(prompt))),
            Err(_) => Ok(TaskView {
                task_id: handle.task_id,
                prompt: prompt.to_string(),
                status: "running".into(),
                runtime_id: None,
                error: None,
            }),
        }
    }

    async fn inspect_task(&self, task_id: &str) -> ShellResult<TaskView> {
        let task = self
            .inner
            .task_manager
            .get_task(task_id)
            .map_err(|e| ShellError::NotFound(e.to_string()))?;
        Ok(task_view(&task, None))
    }

    async fn inspect_agent(&self, agent_id: &str) -> ShellResult<AgentNode> {
        // The kernel does not currently expose a separate agent tree service.
        // Return an honest kernel node for the matching task, not a fabricated hierarchy.
        if let Some(task_id) = agent_id.strip_prefix("kernel-") {
            return self.agent_tree(task_id).await;
        }
        Err(ShellError::NotFound(agent_id.to_string()))
    }

    async fn recover_task(&self, task_id: &str) -> ShellResult<TaskView> {
        let _ = AgentKernelService::recover_task(
            self.inner.as_ref(),
            Request::new(TaskHandle {
                task_id: task_id.to_string(),
                status: 0,
                submitted_at: None,
            }),
        )
        .await
        .map_err(|e| ShellError::Kernel(e.to_string()))?;
        self.inspect_task(task_id).await
    }

    async fn agent_tree(&self, task_id: &str) -> ShellResult<AgentNode> {
        let task = self.inspect_task(task_id).await?;
        Ok(AgentNode {
            agent_id: format!("kernel-{}", task.task_id),
            role: "kernel".into(),
            status: task.status,
            runtime_id: task.runtime_id,
            operation: Some(task.prompt),
            children: vec![],
        })
    }
}

/// TEST ONLY. Do not wire this into production `main.rs`.
/// Records prompts and fabricates task/agent views. It does not execute
/// capabilities and is not a stand-in for `AgentKernelServer`.
pub struct RecordingKernel {
    pub submitted: Mutex<Vec<String>>,
    tasks: DashMap<String, TaskView>,
    agents: DashMap<String, AgentNode>,
    fail_next: Mutex<bool>,
}

impl Default for RecordingKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl RecordingKernel {
    pub fn new() -> Self {
        Self {
            submitted: Mutex::new(Vec::new()),
            tasks: DashMap::new(),
            agents: DashMap::new(),
            fail_next: Mutex::new(false),
        }
    }

    pub fn fail_next_submit(&self) {
        *self.fail_next.lock().unwrap() = true;
    }

    pub fn submitted_prompts(&self) -> Vec<String> {
        self.submitted.lock().unwrap().clone()
    }
}

#[async_trait]
impl KernelClient for RecordingKernel {
    async fn submit_intent(&self, prompt: &str) -> ShellResult<TaskView> {
        self.submitted.lock().unwrap().push(prompt.to_string());
        if *self.fail_next.lock().unwrap() {
            *self.fail_next.lock().unwrap() = false;
            let task = TaskView {
                task_id: format!("task-{}", self.submitted.lock().unwrap().len()),
                prompt: prompt.to_string(),
                status: "failed".into(),
                runtime_id: None,
                error: Some("simulated kernel failure".into()),
            };
            self.tasks.insert(task.task_id.clone(), task.clone());
            return Ok(task);
        }
        let task = TaskView {
            task_id: format!("task-{}", self.submitted.lock().unwrap().len()),
            prompt: prompt.to_string(),
            status: "running".into(),
            runtime_id: Some("linux-host".into()),
            error: None,
        };
        let tree = AgentNode {
            agent_id: format!("agent-{}", task.task_id),
            role: "manager".into(),
            status: "running".into(),
            runtime_id: Some("linux-host".into()),
            operation: Some(prompt.to_string()),
            children: vec![AgentNode {
                agent_id: format!("agent-{}-file", task.task_id),
                role: "file".into(),
                status: "running".into(),
                runtime_id: Some("linux-host".into()),
                operation: Some("workspace.search".into()),
                children: vec![],
            }],
        };
        self.agents.insert(task.task_id.clone(), tree);
        self.tasks.insert(task.task_id.clone(), task.clone());
        Ok(task)
    }

    async fn inspect_task(&self, task_id: &str) -> ShellResult<TaskView> {
        self.tasks
            .get(task_id)
            .map(|t| t.clone())
            .ok_or_else(|| crate::error::ShellError::NotFound(task_id.to_string()))
    }

    async fn inspect_agent(&self, agent_id: &str) -> ShellResult<AgentNode> {
        for entry in self.agents.iter() {
            if entry.value().agent_id == agent_id {
                return Ok(entry.value().clone());
            }
            for child in &entry.value().children {
                if child.agent_id == agent_id {
                    return Ok(child.clone());
                }
            }
        }
        Err(crate::error::ShellError::NotFound(agent_id.to_string()))
    }

    async fn recover_task(&self, task_id: &str) -> ShellResult<TaskView> {
        if let Some(mut t) = self.tasks.get_mut(task_id) {
            t.status = "running".into();
            t.error = None;
            return Ok(t.clone());
        }
        Err(crate::error::ShellError::NotFound(task_id.to_string()))
    }

    async fn agent_tree(&self, task_id: &str) -> ShellResult<AgentNode> {
        self.agents
            .get(task_id)
            .map(|t| t.clone())
            .ok_or_else(|| crate::error::ShellError::NotFound(task_id.to_string()))
    }
}
