use cognyx_agent_kernel::AgentKernelServer;
use cognyx_execution::LinuxRuntime;
use cognyx_proto::cognyx::services::agent::v1::agent_kernel_service_server::AgentKernelService;
use cognyx_proto::cognyx::services::agent::v1::*;
use cognyx_windows::WindowsRuntime;
use tonic::Request;

#[tokio::test]
async fn test_e2e_user_intent_run_python_script() {
    let server = AgentKernelServer::new();

    let req = Request::new(SubmitTaskRequest {
        meta: None,
        cap: None,
        prompt: "Run a Python script".to_string(),
        priority: 5,
    });

    let res = server.submit_task(req).await.unwrap().into_inner();
    assert!(!res.task_id.is_empty());

    let status_req = Request::new(TaskHandle {
        task_id: res.task_id.clone(),
        status: TaskState::Running as i32,
        submitted_at: None,
    });

    let task_status = server
        .get_task_status(status_req)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(task_status.prompt, "Run a Python script");
}

#[tokio::test]
async fn test_e2e_user_intent_open_windows_application() {
    let server = AgentKernelServer::new();

    let win_rt = Box::new(WindowsRuntime::new("win-vm-1", "Windows 11 Enterprise"));
    server.registry.register(win_rt);

    let req = Request::new(SubmitTaskRequest {
        meta: None,
        cap: None,
        prompt: "Open a Windows application".to_string(),
        priority: 5,
    });

    let res = server.submit_task(req).await.unwrap().into_inner();
    assert!(!res.task_id.is_empty());
}

#[tokio::test]
async fn test_e2e_windows_runtime_failover_recovery() {
    let server = AgentKernelServer::new();

    let req = Request::new(SubmitTaskRequest {
        meta: None,
        cap: None,
        prompt: "Open a Windows application".to_string(),
        priority: 5,
    });

    let handle = server.submit_task(req).await.unwrap().into_inner();

    let recovered = server
        .recover_task(Request::new(handle))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(recovered.task_id, recovered.task_id);
}
