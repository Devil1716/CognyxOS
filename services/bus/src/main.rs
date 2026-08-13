use cognyx_bus::MessageBusDaemon;
use cognyx_proto::cognyx::bus::v1::message_bus_service_server::MessageBusServiceServer;
use tonic::transport::Server;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let addr = "127.0.0.1:50051".parse()?;
    let bus_daemon = MessageBusDaemon::new();

    info!(
        "Starting CognyxOS Message Bus Daemon listening on http://{}",
        addr
    );

    Server::builder()
        .add_service(MessageBusServiceServer::new(bus_daemon))
        .serve(addr)
        .await?;

    Ok(())
}
