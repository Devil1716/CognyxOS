use cognyx_agent_kernel::AgentKernelServer;
use cognyx_proto::cognyx::services::agent::v1::agent_kernel_service_server::AgentKernelServiceServer;
use cognyx_proto::cognyx::services::agent::v1::capability_gateway_service_server::CapabilityGatewayServiceServer;
use cognyx_proto::cognyx::services::agent::v1::intent_engine_service_server::IntentEngineServiceServer;
use cognyx_proto::cognyx::services::agent::v1::planner_service_server::PlannerServiceServer;
use cognyx_proto::cognyx::services::agent::v1::task_manager_service_server::TaskManagerServiceServer;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let addr = "127.0.0.1:50053".parse()?;
    let server = Arc::new(AgentKernelServer::new());

    info!(
        "Starting CognyxOS Agent Kernel Services listening on http://{}",
        addr
    );

    Server::builder()
        .add_service(AgentKernelServiceServer::from_arc(server.clone()))
        .add_service(IntentEngineServiceServer::from_arc(server.clone()))
        .add_service(TaskManagerServiceServer::from_arc(server.clone()))
        .add_service(PlannerServiceServer::from_arc(server.clone()))
        .add_service(CapabilityGatewayServiceServer::from_arc(server))
        .serve(addr)
        .await?;

    Ok(())
}
