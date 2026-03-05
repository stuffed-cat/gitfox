//! GitLayer gRPC client
//! 用于主应用调用 GitLayer 服务

// 导出 GitLayer proto 生成的客户端代码
tonic::include_proto!("gitlayer");

pub use repository_service_client::RepositoryServiceClient;
pub use ref_service_client::RefServiceClient;
pub use commit_service_client::CommitServiceClient;
pub use tree_service_client::TreeServiceClient;
pub use blob_service_client::BlobServiceClient;
pub use diff_service_client::DiffServiceClient;
pub use gpg_service_client::GpgServiceClient;
pub use operation_service_client::OperationServiceClient;
