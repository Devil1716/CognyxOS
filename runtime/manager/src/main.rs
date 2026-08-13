use cognyx_proto::cognyx::services::runtime::v1::runtime_manager_service_server::RuntimeManagerServiceServer;
use cognyx_runtime_manager::RuntimeManagerServer;
use tonic::transport::Server;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let addr = "127.0.0.1:50052".parse()?;
    let server = RuntimeManagerServer::new();

    info!(
        "Starting CognyxOS Runtime Manager Daemon listening on http://{}",
        addr
    );

    Server::builder()
        .add_service(RuntimeManagerServiceServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}
