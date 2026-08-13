use crate::events::RuntimeEventPublisher;
use cognyx_execution::RuntimeRegistry;
use cognyx_proto::cognyx::services::runtime::v1::runtime_manager_service_server::RuntimeManagerService;
use cognyx_proto::cognyx::services::runtime::v1::*;
use cognyx_resources::ResourceManager;
use cognyx_runtime_network::VirtualNetworkManager;
use cognyx_runtime_storage::VMStorageManager;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct RuntimeManagerServer {
    pub registry: Arc<RuntimeRegistry>,
    pub resources: Arc<ResourceManager>,
    pub storage: Arc<VMStorageManager>,
    pub network: Arc<VirtualNetworkManager>,
}

impl Default for RuntimeManagerServer {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeManagerServer {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RuntimeRegistry::new()),
            resources: Arc::new(ResourceManager::default()),
            storage: Arc::new(VMStorageManager::default()),
            network: Arc::new(VirtualNetworkManager::default()),
        }
    }
}

#[tonic::async_trait]
impl RuntimeManagerService for RuntimeManagerServer {
    async fn create_runtime(
        &self,
        request: Request<CreateRuntimeRequest>,
    ) -> Result<Response<CreateRuntimeResponse>, Status> {
        let req = request.into_inner();
        let runtime_id = format!("rt-{}", uuid::Uuid::now_v7());

        RuntimeEventPublisher::publish_event(
            &runtime_id,
            RuntimeType::try_from(req.r#type).unwrap_or(RuntimeType::NativeLinux),
            "runtime.created",
            &format!("Created runtime '{}'", req.name),
        );

        let spec = ExecutionRuntimeSpec {
            runtime_id: runtime_id.clone(),
            name: req.name,
            r#type: req.r#type,
            state: RuntimeState::Created as i32,
            resources: req.resources,
            capabilities: vec!["execution.basic".to_string()],
            location: "local".to_string(),
            created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            updated_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        };

        Ok(Response::new(CreateRuntimeResponse {
            runtime: Some(spec),
        }))
    }

    async fn start_runtime(
        &self,
        request: Request<StartRuntimeRequest>,
    ) -> Result<Response<ExecutionRuntimeSpec>, Status> {
        let req = request.into_inner();
        RuntimeEventPublisher::publish_event(
            &req.runtime_id,
            RuntimeType::NativeLinux,
            "runtime.started",
            "Started execution runtime",
        );

        let spec = ExecutionRuntimeSpec {
            runtime_id: req.runtime_id,
            name: "Runtime".to_string(),
            r#type: RuntimeType::NativeLinux as i32,
            state: RuntimeState::Running as i32,
            resources: None,
            capabilities: vec!["execution.basic".to_string()],
            location: "local".to_string(),
            created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            updated_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        };

        Ok(Response::new(spec))
    }

    async fn stop_runtime(
        &self,
        request: Request<StopRuntimeRequest>,
    ) -> Result<Response<ExecutionRuntimeSpec>, Status> {
        let req = request.into_inner();
        RuntimeEventPublisher::publish_event(
            &req.runtime_id,
            RuntimeType::NativeLinux,
            "runtime.stopped",
            "Stopped execution runtime",
        );

        let spec = ExecutionRuntimeSpec {
            runtime_id: req.runtime_id,
            name: "Runtime".to_string(),
            r#type: RuntimeType::NativeLinux as i32,
            state: RuntimeState::Stopped as i32,
            resources: None,
            capabilities: vec![],
            location: "local".to_string(),
            created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            updated_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        };

        Ok(Response::new(spec))
    }

    async fn pause_runtime(
        &self,
        request: Request<PauseRuntimeRequest>,
    ) -> Result<Response<ExecutionRuntimeSpec>, Status> {
        let req = request.into_inner();
        RuntimeEventPublisher::publish_event(
            &req.runtime_id,
            RuntimeType::NativeLinux,
            "runtime.paused",
            "Paused runtime",
        );
        Err(Status::unimplemented("Not implemented"))
    }

    async fn resume_runtime(
        &self,
        request: Request<ResumeRuntimeRequest>,
    ) -> Result<Response<ExecutionRuntimeSpec>, Status> {
        let req = request.into_inner();
        RuntimeEventPublisher::publish_event(
            &req.runtime_id,
            RuntimeType::NativeLinux,
            "runtime.resumed",
            "Resumed runtime",
        );
        Err(Status::unimplemented("Not implemented"))
    }

    async fn delete_runtime(
        &self,
        _request: Request<DeleteRuntimeRequest>,
    ) -> Result<Response<()>, Status> {
        Ok(Response::new(()))
    }

    async fn get_runtime(
        &self,
        request: Request<GetRuntimeRequest>,
    ) -> Result<Response<ExecutionRuntimeSpec>, Status> {
        let req = request.into_inner();
        let spec = ExecutionRuntimeSpec {
            runtime_id: req.runtime_id,
            name: "Default Runtime".to_string(),
            r#type: RuntimeType::NativeLinux as i32,
            state: RuntimeState::Running as i32,
            resources: None,
            capabilities: vec!["bash".to_string()],
            location: "local".to_string(),
            created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            updated_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        };
        Ok(Response::new(spec))
    }

    async fn list_runtimes(
        &self,
        _request: Request<ListRuntimesRequest>,
    ) -> Result<Response<ListRuntimesResponse>, Status> {
        let runtimes = vec![ExecutionRuntimeSpec {
            runtime_id: "host-native".to_string(),
            name: "Native Linux Host".to_string(),
            r#type: RuntimeType::NativeLinux as i32,
            state: RuntimeState::Running as i32,
            resources: Some(ResourceAllocation {
                cpus: 8,
                memory_bytes: 16 * 1024 * 1024 * 1024,
                disk_bytes: 200 * 1024 * 1024 * 1024,
                vram_bytes: 2048 * 1024 * 1024,

                gpu_passthrough: false,
            }),
            capabilities: vec![
                "bash".to_string(),
                "process.spawn".to_string(),
                "filesystem.read".to_string(),
            ],
            location: "local".to_string(),
            created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            updated_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        }];

        Ok(Response::new(ListRuntimesResponse { runtimes }))
    }

    async fn create_snapshot(
        &self,
        request: Request<CreateSnapshotRequest>,
    ) -> Result<Response<VmSnapshot>, Status> {
        let req = request.into_inner();
        let snap_id = format!("snap-{}", uuid::Uuid::now_v7());
        RuntimeEventPublisher::publish_event(
            &req.runtime_id,
            RuntimeType::WindowsVm,
            "runtime.snapshot_created",
            &format!("Created snapshot '{}'", req.snapshot_name),
        );

        Ok(Response::new(VmSnapshot {
            snapshot_id: snap_id,
            vm_id: req.runtime_id,
            name: req.snapshot_name,
            description: req.description,
            created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            size_bytes: 1024 * 1024,
        }))
    }

    async fn restore_snapshot(
        &self,
        request: Request<RestoreSnapshotRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        RuntimeEventPublisher::publish_event(
            &req.runtime_id,
            RuntimeType::WindowsVm,
            "runtime.snapshot_restored",
            &format!("Restored snapshot '{}'", req.snapshot_id),
        );
        Ok(Response::new(()))
    }

    async fn get_metrics(
        &self,
        _request: Request<GetMetricsRequest>,
    ) -> Result<Response<ResourceUsageMetrics>, Status> {
        Ok(Response::new(ResourceUsageMetrics {
            cpu_usage_percent: 15.0,
            memory_used_bytes: 4 * 1024 * 1024 * 1024,
            memory_total_bytes: 16 * 1024 * 1024 * 1024,
            disk_read_bytes: 10240,
            disk_write_bytes: 20480,
            net_rx_bytes: 5000,
            net_tx_bytes: 2000,
            gpu_usage_percent: 0.0,
        }))
    }

    async fn check_network_policy(
        &self,
        request: Request<NetworkPolicyCheckRequest>,
    ) -> Result<Response<NetworkPolicyCheckResponse>, Status> {
        let req = request.into_inner();
        let (allowed, reason) = self
            .network
            .can_communicate(
                &req.source_runtime_id,
                &req.target_runtime_id,
                req.port,
                &req.protocol,
            )
            .await;

        Ok(Response::new(NetworkPolicyCheckResponse {
            allowed,
            reason,
        }))
    }
}
