pub mod user;
pub mod project;
pub mod repository;
pub mod branch;
pub mod commit;
pub mod tag;
pub mod merge_request;
pub mod pipeline;
pub mod webhook;
pub mod namespace;
pub mod ssh_key;
pub mod gpg_key;  // GPG 签名支持
pub mod issue;
pub mod personal_access_token;
pub mod oauth;
pub mod two_factor;
pub mod runner_usage;
pub mod scope;  // 新增：类型安全的 scope 模块
pub mod lfs;    // Git LFS 支持
pub mod package; // Package Registry 支持

pub use user::*;
pub use project::*;
pub use repository::*;
pub use branch::*;
pub use commit::*;
pub use tag::*;
pub use merge_request::*;
pub use pipeline::*;
pub use webhook::*;
pub use namespace::*;
pub use ssh_key::*;
pub use gpg_key::*;  // 导出 GPG 类型
pub use issue::*;
pub use personal_access_token::*;
pub use oauth::*;
pub use two_factor::*;
pub use runner_usage::*;
pub use scope::*;  // 导出 scope 类型
pub use package::*; // 导出 package 类型