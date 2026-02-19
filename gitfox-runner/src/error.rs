use thiserror::Error;

pub type Result<T> = std::result::Result<T, RunnerError>;

#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Other error: {0}")]
    Other(String),
}
