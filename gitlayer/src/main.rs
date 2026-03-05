//! GitLayer server entry point

use std::net::SocketAddr;
use std::sync::Arc;

use tonic::transport::Server;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use gitlayer::config::Config;
use gitlayer::services::{
    repository::RepositoryServiceImpl,
    refs::RefServiceImpl,
    commit::CommitServiceImpl,
    blob::BlobServiceImpl,
    tree::TreeServiceImpl,
    diff::DiffServiceImpl,
    smarthttp::{SmartHttpServiceImpl, SshServiceImpl},
    operations::OperationServiceImpl,
    health::HealthServiceImpl,
    gpg::GpgServiceImpl,
};
use gitlayer::proto::{
    repository_service_server::RepositoryServiceServer,
    ref_service_server::RefServiceServer,
    commit_service_server::CommitServiceServer,
    blob_service_server::BlobServiceServer,
    tree_service_server::TreeServiceServer,
    diff_service_server::DiffServiceServer,
    smart_http_service_server::SmartHttpServiceServer,
    ssh_service_server::SshServiceServer,
    operation_service_server::OperationServiceServer,
    health_service_server::HealthServiceServer,
    gpg_service_server::GpgServiceServer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gitlayer=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("GitLayer starting...");

    // Load configuration
    let config = Config::load();
    let config = Arc::new(config);
    
    info!("Storage path: {}", config.storage_path);
    info!("Listen address: {}", config.listen_addr);

    // Create services
    let repository_service = RepositoryServiceImpl::new(config.clone());
    let ref_service = RefServiceImpl::new(config.clone());
    let commit_service = CommitServiceImpl::new(config.clone());
    let blob_service = BlobServiceImpl::new(config.clone());
    let tree_service = TreeServiceImpl::new(config.clone());
    let diff_service = DiffServiceImpl::new(config.clone());
    let smarthttp_service = SmartHttpServiceImpl::new(config.clone());
    let ssh_service = SshServiceImpl::new(config.clone());
    let operation_service = OperationServiceImpl::new(config.clone());
    let health_service = HealthServiceImpl::new();
    let gpg_service = GpgServiceImpl::new(config.clone());

    let addr: SocketAddr = config.listen_addr.parse()?;
    
    info!("GitLayer listening on {}", addr);

    Server::builder()
        .add_service(RepositoryServiceServer::new(repository_service))
        .add_service(RefServiceServer::new(ref_service))
        .add_service(CommitServiceServer::new(commit_service))
        .add_service(BlobServiceServer::new(blob_service))
        .add_service(TreeServiceServer::new(tree_service))
        .add_service(DiffServiceServer::new(diff_service))
        .add_service(SmartHttpServiceServer::new(smarthttp_service))
        .add_service(SshServiceServer::new(ssh_service))
        .add_service(OperationServiceServer::new(operation_service))
        .add_service(HealthServiceServer::new(health_service))
        .add_service(GpgServiceServer::new(gpg_service))
        .serve(addr)
        .await?;

    Ok(())
}
