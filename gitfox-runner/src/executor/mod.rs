mod docker;
mod kubernetes;
mod shell;

pub use docker::DockerExecutor;
pub use kubernetes::KubernetesExecutor;
pub use shell::ShellExecutor;

use crate::config::RunnerConfig;
use crate::error::{Result, RunnerError};
use crate::messages::Job;
use log::warn;

/// 所有执行器必须实现的 trait
pub trait ExecutorTrait {
    fn execute<F>(&self, job: &Job, log_callback: F) -> Result<i32>
    where
        F: FnMut(&str);
}

/// 执行器入口，根据配置的 executor 类型分发到具体实现
pub struct Executor {
    config: RunnerConfig,
}

impl Executor {
    pub fn new(config: RunnerConfig) -> Self {
        if let Err(e) = std::fs::create_dir_all(&config.builds_dir) {
            warn!("Failed to create builds directory: {}", e);
        }
        Self { config }
    }

    pub fn execute<F>(&self, job: &Job, log_callback: F) -> Result<i32>
    where
        F: FnMut(&str),
    {
        match self.config.executor.as_str() {
            "shell" => ShellExecutor::new(&self.config).execute(job, log_callback),
            "docker" => DockerExecutor::new(&self.config).execute(job, log_callback),
            "kubernetes" | "k8s" => KubernetesExecutor::new(&self.config).execute(job, log_callback),
            other => Err(RunnerError::Execution(format!(
                "Unknown executor type: {}. Supported: shell, docker, kubernetes",
                other
            ))),
        }
    }
}
