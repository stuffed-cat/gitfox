use crate::config::RunnerConfig;
use crate::error::{Result, RunnerError};
use crate::messages::Job;
use log::info;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use super::ExecutorTrait;

pub struct DockerExecutor<'a> {
    config: &'a RunnerConfig,
}

impl<'a> DockerExecutor<'a> {
    pub fn new(config: &'a RunnerConfig) -> Self {
        Self { config }
    }

    /// 解析要使用的镜像：优先从 Job 变量 CI_DOCKER_IMAGE 取，其次 default_image
    fn resolve_image<'b>(&self, job: &'b Job) -> &'b str
    where
        'a: 'b,
    {
        if let Some(img) = job.variables.get("CI_DOCKER_IMAGE") {
            return img.as_str();
        }
        &self.config.default_image
    }
}

impl<'a> ExecutorTrait for DockerExecutor<'a> {
    fn execute<F>(&self, job: &Job, mut log_callback: F) -> Result<i32>
    where
        F: FnMut(&str),
    {
        info!("[Docker] Starting job {} '{}'", job.id, job.name);
        info!("[Docker] Repository: {}", job.repository_url);
        info!("[Docker] Commit: {} (ref: {})", &job.commit_sha[..8.min(job.commit_sha.len())], job.ref_name);

        let image = self.resolve_image(job).to_owned();
        if image.is_empty() {
            return Err(RunnerError::Execution(
                "No Docker image specified. Set default_image in config or CI_DOCKER_IMAGE variable.".to_string(),
            ));
        }

        log_callback(&format!("Using Docker image: {}\n", image));

        // Pull image
        info!("[Docker] Pulling image: {}", image);
        log_callback(&format!("Pulling image {}...\n", image));
        let pull_result = Command::new("docker")
            .args(&["pull", &image])
            .output()?;

        if !pull_result.status.success() {
            let error_msg = String::from_utf8_lossy(&pull_result.stderr).to_string();
            return Err(RunnerError::Execution(format!(
                "Failed to pull image '{}': {}",
                image, error_msg
            )));
        }

        let container_name = format!("gitfox-job-{}", job.id);
        info!("[Docker] Container name: {}", container_name);
        info!("[Docker] Network mode: {}", self.config.network_mode);

        // Build docker run command
        let mut docker_cmd = Command::new("docker");
        docker_cmd.args(&[
            "run",
            "--rm",
            "--name",
            &container_name,
            "-w",
            "/builds",
        ]);

        // Resource limits (防止滥用宿主机资源)
        docker_cmd.args(&["--memory", "2g", "--cpus", "2"]);

        // 网络模式
        match self.config.network_mode.as_str() {
            "none" => {
                docker_cmd.args(&["--network", "none"]);
            }
            "restricted" => {
                docker_cmd.args(&["--network", "bridge"]);
            }
            _ => {} // "full" - 默认 bridge
        }

        // Environment variables
        log_callback(&format!("Environment variables: {} total (including {} CI_* variables)\n",
            job.variables.len(),
            job.variables.keys().filter(|k| k.starts_with("CI_") || k == &"CI" || k == &"GITFOX_CI").count()
        ));
        for (key, value) in &job.variables {
            docker_cmd.args(&["-e", &format!("{}={}", key, value)]);
        }

        docker_cmd.arg(&image);
        docker_cmd.args(&["sh", "-c"]);

        // Build combined script: clone + checkout + user scripts
        let mut full_script = format!(
            "set -e\ngit clone --depth=1 '{repo}' /builds && cd /builds && git checkout '{sha}'\n",
            repo = job.repository_url,
            sha = job.commit_sha
        );
        for line in &job.script {
            full_script.push_str(line);
            full_script.push('\n');
        }

        docker_cmd.arg(&full_script);
        docker_cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        log_callback(&format!("\n=== Executing stage: {} ===\n", job.stage));

        let mut child = docker_cmd.spawn()?;

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                log_callback(&format!("{}\n", line));
            }
        }

        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                log_callback(&format!("{}\n", line));
            }
        }

        let status = child.wait()?;
        let exit_code = status.code().unwrap_or(1);

        if exit_code != 0 && !job.allow_failure {
            return Err(RunnerError::Execution(format!(
                "Docker job failed with exit code {}",
                exit_code
            )));
        }

        log_callback("\n=== Job completed successfully ===\n");
        Ok(0)
    }
}
