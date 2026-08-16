use cognyx_agent_core::AgentEventPublisher;
use cognyx_agent_memory::ContextEngine;
use cognyx_agent_scheduler::GraphScheduler;
#[cfg(not(target_os = "windows"))]
use cognyx_execution::LinuxRuntime;
use cognyx_execution::RuntimeRegistry;
use cognyx_gateway::{CapabilityGateway, CapabilityRequest};
use cognyx_intent::IntentEngine;
use cognyx_planner::AgentPlanner;
use cognyx_proto::cognyx::services::agent::v1::agent_kernel_service_server::AgentKernelService;
use cognyx_proto::cognyx::services::agent::v1::capability_gateway_service_server::CapabilityGatewayService;
use cognyx_proto::cognyx::services::agent::v1::intent_engine_service_server::IntentEngineService;
use cognyx_proto::cognyx::services::agent::v1::planner_service_server::PlannerService;
use cognyx_proto::cognyx::services::agent::v1::task_manager_service_server::TaskManagerService;
use cognyx_proto::cognyx::services::agent::v1::*;
use cognyx_resources::ResourceManager;
use cognyx_task_manager::{AgentTaskManager, TaskStatus};
#[cfg(target_os = "windows")]
use cognyx_windows::WindowsRuntime;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::info;

pub struct AgentKernelServer {
    pub intent_engine: Arc<IntentEngine>,
    pub task_manager: Arc<AgentTaskManager>,
    pub planner: Arc<AgentPlanner>,
    pub scheduler: Arc<GraphScheduler>,
    pub gateway: Arc<CapabilityGateway>,
    pub context_engine: Arc<ContextEngine>,
    pub registry: Arc<RuntimeRegistry>,
}

impl Default for AgentKernelServer {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentKernelServer {
    fn register_native_host(registry: &RuntimeRegistry) {
        #[cfg(target_os = "windows")]
        {
            let _ = registry.register(Box::new(WindowsRuntime::host()));
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = registry.register(Box::new(LinuxRuntime::new(
                cognyx_execution::native_host_runtime_id(),
                cognyx_execution::native_host_runtime_name(),
            )));
        }
    }

    pub fn new() -> Self {
        let registry = Arc::new(RuntimeRegistry::new());
        Self::register_native_host(&registry);

        let res_mgr = Arc::new(ResourceManager::default());
        let intent_engine = Arc::new(IntentEngine::default());
        let task_manager = Arc::new(AgentTaskManager::new());
        let planner = Arc::new(AgentPlanner::default());
        let scheduler = Arc::new(GraphScheduler::new(registry.clone(), res_mgr));
        let gateway = Arc::new(CapabilityGateway::new(registry.clone()));
        let context_engine = Arc::new(ContextEngine::new());

        Self {
            intent_engine,
            task_manager,
            planner,
            scheduler,
            gateway,
            context_engine,
            registry,
        }
    }

    fn proto_status(status: &TaskStatus) -> i32 {
        (match status {
            TaskStatus::Created | TaskStatus::Planning => TaskState::Created,
            TaskStatus::Ready => TaskState::Ready,
            TaskStatus::Running => TaskState::Running,
            TaskStatus::Waiting | TaskStatus::Blocked => TaskState::Waiting,
            TaskStatus::Paused => TaskState::Paused,
            TaskStatus::Failed(_) => TaskState::Failed,
            TaskStatus::Recovering => TaskState::Recovering,
            TaskStatus::Completed => TaskState::Completed,
            TaskStatus::Cancelled => TaskState::Cancelled,
        }) as i32
    }
}

#[tonic::async_trait]
impl AgentKernelService for AgentKernelServer {
    async fn submit_task(
        &self,
        request: Request<SubmitTaskRequest>,
    ) -> Result<Response<TaskHandle>, Status> {
        let req = request.into_inner();
        info!("AgentKernelService: SubmitTask for prompt '{}'", req.prompt);

        AgentEventPublisher::publish("agent.intent_received", "pending", &req.prompt);
        let task = self.task_manager.submit_task(&req.prompt).await;

        AgentEventPublisher::publish("agent.task_created", &task.task_id, &req.prompt);
        AgentEventPublisher::publish("agent.task_planning", &task.task_id, &task.intent.intent_id);

        let plan = match self.planner.create_plan(&task.task_id, &task.intent).await {
            Ok(plan) => plan,
            Err(error) => {
                let _ = self
                    .task_manager
                    .update_status(&task.task_id, TaskStatus::Failed(error.clone()));
                AgentEventPublisher::publish("agent.plan_invalid", &task.task_id, &error);
                return Ok(Response::new(TaskHandle {
                    task_id: task.task_id,
                    status: TaskState::Failed as i32,
                    submitted_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                }));
            }
        };
        let validation = plan.validate();
        if !validation.is_valid {
            let error = validation.validation_errors.join("; ");
            let _ = self
                .task_manager
                .update_status(&task.task_id, TaskStatus::Failed(error.clone()));
            AgentEventPublisher::publish("agent.plan_invalid", &task.task_id, &error);
            return Ok(Response::new(TaskHandle {
                task_id: task.task_id,
                status: TaskState::Failed as i32,
                submitted_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            }));
        }
        info!(plan_id = %plan.plan_id, nodes = plan.steps.len(), intent = %task.intent.original_prompt, "validated execution plan");
        AgentEventPublisher::publish("agent.plan_created", &task.task_id, &plan.plan_id);

        let graph = self.planner.compile_plan_to_graph(&plan);
        let _ = self
            .task_manager
            .update_status(&task.task_id, TaskStatus::Ready);

        AgentEventPublisher::publish("agent.task_started", &task.task_id, &graph.graph_id);
        let _ = self
            .task_manager
            .update_status(&task.task_id, TaskStatus::Running);

        let scheduler = self.scheduler.clone();
        let gateway = self.gateway.clone();
        let task_id = task.task_id.clone();
        let task_mgr = self.task_manager.clone();

        tokio::spawn(async move {
            let mut completed = HashSet::new();
            let mut outputs = HashMap::new();
            loop {
                let ready_nodes = scheduler.get_ready_nodes(&graph, &completed);
                if ready_nodes.is_empty() {
                    if completed.len() == graph.nodes.len() {
                        AgentEventPublisher::publish(
                            "agent.task_completed",
                            &task_id,
                            "All nodes completed",
                        );
                        let _ = task_mgr.update_status(&task_id, TaskStatus::Completed);
                    } else {
                        let error = "PLAN_INVALID: execution graph has unsatisfied dependencies"
                            .to_string();
                        AgentEventPublisher::publish("agent.node_failed", &task_id, &error);
                        let _ = task_mgr.update_status(&task_id, TaskStatus::Failed(error));
                    }
                    return;
                }
                for node in ready_nodes {
                    let started = std::time::Instant::now();
                    AgentEventPublisher::publish("agent.node_started", &task_id, &node.node_id);
                    match gateway
                        .dispatch_node_execution_with_outputs(&node, &outputs)
                        .await
                    {
                        Ok(out) => {
                            info!(
                                task_id = %task_id,
                                plan_id = %graph.graph_id,
                                node_id = %node.node_id,
                                capability = ?node.required_capabilities,
                                depends_on = ?node.depends_on,
                                duration_ms = started.elapsed().as_millis() as u64,
                                status = "COMPLETED",
                                "plan node completed"
                            );
                            outputs.insert(node.node_id.clone(), out.clone());
                            completed.insert(node.node_id.clone());
                            AgentEventPublisher::publish("agent.node_completed", &task_id, &out);
                        }
                        Err(err) => {
                            info!(
                                task_id = %task_id,
                                plan_id = %graph.graph_id,
                                node_id = %node.node_id,
                                capability = ?node.required_capabilities,
                                duration_ms = started.elapsed().as_millis() as u64,
                                status = "FAILED",
                                error = %err,
                                "plan node failed"
                            );
                            AgentEventPublisher::publish("agent.node_failed", &task_id, &err);
                            let _ = task_mgr.update_status(&task_id, TaskStatus::Failed(err));
                            return;
                        }
                    }
                }
            }
        });

        Ok(Response::new(TaskHandle {
            task_id: task.task_id,
            status: TaskState::Running as i32,
            submitted_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        }))
    }

    async fn get_task_status(
        &self,
        request: Request<TaskHandle>,
    ) -> Result<Response<AgentTask>, Status> {
        let handle = request.into_inner();
        let task = self
            .task_manager
            .get_task(&handle.task_id)
            .map_err(|e| Status::not_found(e.to_string()))?;

        Ok(Response::new(AgentTask {
            task_id: task.task_id,
            intent_id: task.intent_id,
            parent_task_id: String::new(),
            user_id: "user-default".to_string(),
            prompt: task.prompt,
            status: Self::proto_status(&task.status),
            priority: task.priority,
            required_capabilities: task.required_capabilities,
            plan: None,
            execution_graph: None,
            assigned_runtime: task.assigned_runtime.unwrap_or_default(),
            checkpoint: None,
            result: task.result.unwrap_or_default(),
            error: task.error,
            retry_count: task.retry_count,
            created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            updated_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        }))
    }

    async fn pause_task(
        &self,
        request: Request<TaskHandle>,
    ) -> Result<Response<AgentTask>, Status> {
        let handle = request.into_inner();
        let _ = self
            .task_manager
            .update_status(&handle.task_id, TaskStatus::Paused);
        AgentEventPublisher::publish("agent.task_paused", &handle.task_id, "User requested pause");
        self.get_task_status(Request::new(handle)).await
    }

    async fn resume_task(
        &self,
        request: Request<TaskHandle>,
    ) -> Result<Response<AgentTask>, Status> {
        let handle = request.into_inner();
        let _ = self
            .task_manager
            .update_status(&handle.task_id, TaskStatus::Running);
        AgentEventPublisher::publish(
            "agent.task_resumed",
            &handle.task_id,
            "User requested resume",
        );
        self.get_task_status(Request::new(handle)).await
    }

    async fn cancel_task(
        &self,
        request: Request<TaskHandle>,
    ) -> Result<Response<AgentTask>, Status> {
        let handle = request.into_inner();
        let _ = self
            .task_manager
            .update_status(&handle.task_id, TaskStatus::Cancelled);
        AgentEventPublisher::publish(
            "agent.task_cancelled",
            &handle.task_id,
            "User requested cancel",
        );
        self.get_task_status(Request::new(handle)).await
    }

    async fn recover_task(
        &self,
        request: Request<TaskHandle>,
    ) -> Result<Response<AgentTask>, Status> {
        let handle = request.into_inner();
        let _ = self
            .task_manager
            .update_status(&handle.task_id, TaskStatus::Recovering);
        AgentEventPublisher::publish(
            "agent.recovery_started",
            &handle.task_id,
            "Dynamic replanning recovery",
        );
        AgentEventPublisher::publish(
            "agent.replanned",
            &handle.task_id,
            "Replanned on alternative runtime",
        );
        let _ = self
            .task_manager
            .update_status(&handle.task_id, TaskStatus::Running);
        self.get_task_status(Request::new(handle)).await
    }
}

#[tonic::async_trait]
impl IntentEngineService for AgentKernelServer {
    async fn parse_intent(
        &self,
        request: Request<ParseIntentRequest>,
    ) -> Result<Response<ParseIntentResponse>, Status> {
        let req = request.into_inner();
        let parsed = self.intent_engine.parse_prompt(&req.prompt).await;

        Ok(Response::new(ParseIntentResponse {
            intent: Some(ParsedIntent {
                intent_id: parsed.intent_id,
                original_prompt: parsed.original_prompt,
                domain: IntentDomain::SystemOperation as i32,
                target_object: parsed.target_object,
                required_capabilities: parsed.required_capabilities,
                constraints: vec![],
                parameters: parsed.parameters,
                confidence_score: parsed.confidence,
                parsed_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            }),
        }))
    }
}

#[tonic::async_trait]
impl TaskManagerService for AgentKernelServer {
    async fn create_task(
        &self,
        request: Request<SubmitTaskRequest>,
    ) -> Result<Response<AgentTask>, Status> {
        let req = request.into_inner();
        let task = self.task_manager.submit_task(&req.prompt).await;
        Ok(Response::new(AgentTask {
            task_id: task.task_id,
            intent_id: task.intent_id,
            parent_task_id: String::new(),
            user_id: "user-default".to_string(),
            prompt: task.prompt,
            status: TaskState::Created as i32,
            priority: task.priority,
            required_capabilities: task.required_capabilities,
            plan: None,
            execution_graph: None,
            assigned_runtime: String::new(),
            checkpoint: None,
            result: String::new(),
            error: None,
            retry_count: 0,
            created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            updated_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        }))
    }

    async fn get_task(&self, request: Request<TaskHandle>) -> Result<Response<AgentTask>, Status> {
        self.get_task_status(request).await
    }
}

#[tonic::async_trait]
impl PlannerService for AgentKernelServer {
    async fn generate_plan(
        &self,
        request: Request<GeneratePlanRequest>,
    ) -> Result<Response<GeneratePlanResponse>, Status> {
        let req = request.into_inner();
        let parsed = match req.intent {
            Some(intent) if !intent.original_prompt.is_empty() => {
                self.intent_engine
                    .parse_prompt(&intent.original_prompt)
                    .await
            }
            _ => {
                return Err(Status::invalid_argument(
                    "GeneratePlanRequest.intent.original_prompt is required",
                ))
            }
        };
        let plan = self
            .planner
            .create_plan(&req.task_id, &parsed)
            .await
            .map_err(Status::invalid_argument)?;
        let validation = plan.validate();

        let steps_proto = plan
            .steps
            .into_iter()
            .map(|s| PlanStep {
                step_id: s.step_id,
                description: s.description,
                target_runtime_kind: s.target_runtime_kind,
                required_capabilities: s.required_capabilities,
                depends_on_step_ids: s.depends_on_step_ids,
                parameters: s.parameters,
            })
            .collect();

        Ok(Response::new(GeneratePlanResponse {
            plan: Some(Plan {
                plan_id: plan.plan_id,
                task_id: plan.task_id,
                steps: steps_proto,
                created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            }),
            validation: Some(PlanValidationResult {
                is_valid: validation.is_valid,
                validation_errors: validation.validation_errors,
                missing_capabilities: validation.missing_capabilities,
            }),
        }))
    }
}

#[tonic::async_trait]
impl CapabilityGatewayService for AgentKernelServer {
    async fn execute_capability(
        &self,
        request: Request<cognyx_proto::cognyx::services::agent::v1::CapabilityRequest>,
    ) -> Result<Response<cognyx_proto::cognyx::services::agent::v1::CapabilityResult>, Status> {
        let req = request.into_inner();
        let cap_req = CapabilityRequest {
            request_id: req.request_id,
            task_id: req.task_id,
            agent_id: req.agent_id,
            capability: req.capability,
            target: req.target,
            arguments: req.arguments,
            constraints: req.constraints,
            permission_context: cognyx_agent_core::PermissionContext {
                user_id: "user-default".to_string(),
                session_id: "sess-default".to_string(),
                granted_capabilities: HashSet::from([
                    "bash".to_string(),
                    "win32.powershell".to_string(),
                    "package.install".to_string(),
                    "application.open".to_string(),
                    "gui".to_string(),
                    "terminal.execute".to_string(),
                ]),
                is_administrator: false,
            },
            timeout_seconds: req.timeout_seconds,
        };

        let res = self.gateway.execute_capability(cap_req).await;

        Ok(Response::new(
            cognyx_proto::cognyx::services::agent::v1::CapabilityResult {
                request_id: res.request_id,
                success: res.success,
                output: res.output,
                error: res.error,
                assigned_runtime_id: res.assigned_runtime_id,
                executed_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            },
        ))
    }
}
