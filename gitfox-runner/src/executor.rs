use crate::error::{Result, RunnerError};
use crate::messages::Job;
use log::{debug, info};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

pub struct Executor {
    executor_type: String,
}

impl Executor {
    pub fn new(executor_type: &str) -> Self {
        Self {
            executor_type: executor_type.to_string(),
        }
    }

    pub fn execute<F>(&self, job: &Job, mut log_callback: F) -> Result<i32>
    where
        F: FnMut(&str),
    {
        match self.executor_type.as_str() {
            "shell" => self.execute_shell(job, &mut log_callback),
            "docker" => self.execute_docker(job, &mut log_callback),
            _ => Err(RunnerError::Execution(format!(
                "Unknown executor type: {}",
                self.executor_type
            ))),
        }
    }

    fn execute_shell<F>(&self, job: &Job, log_callback: &mut F) -> Result<i32>
    where
        F: FnMut(&str),
    {
        info!("Executing job {} with shell executor", job.id);

        // Create temporary directory for job
        let job_dir = std::env::temp_dir().join(format!("gitfox-job-{}", job.id));
        std::fs::create_dir_all(&job_dir)?;

        log_callback(&format!("Job directory: {:?}\n", job_dir));

        // Clone repository
        log_callback(&format!("Cloning repository: {}\n", job.repository_url));
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
            log_callback(&format!(
                "Clone failed: {}\n",
                error_msg
            ));
            return Err(RunnerError::Execution(format!("Git clone failed: {}", error_msg)));
        }

        // Checkout specific commit
        log_callback(&format!("Checking out commit: {}\n", job.commit_sha));
        let checkout_result = Command::new("git")
            .args(&["checkout", &job.commit_sha])
            .current_dir(&job_dir)
            .output()?;

        if !checkout_result.status.success() {
            let error_msg = String::from_utf8_lossy(&checkout_result.stderr).to_string();
            log_callback(&format!(
                "Checkout failed: {}\n",
                error_msg
            ));
            return Err(RunnerError::Execution(format!("Git checkout failed: {}", error_msg)));
        }

        // Execute scripts
        log_callback(&format!("\n=== Executing stage: {} ===\n", job.stage));

        for (idx, script_line) in job.script.iter().enumerate() {
            log_callback(&format!("$ {}\n", script_line));

            let mut child = Command::new("sh")
                .arg("-c")
                .arg(script_line)
                .current_dir(&job_dir)
                .envs(&job.variables)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            // Stream stdout
            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        log_callback(&format!("{}\n", line));
                    }
                }
            }

            // Get stderr
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
                log_callback(&format!(
                    "\nScript failed with exit code: {}\n",
                    exit_code
                ));
                // Cleanup
                let _ = std::fs::remove_dir_all(&job_dir);
                return Err(RunnerError::Execution(format!(
                    "Script '{}' failed with exit code {}",
                    script_line, exit_code
                )));
            }
        }

        log_callback("\n=== Job completed successfully ===\n");

        // Cleanup
        let _ = std::fs::remove_dir_all(&job_dir);

        Ok(0)
    }

    fn execute_docker<F>(&self, job: &Job, log_callback: &mut F) -> Result<i32>
    where
        F: FnMut(&str),
    {
        log_callback("Docker executor not implemented yet\n");
        // TODO: Implement Docker executor
        Ok(0)
    }
}
