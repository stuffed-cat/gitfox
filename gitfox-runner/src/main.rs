mod config;
mod error;
mod executor;
mod messages;
mod runner;
mod security;

use clap::{Parser, Subcommand};
use config::RunnerConfig;
use error::Result;
use log::{error, info};
use runner::Runner;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "gitfox-runner")]
#[command(about = "GitFox CI/CD Runner", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Register a new runner with the GitFox server
    Register {
        /// GitFox server URL (e.g., http://localhost:8081)
        #[arg(short, long)]
        url: String,

        /// Runner registration token (e.g., glrt-xxx)
        #[arg(short, long)]
        token: String,

        /// Runner name (defaults to hostname)
        #[arg(short, long)]
        name: Option<String>,

        /// Runner description
        #[arg(short, long)]
        description: Option<String>,

        /// Runner tags (comma-separated)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,

        /// Executor type (shell, docker)
        #[arg(short, long, default_value = "shell")]
        executor: String,

        /// Config file path (default: ~/.gitfox-runner/config.toml)
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Run the registered runner
    Run {
        /// Config file path (default: ~/.gitfox-runner/config.toml)
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterRequest {
    token: String,
    name: String,
    description: Option<String>,
    tags: Option<Vec<String>>,
    executor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterResponse {
    runner_id: i64,
    auth_token: String,
    websocket_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    match args.command {
        Commands::Register {
            url,
            token,
            name,
            description,
            tags,
            executor,
            config,
        } => {
            register_runner(url, token, name, description, tags, executor, config).await?;
        }
        Commands::Run { config } => {
            run_runner(config).await?;
        }
    }

    Ok(())
}

async fn register_runner(
    server_url: String,
    registration_token: String,
    name: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
    executor: String,
    config_path: Option<PathBuf>,
) -> Result<()> {
    let runner_name = name.unwrap_or_else(|| {
        hostname::get()
            .unwrap()
            .to_string_lossy()
            .to_string()
    });

    info!("Registering runner '{}' with server {}", runner_name, server_url);

    // 调用注册 API
    let client = reqwest::Client::new();
    let register_url = format!("{}/api/v1/runner/register", server_url.trim_end_matches('/'));
    
    let request = RegisterRequest {
        token: registration_token,
        name: runner_name.clone(),
        description,
        tags: if tags.is_empty() { None } else { Some(tags.clone()) },
        executor: Some(executor.clone()),
    };

    info!("Sending registration request to {}", register_url);
    
    let response = client
        .post(&register_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| error::RunnerError::Connection(format!("Failed to connect to server: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(error::RunnerError::Auth(format!(
            "Registration failed with status {}: {}",
            status, error_text
        )));
    }

    let register_response: RegisterResponse = response
        .json()
        .await
        .map_err(|e| error::RunnerError::Connection(format!("Failed to parse response: {}", e)))?;

    info!("✓ Runner registered successfully!");
    info!("  Runner ID: {}", register_response.runner_id);
    info!("  WebSocket URL: {}", register_response.websocket_url);

    // 保存配置到文件
    let config_file = get_config_path(config_path)?;
    
    let config = RunnerConfig {
        server_url: register_response.websocket_url,
        token: register_response.auth_token,
        name: runner_name,
        tags,
        executor,
        builds_dir: std::path::PathBuf::from("./builds"),
        max_work_dir_size_mb: 10 * 1024, // 10 GB
        concurrent_jobs: 1,
        default_docker_image: String::from("alpine:latest"),
        clean_builds: true,
        security_enabled: true,  // MANDATORY: Always enable security
        script_timeout_seconds: 3600,  // 1 hour default
        network_mode: String::from("full"),  // Allow network by default
    };

    config.save(&config_file)?;
    
    info!("✓ Configuration saved to {}", config_file.display());
    info!("\nYou can now start the runner with:");
    info!("  gitfox-runner run");

    Ok(())
}

async fn run_runner(config_path: Option<PathBuf>) -> Result<()> {
    let config_file = get_config_path(config_path)?;
    
    if !config_file.exists() {
        error!("Config file not found: {}", config_file.display());
        error!("Please register the runner first with:");
        error!("  gitfox-runner register --url <SERVER_URL> --token <TOKEN>");
        return Err(error::RunnerError::Config(format!(
            "Config file not found: {}",
            config_file.display()
        )));
    }

    let config = RunnerConfig::load(&config_file)?;

    info!("Starting GitFox Runner: {}", config.name);
    info!("Connecting to: {}", config.server_url);
    info!("Tags: {:?}", config.tags);

    let mut runner = Runner::new(config);

    if let Err(e) = runner.run().await {
        error!("Runner error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

fn get_config_path(custom_path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = custom_path {
        return Ok(path);
    }

    // 默认路径: ~/.gitfox-runner/config.toml
    let home = dirs::home_dir().ok_or_else(|| {
        error::RunnerError::Config("Cannot determine home directory".to_string())
    })?;

    let config_dir = home.join(".gitfox-runner");
    std::fs::create_dir_all(&config_dir).map_err(|e| {
        error::RunnerError::Config(format!("Failed to create config directory: {}", e))
    })?;

    Ok(config_dir.join("config.toml"))
}
