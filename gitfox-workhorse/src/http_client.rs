use hyper::body::Incoming;
use hyper::Request;
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::rt::TokioExecutor;
use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use hyperlocal::{UnixConnector, Uri};
use std::path::Path;

/// HTTP 客户端包装器，支持 TCP 和 Unix Socket 两种连接方式
#[derive(Clone)]
pub enum BackendClient {
    /// TCP 连接（使用 reqwest）
    Tcp(reqwest::Client),
    /// Unix Socket 连接（使用 hyper + hyperlocal）
    Unix {
        client: HyperClient<UnixConnector, BoxBody<Bytes, std::convert::Infallible>>,
        socket_path: String,
    },
}

impl BackendClient {
    /// 创建 TCP 客户端
    pub fn tcp(timeout: std::time::Duration) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()?;
        Ok(BackendClient::Tcp(client))
    }

    /// 创建 Unix Socket 客户端
    pub fn unix<P: AsRef<Path>>(socket_path: P) -> Self {
        let connector = UnixConnector;
        let client = HyperClient::builder(TokioExecutor::new()).build(connector);
        
        BackendClient::Unix {
            client,
            socket_path: socket_path.as_ref().to_string_lossy().to_string(),
        }
    }

    /// 判断是否使用 Unix Socket
    pub fn is_unix(&self) -> bool {
        matches!(self, BackendClient::Unix { .. })
    }

    /// 获取 reqwest 客户端（仅用于 TCP）
    pub fn as_reqwest(&self) -> Option<&reqwest::Client> {
        match self {
            BackendClient::Tcp(client) => Some(client),
            _ => None,
        }
    }

    /// 获取 Unix socket 路径
    pub fn socket_path(&self) -> Option<&str> {
        match self {
            BackendClient::Unix { socket_path, .. } => Some(socket_path),
            _ => None,
        }
    }

    /// 构建 Unix Socket URI
    pub fn build_unix_uri(&self, path: &str) -> Option<hyper::Uri> {
        match self {
            BackendClient::Unix { socket_path, .. } => {
                Some(Uri::new(socket_path, path).into())
            }
            _ => None,
        }
    }

    /// 发送 Unix Socket 请求
    pub async fn send_unix_request(
        &self,
        req: Request<BoxBody<Bytes, std::convert::Infallible>>,
    ) -> Result<hyper::Response<Incoming>, hyper_util::client::legacy::Error> {
        match self {
            BackendClient::Unix { client, .. } => {
                client.request(req).await
            }
            _ => panic!("send_unix_request called on non-Unix client"),
        }
    }
}
