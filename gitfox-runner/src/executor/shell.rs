use crate::config::RunnerConfig;
use crate::error::{Result, RunnerError};
use crate::messages::Job;
use crate::security::{IsolationGuard, SecurityContext};
use log::{info, warn};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::ExecutorTrait;

pub struct ShellExecutor<'a> {
    config: &'a RunnerConfig,
}

impl<'a> ShellExecutor<'a> {
    pub fn new(config: &'a RunnerConfig) -> Self {
        Self { config }
    }
}

impl<'a> ExecutorTrait for ShellExecutor<'a> {
    fn execute<F>(&self, job: &Job, mut log_callback: F) -> Result<i32>
    where
        F: FnMut(&str),
    {
        info!("[Shell] Starting job {} '{}'", job.id, job.name);
        info!("[Shell] Repository: {}", job.repository_url);
        info!("[Shell] Commit: {} (ref: {})", &job.commit_sha[..8.min(job.commit_sha.len())], job.ref_name);

        let job_dir = self.config.builds_dir.join(format!("job-{}", job.id));
        info!("[Shell] Work directory: {}", job_dir.display());
        std::fs::create_dir_all(&job_dir)?;

        log_callback("Preparing build environment...\n");

        // Clone repository
        log_callback("Cloning repository...\n");
        let clone_result = Command::new("git")
            .args(&[
                "clone",
                "--depth=1",
                &job.repository_url,
                job_dir.to_str().unwrap(),
            ])
            .output()?;

        if !clone_result.status.success() {
            let error_msg = String::from_utf8_lossy(&clone_result.stderr).to_string();
            log_callback(&format!("Clone failed: {}\n", error_msg));
            cleanup_job_dir(&job_dir);
            return Err(RunnerError::Execution(format!("Git clone failed: {}", error_msg)));
        }

        // Checkout specific commit
        log_callback(&format!("Checking out {}\n", &job.commit_sha[..8]));
        let checkout_result = Command::new("git")
            .args(&["checkout", &job.commit_sha])
            .current_dir(&job_dir)
            .output()?;

        if !checkout_result.status.success() {
            let error_msg = String::from_utf8_lossy(&checkout_result.stderr).to_string();
            log_callback(&format!("Checkout failed: {}\n", error_msg));
            cleanup_job_dir(&job_dir);
            return Err(RunnerError::Execution(format!(
                "Git checkout failed: {}",
                error_msg
            )));
        }

        log_callback(&format!("\n=== Executing stage: {} ===\n", job.stage));
        log_callback(&format!("Environment variables: {} total (including {} CI_* variables)\n", 
            job.variables.len(),
            job.variables.keys().filter(|k| k.starts_with("CI_") || k == &"CI" || k == &"GITFOX_CI").count()
        ));

        let security_ctx = SecurityContext::new(
            self.config.security_enabled,
            &job_dir,
            &self.config.network_mode,
        );

        for script_line in &job.script {
            log_callback(&format!("$ {}\n", script_line));

            let mut cmd = Command::new("sh");
            cmd.arg("-c")
                .arg(script_line)
                .current_dir(&job_dir)
                .envs(&job.variables)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let _guard: IsolationGuard;
            if self.config.security_enabled {
                match security_ctx.wrap_command(&mut cmd) {
                    Ok(guard) => _guard = guard,
                    Err(e) => {
                        log_callback(&format!("\n⚠️  Security isolation failed: {}\n", e));
                        log_callback("⚠️  Continuing without isolation...\n");
                        _guard = IsolationGuard::none();
                    }
                }
            } else {
                log_callback("\n🚨 WARNING: Security isolation is DISABLED\n");
                _guard = IsolationGuard::none();
            }

            let mut child = cmd.spawn()?;

            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        log_callback(&format!("{}\n", line));
                    }
                }
            }

            if let Some(stderr) = child.stderr.take() {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        log_callback(&format!("{}\n", line));
                    }
                }
            }

            let status = child.wait()?;
            let exit_code = status.code().unwrap_or(1);

            if exit_code != 0 && !job.allow_failure {
                log_callback(&format!("\nScript failed with exit code: {}\n", exit_code));
                if self.config.clean_builds {
                    cleanup_job_dir(&job_dir);
                }
                return Err(RunnerError::Execution(format!(
                    "Script '{}' failed with exit code {}",
                    script_line, exit_code
                )));
            }

            if let Ok(size_mb) = get_dir_size_mb(&job_dir) {
                if size_mb > self.config.max_work_dir_size_mb {
                    log_callback(&format!(
                        "\n⚠️  Work directory exceeded size limit: {} MB / {} MB\n",
                        size_mb, self.config.max_work_dir_size_mb
                    ));
                    if self.config.clean_builds {
                        cleanup_job_dir(&job_dir);
                    }
                    return Err(RunnerError::Execution(
                        "Work directory size exceeded limit".to_string(),
                    ));
                }
            }
        }

        log_callback("\n=== Job completed successfully ===\n");

        if self.config.clean_builds {
            cleanup_job_dir(&job_dir);
        } else {
            log_callback("Build artifacts preserved for inspection\n");
        }

        Ok(0)
    }
}

fn cleanup_job_dir(dir: &PathBuf) {
    if let Err(e) = std::fs::remove_dir_all(dir) {
        warn!("Failed to cleanup job directory: {}", e);
    }
}

fn get_dir_size_mb(dir: &PathBuf) -> Result<u64> {
    let mut total_size: u64 = 0;
    if dir.is_dir() {
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total_size += metadata.len();
                }
            }
        }
    }
    Ok(total_size / (1024 * 1024))
}
