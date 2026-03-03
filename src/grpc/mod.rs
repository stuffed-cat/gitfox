//! gRPC 服务模块
//!
//! 主应用作为 gRPC 服务端，提供内部服务间通信接口。
//! 符合 GitLab 架构：主应用只负责权限管理，Git 操作通过 GitLayer 完成。

pub mod auth_service;
pub mod lfs_service;

pub use auth_service::AuthServiceImpl;
pub use auth_service::auth_proto;
pub use lfs_service::LfsServiceImpl;
pub use lfs_service::lfs_proto;
