mod config;
mod error;
mod executor;
mod messages;
mod runner;
mod security;

use clap::{Parser, Subcommand};
use config::RunnerConfig;
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
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
    /// Register a new runner with the GitFox server (missing params are prompted interactively)
    Register {
        /// GitFox server URL (e.g., http://localhost:8080)
        #[arg(short, long)]
        url: Option<String>,

        /// Runner registration token
        #[arg(short, long)]
        token: Option<String>,

        /// Runner name (defaults to hostname if omitted)
        #[arg(short, long)]
        name: Option<String>,

        /// Runner description
        #[arg(short, long)]
        description: Option<String>,

        /// Runner tags (comma-separated)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,

        /// Executor type: shell, docker, kubernetes  (prompted if omitted)
        #[arg(short, long)]
        executor: Option<String>,

        /// Default container image for docker/kubernetes executor  (prompted if executor needs one)
        #[arg(long)]
        image: Option<String>,

        /// Kubernetes namespace  (prompted if executor=kubernetes and omitted)
        #[arg(long)]
        kubernetes_namespace: Option<String>,

        /// Path to kubeconfig file  (prompted if executor=kubernetes and omitted)
        #[arg(long)]
        kubernetes_config: Option<String>,

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
            image,
            kubernetes_namespace,
            kubernetes_config,
            config,
        } => {
            register_runner(
                url,
                token,
                name,
                description,
                tags,
                executor,
                image,
                kubernetes_namespace,
                kubernetes_config,
                config,
            )
            .await?;
        }
        Commands::Run { config } => {
            run_runner(config).await?;
        }
    }

    Ok(())
}

// ─── 交互式注册向导 ──────────────────────────────────────────────────────────

struct RegisterParams {
    url: String,
    token: String,
    name: String,
    description: Option<String>,
    tags: Vec<String>,
    executor: String,
    image: String,
    kubernetes_namespace: String,
    kubernetes_config: Option<String>,
}

/// 对于每个缺失的字段弹出交互式提示，已通过 CLI 提供的字段直接使用
fn prompt_register(
    url: Option<String>,
    token: Option<String>,
    name: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
    executor: Option<String>,
    image: Option<String>,
    kubernetes_namespace: Option<String>,
    kubernetes_config: Option<String>,
) -> Result<RegisterParams> {
    let theme = ColorfulTheme::default();

    println!();
    println!("{}", style("GitFox Runner Registration").bold().cyan());
    println!("{}", style("─".repeat(40)).dim());
    println!();

    // ── Server URL ──────────────────────────────
    let url = match url {
        Some(u) => u,
        None => Input::with_theme(&theme)
            .with_prompt("GitFox server URL")
            .with_initial_text("http://localhost:8080")
            .interact_text()
            .map_err(|e| error::RunnerError::Config(e.to_string()))?,
    };

    // ── Registration token ───────────────────────
    let token = match token {
        Some(t) => t,
        None => Password::with_theme(&theme)
            .with_prompt("Registration token")
            .interact()
            .map_err(|e| error::RunnerError::Config(e.to_string()))?,
    };

    // ── Runner name ──────────────────────────────
    let default_name = hostname::get()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let runner_name = match name {
        Some(n) => n,
        None => Input::with_theme(&theme)
            .with_prompt("Runner name")
            .default(default_name)
            .interact_text()
            .map_err(|e| error::RunnerError::Config(e.to_string()))?,
    };

    // ── Description (optional, skip if not provided) ──
    let description = match description {
        Some(d) => Some(d),
        None => {
            let d: String = Input::with_theme(&theme)
                .with_prompt("Runner description (optional, press Enter to skip)")
                .allow_empty(true)
                .interact_text()
                .map_err(|e| error::RunnerError::Config(e.to_string()))?;
            if d.is_empty() { None } else { Some(d) }
        }
    };

    // ── Tags ─────────────────────────────────────
    let tags = if !tags.is_empty() {
        tags
    } else {
        let raw: String = Input::with_theme(&theme)
            .with_prompt("Tags (comma-separated, press Enter to skip)")
            .allow_empty(true)
            .interact_text()
            .map_err(|e| error::RunnerError::Config(e.to_string()))?;
        raw.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    // ── Executor ─────────────────────────────────
    let executor_choices = &["shell", "docker", "kubernetes"];
    let executor = match executor {
        Some(e) => {
            // 规范化别名
            match e.as_str() {
                "k8s" => "kubernetes".to_string(),
                other => other.to_string(),
            }
        }
        None => {
            println!();
            println!("{}", style("Select executor type:").bold());
            let idx = Select::with_theme(&theme)
                .with_prompt("Executor")
                .items(executor_choices)
                .default(0)
                .interact()
                .map_err(|e| error::RunnerError::Config(e.to_string()))?;
            executor_choices[idx].to_string()
        }
    };

    // ── Container image (docker / kubernetes) ────
    let needs_image = matches!(executor.as_str(), "docker" | "kubernetes");
    let image = if needs_image {
        match image {
            Some(img) => img,
            None => {
                println!();
                let default_img = match executor.as_str() {
                    "docker" => "alpine:latest",
                    _ => "alpine:latest",
                };
                Input::with_theme(&theme)
                    .with_prompt(format!("Container image for {} executor", executor))
                    .default(default_img.to_string())
                    .interact_text()
                    .map_err(|e| error::RunnerError::Config(e.to_string()))?
            }
        }
    } else {
        image.unwrap_or_else(|| "alpine:latest".to_string())
    };

    // ── Kubernetes-specific ───────────────────────
    let (kubernetes_namespace, kubernetes_config) = if executor == "kubernetes" {
        let ns = match kubernetes_namespace {
            Some(n) => n,
            None => Input::with_theme(&theme)
                .with_prompt("Kubernetes namespace")
                .default("default".to_string())
                .interact_text()
                .map_err(|e| error::RunnerError::Config(e.to_string()))?,
        };

        let kc = match kubernetes_config {
            Some(k) => Some(k),
            None => {
                let use_custom = Confirm::with_theme(&theme)
                    .with_prompt("Specify a custom kubeconfig file?")
                    .default(false)
                    .interact()
                    .map_err(|e| error::RunnerError::Config(e.to_string()))?;
                if use_custom {
                    let path: String = Input::with_theme(&theme)
                        .with_prompt("Path to kubeconfig")
                        .with_initial_text(
                            dirs::home_dir()
                                .map(|h| h.join(".kube/config").to_string_lossy().to_string())
                                .unwrap_or_else(|| "~/.kube/config".to_string()),
                        )
                        .interact_text()
                        .map_err(|e| error::RunnerError::Config(e.to_string()))?;
                    Some(path)
                } else {
                    None
                }
            }
        };

        (ns, kc)
    } else {
        (
            kubernetes_namespace.unwrap_or_else(|| "default".to_string()),
            kubernetes_config,
        )
    };

    println!();
    println!("{}", style("─".repeat(40)).dim());
    println!("  {:<18} {}", style("Server:").dim(), style(&url).green());
    println!("  {:<18} {}", style("Runner name:").dim(), style(&runner_name).green());
    println!("  {:<18} {}", style("Executor:").dim(), style(&executor).green());
    if needs_image {
        println!("  {:<18} {}", style("Image:").dim(), style(&image).green());
    }
    println!("{}", style("─".repeat(40)).dim());
    println!();

    Ok(RegisterParams {
        url,
        token,
        name: runner_name,
        description,
        tags,
        executor,
        image,
        kubernetes_namespace,
        kubernetes_config,
    })
}

async fn register_runner(
    url: Option<String>,
    token: Option<String>,
    name: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
    executor: Option<String>,
    image: Option<String>,
    kubernetes_namespace: Option<String>,
    kubernetes_config: Option<String>,
    config_path: Option<PathBuf>,
) -> Result<()> {
    let params = prompt_register(
        url, token, name, description, tags,
        executor, image, kubernetes_namespace, kubernetes_config,
    )?;

    info!(
        "Registering runner '{}' (executor: {}) with {}",
        params.name, params.executor, params.url
    );

    let client = reqwest::Client::new();
    let register_url = format!("{}/api/v1/runner/register", params.url.trim_end_matches('/'));

    let request = RegisterRequest {
        token: params.token,
        name: params.name.clone(),
        description: params.description,
        tags: if params.tags.is_empty() { None } else { Some(params.tags.clone()) },
        executor: Some(params.executor.clone()),
    };

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

    println!("{} Runner registered!", style("✓").green().bold());
    println!("  Runner ID: {}", register_response.runner_id);

    let config_file = get_config_path(config_path)?;

    let config = RunnerConfig {
        server_url: register_response.websocket_url,
        token: register_response.auth_token,
        name: params.name,
        tags: params.tags,
        executor: params.executor,
        builds_dir: std::path::PathBuf::from("./builds"),
        max_work_dir_size_mb: 10 * 1024,
        concurrent_jobs: 1,
        default_image: params.image,
        kubernetes_namespace: params.kubernetes_namespace,
        kubernetes_config: params.kubernetes_config,
        clean_builds: true,
        security_enabled: true,
        script_timeout_seconds: 3600,
        network_mode: String::from("full"),
    };

    config.save(&config_file)?;

    println!("{} Config saved to {}", style("✓").green().bold(), config_file.display());
    println!();
    println!("Start the runner with:  {}", style("gitfox-runner run").bold());

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

    info!("============================================");
    info!("          GitFox Runner v{}", env!("CARGO_PKG_VERSION"));
    info!("============================================");
    info!("Runner name:     {}", config.name);
    info!("Server URL:      {}", config.server_url);
    info!("Executor:        {}", config.executor);
    info!("Tags:            {}", if config.tags.is_empty() { "(none)".to_string() } else { config.tags.join(", ") });
    info!("Builds dir:      {}", config.builds_dir.display());
    info!("Concurrent jobs: {}", config.concurrent_jobs);
    info!("Security:        {}", if config.security_enabled { "enabled" } else { "DISABLED" });
    info!("Network mode:    {}", config.network_mode);
    if config.executor == "docker" || config.executor == "kubernetes" {
        info!("Default image:   {}", config.default_image);
    }
    if config.executor == "kubernetes" {
        info!("K8s namespace:   {}", config.kubernetes_namespace);
    }
    info!("============================================");

    let mut runner = Runner::new(config, config_file);

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
