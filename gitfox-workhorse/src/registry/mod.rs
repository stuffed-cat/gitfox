//! Package Registry 模块
//!
//! 实现 Docker Registry V2 API 和 npm Registry API
//! 
//! ## 架构设计
//! 
//! Registry 通过独立域名访问（如 registry.gitfox.studio），Workhorse 根据 Host header
//! 判断请求是否是 Registry 请求。
//! 
//! ### 路由结构
//! 
//! **Docker Registry V2** (registry.gitfox.studio/v2/...):
//! - `GET /v2/` - API 版本检查
//! - `GET /v2/{name}/manifests/{reference}` - 获取 manifest
//! - `PUT /v2/{name}/manifests/{reference}` - 上传 manifest
//! - `DELETE /v2/{name}/manifests/{reference}` - 删除 manifest
//! - `GET /v2/{name}/blobs/{digest}` - 下载 blob
//! - `HEAD /v2/{name}/blobs/{digest}` - 检查 blob 是否存在
//! - `POST /v2/{name}/blobs/uploads/` - 开始上传
//! - `PATCH /v2/{name}/blobs/uploads/{uuid}` - 分块上传
//! - `PUT /v2/{name}/blobs/uploads/{uuid}` - 完成上传
//! - `DELETE /v2/{name}/blobs/uploads/{uuid}` - 取消上传
//! - `GET /v2/{name}/tags/list` - 列出标签
//! 
//! **npm Registry** (registry.gitfox.studio/npm/...):
//! - `GET /npm/{scope}/{name}` - 获取包信息
//! - `GET /npm/{scope}/{name}/-/{tarball}` - 下载 tarball
//! - `PUT /npm/{scope}/{name}` - 发布包
//! - `DELETE /npm/{scope}/{name}/-/{tarball}/-rev/{rev}` - 删除版本
//! - `GET/PUT/DELETE /npm/-/package/{scope}/{name}/dist-tags/{tag}` - dist-tag 操作

pub mod types;
pub mod storage;
pub mod docker;
pub mod npm;
pub mod auth;
pub mod config;

pub use types::*;
// Note: RegistryStorage 已在 storage module 内部使用
// pub use storage::RegistryStorage;
pub use docker::*;
pub use npm::*;
// Auth utilities - 仅在 registry 模块内部使用
// pub use auth::*;
pub use config::RegistryConfig;
