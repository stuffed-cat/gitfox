use crate::config::RunnerConfig;
use crate::error::{Result, RunnerError};
use crate::messages::Job;
use crate::security::SecurityContext;
use log::{info, warn};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::path::PathBuf;

pub struct Executor {
    config: RunnerConfig,
}

impl Executor {
    pub fn new(config: RunnerConfig) -> Self {
        // Ensure builds directory exists
        if let Err(e) = std::fs::create_dir_all(&config.builds_dir) {
            warn!("Failed to create builds directory: {}", e);
        }
        
        Self { config }
    }

    pub fn execute<F>(&self, job: &Job, mut log_callback: F) -> Result<i32>
    where
        F: FnMut(&str),
    {
        match self.config.executor.as_str() {
            "shell" => self.execute_shell(job, &mut log_callback),
            "docker" => self.execute_docker(job, &mut log_callback),
            _ => Err(RunnerError::Execution(format!(
                "Unknown executor type: {}",
                self.config.executor
            ))),
        }
    }

    fn execute_shell<F>(&self, job: &Job, log_callback: &mut F) -> Result<i32>
    where
        F: FnMut(&str),
    {
        info!("Executing job {} with shell executor", job.id);

        // Use configured builds directory instead of /tmp
        let job_dir = self.config.builds_dir.join(format!("job-{}", job.id));
        std::fs::create_dir_all(&job_dir)?;

        // SECURITY: Don't output the actual directory path to prevent information disclosure
        log_callback("Preparing build environment...\n");

        // Clone repository
        log_callback(&format!("Cloning repository...\n"));
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
            log_callback(&format!(
                "Checkout failed: {}\n",
                error_msg
            ));
            cleanup_job_dir(&job_dir);
            return Err(RunnerError::Execution(format!("Git checkout failed: {}", error_msg)));
        }

        // Execute scripts
        log_callback(&format!("\n=== Executing stage: {} ===\n", job.stage));

        // Create security context for isolation
        let security_ctx = SecurityContext::new(
            self.config.security_enabled,
            &job_dir,
            &self.config.network_mode,
        );

        for (_idx, script_line) in job.script.iter().enumerate() {
            log_callback(&format!("$ {}\n", script_line));

            let mut cmd = Command::new("sh");
            cmd.arg("-c")
                .arg(script_line)
                .current_dir(&job_dir)
                .envs(&job.variables)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            // Apply security isolation
            if self.config.security_enabled {
                if let Err(e) = security_ctx.wrap_command(&mut cmd) {
                    log_callback(&format!("\n⚠️  Security isolation failed: {}\n", e));
                    log_callback("⚠️  Continuing without isolation...\n");
                }
            } else {
                log_callback("\n🚨 WARNING: Security isolation is DISABLED\n");
            }

            let mut child = cmd.spawn()?;

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
                if self.config.clean_builds {
                    cleanup_job_dir(&job_dir);
                }
                return Err(RunnerError::Execution(format!(
                    "Script '{}' failed with exit code {}",
                    script_line, exit_code
                )));
            }
            
            // Check work directory size
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
                        "Work directory size exceeded limit".to_string()
                    ));
                }
            }
        }

        log_callback("\n=== Job completed successfully ===\n");

        // Cleanup
        if self.config.clean_builds {
            cleanup_job_dir(&job_dir);
        } else {
            log_callback(&format!("Build artifacts preserved for inspection\n"));
        }

        Ok(0)
    }

    fn execute_docker<F>(&self, job: &Job, log_callback: &mut F) -> Result<i32>
    where
        F: FnMut(&str),
    {
        info!("Executing job {} with docker executor", job.id);
        
        log_callback("Preparing Docker container...\n");
        
        // Use job's CI_DOCKER_IMAGE variable or default from config
        let image = job.variables.get("CI_DOCKER_IMAGE")
            .map(|s| s.as_str())
            .unwrap_or(&self.config.default_docker_image);
        
        log_callback(&format!("Using image: {}\n", image));
        
        // Pull image
        let pull_result = Command::new("docker")
            .args(&["pull", image])
            .output()?;
        
        if !pull_result.status.success() {
            let error_msg = String::from_utf8_lossy(&pull_result.stderr).to_string();
            return Err(RunnerError::Execution(format!("Failed to pull image: {}", error_msg)));
        }
        
        // Create container name
        let container_name = format!("gitfox-job-{}", job.id);
        
        // Build docker run command
        let mut docker_cmd = Command::new("docker");
        docker_cmd.args(&[
            "run",
            "--rm",
            "--name", &container_name,
            "-w", "/builds",
        ]);
        
        // Add environment variables
        for (key, value) in &job.variables {
            docker_cmd.args(&["-e", &format!("{}={}", key, value)]);
        }
        
        docker_cmd.arg(image);
        docker_cmd.arg("sh");
        docker_cmd.arg("-c");
        
        // Build script
        let mut full_script = format!(
            "git clone --depth=1 {} /builds && cd /builds && git checkout {}\n",
            job.repository_url, job.commit_sha
        );
        
        for script_line in &job.script {
            full_script.push_str(&format!("{}\n", script_line));
        }
        
        docker_cmd.arg(&full_script);
        docker_cmd.stdout(Stdio::piped());
        docker_cmd.stderr(Stdio::piped());
        
        log_callback(&format!("\n=== Executing stage: {} ===\n", job.stage));
        
        let mut child = docker_cmd.spawn()?;
        
        // Stream stdout
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    log_callback(&format!("{}\n", line));
                }
            }
        }
        
        // Stream stderr
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
        
        if exit_code != 0 {
            return Err(RunnerError::Execution(format!("Job failed with exit code {}", exit_code)));
        }
        
        log_callback("\n=== Job completed successfully ===\n");
        
        Ok(0)
    }
}

/// Check if a command is potentially dangerous
fn is_dangerous_command(cmd: &str) -> bool {
    let dangerous_patterns = [
        "rm -rf /",
        "rm -fr /",
        "dd if=",
        "mkfs",
        ":(){ :|:& };:", // fork bomb
        "chmod -R 777 /",
        "chown -R",
    ];
    
    dangerous_patterns.iter().any(|pattern| cmd.contains(pattern))
}

/// Clean up job directory
fn cleanup_job_dir(dir: &PathBuf) {
    if let Err(e) = std::fs::remove_dir_all(dir) {
        warn!("Failed to cleanup job directory: {}", e);
    }
}

/// Get directory size in MB
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
    
    Ok(total_size / (1024 * 1024)) // Convert to MB
}
