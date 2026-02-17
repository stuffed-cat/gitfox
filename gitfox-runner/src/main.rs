mod config;
mod error;
mod executor;
mod messages;
mod runner;

use clap::Parser;
use config::RunnerConfig;
use error::Result;
use log::{error, info};
use runner::Runner;

#[derive(Parser, Debug)]
#[command(name = "gitfox-runner")]
#[command(about = "GitFox CI/CD Runner", long_about = None)]
struct Args {
    /// Runner registration token
    #[arg(short, long, env("GITFOX_RUNNER_TOKEN"))]
    token: String,

    /// GitFox server URL (e.g., ws://localhost:8081)
    #[arg(short, long, env("GITFOX_SERVER_URL"))]
    server_url: String,

    /// Runner name (defaults to hostname)
    #[arg(short, long, env("GITFOX_RUNNER_NAME"))]
    name: Option<String>,

    /// Runner tags (comma-separated)
    #[arg(long, env("GITFOX_RUNNER_TAGS"), value_delimiter = ',')]
    tags: Vec<String>,

    /// Executor type (shell, docker)
    #[arg(short, long, default_value = "shell", env("GITFOX_EXECUTOR"))]
    executor: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    let runner_name = args
        .name
        .unwrap_or_else(|| hostname::get().unwrap().to_string_lossy().to_string());

    let config = RunnerConfig {
        server_url: args.server_url,
        token: args.token,
        name: runner_name,
        tags: args.tags,
        executor: args.executor,
    };

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
