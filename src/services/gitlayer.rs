//! GitLayer gRPC client
//! 用于主应用调用 GitLayer 服务

// 导出 repository.proto 生成的客户端代码
tonic::include_proto!("gitlayer");

pub use repository_service_client::RepositoryServiceClient;
