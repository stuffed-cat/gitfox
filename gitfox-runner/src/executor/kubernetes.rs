use crate::config::RunnerConfig;
use crate::error::{Result, RunnerError};
use crate::messages::Job;
use log::info;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use super::ExecutorTrait;

/// Kubernetes 执行器，通过 kubectl 创建一次性 Pod 来运行 CI 任务。
/// 依赖宿主机安装了 kubectl，并拥有对目标集群的访问权限。
pub struct KubernetesExecutor<'a> {
    config: &'a RunnerConfig,
}

impl<'a> KubernetesExecutor<'a> {
    pub fn new(config: &'a RunnerConfig) -> Self {
        Self { config }
    }

    fn resolve_image<'b>(&self, job: &'b Job) -> &'b str
    where
        'a: 'b,
    {
        if let Some(img) = job.variables.get("CI_KUBERNETES_IMAGE") {
            return img.as_str();
        }
        if let Some(img) = job.variables.get("CI_DOCKER_IMAGE") {
            return img.as_str();
        }
        &self.config.default_image
    }

    /// 组装 kubectl 的 --kubeconfig / --namespace 参数
    fn kubectl_base_args(&self) -> Vec<String> {
        let mut args: Vec<String> = vec![];

        if let Some(ref kc) = self.config.kubernetes_config {
            args.push("--kubeconfig".to_string());
            args.push(kc.clone());
        }

        args.push("--namespace".to_string());
        args.push(self.config.kubernetes_namespace.clone());

        args
    }

    /// 等待 Pod 进入 Running 状态（最多 120 秒）
    fn wait_for_pod_running(&self, pod_name: &str) -> Result<()> {
        let base = self.kubectl_base_args();
        for _ in 0..60 {
            let mut cmd = Command::new("kubectl");
            cmd.args(&base);
            cmd.args(&[
                "get",
                "pod",
                pod_name,
                "-o",
                "jsonpath={.status.phase}",
            ]);

            let out = cmd.output()?;
            let phase = String::from_utf8_lossy(&out.stdout);

            match phase.trim() {
                "Running" | "Succeeded" => return Ok(()),
                "Failed" | "Unknown" => {
                    return Err(RunnerError::Execution(format!(
                        "Pod '{}' entered unexpected phase: {}",
                        pod_name,
                        phase.trim()
                    )))
                }
                _ => {}
            }

            std::thread::sleep(std::time::Duration::from_secs(2));
        }

        Err(RunnerError::Execution(format!(
            "Timed out waiting for pod '{}' to be Running",
            pod_name
        )))
    }

    /// 删除 Pod（清理）
    fn delete_pod(&self, pod_name: &str) {
        let base = self.kubectl_base_args();
        let mut cmd = Command::new("kubectl");
        cmd.args(&base);
        cmd.args(&["delete", "pod", pod_name, "--ignore-not-found", "--wait=false"]);
        let _ = cmd.output();
    }
}

impl<'a> ExecutorTrait for KubernetesExecutor<'a> {
    fn execute<F>(&self, job: &Job, mut log_callback: F) -> Result<i32>
    where
        F: FnMut(&str),
    {
        info!("[K8s] Starting job {} '{}'", job.id, job.name);
        info!("[K8s] Repository: {}", job.repository_url);
        info!("[K8s] Commit: {} (ref: {})", &job.commit_sha[..8.min(job.commit_sha.len())], job.ref_name);
        info!("[K8s] Namespace: {}", self.config.kubernetes_namespace);

        let image = self.resolve_image(job).to_owned();
        if image.is_empty() {
            return Err(RunnerError::Execution(
                "No Kubernetes image specified. Set default_image in config or CI_KUBERNETES_IMAGE variable.".to_string(),
            ));
        }

        log_callback(&format!("Using Kubernetes image: {}\n", image));
        log_callback(&format!(
            "Namespace: {}\n",
            self.config.kubernetes_namespace
        ));

        let pod_name = format!(
            "gitfox-job-{}-{}",
            job.id,
            &job.commit_sha[..8].to_lowercase()
        );

        // Build the init script (clone + checkout + user scripts)
        let mut full_script = format!(
            "set -e\ngit clone --depth=1 '{repo}' /builds && cd /builds && git checkout '{sha}'\n",
            repo = job.repository_url,
            sha = job.commit_sha
        );
        for line in &job.script {
            full_script.push_str(line);
            full_script.push('\n');
        }

        // Build env vars string for Pod spec
        let env_yaml: String = job
            .variables
            .iter()
            .map(|(k, v)| {
                format!(
                    "        - name: {k}\n          value: {v}\n",
                    k = k,
                    v = shell_escape_yaml(v)
                )
            })
            .collect();

        // Script is passed via env var to avoid shell quoting issues
        let script_escaped = shell_escape_yaml(&full_script);

        let pod_manifest = format!(
            r#"apiVersion: v1
kind: Pod
metadata:
  name: {pod_name}
  labels:
    app: gitfox-runner
    job-id: "{job_id}"
spec:
  restartPolicy: Never
  containers:
    - name: job
      image: {image}
      command: ["sh", "-c"]
      args:
        - $(GITFOX_SCRIPT)
      env:
        - name: GITFOX_SCRIPT
          value: {script}
{env_yaml}      resources:
        limits:
          memory: "2Gi"
          cpu: "2"
        requests:
          memory: "256Mi"
          cpu: "250m"
      workingDir: /builds
"#,
            pod_name = pod_name,
            job_id = job.id,
            image = image,
            script = script_escaped,
            env_yaml = env_yaml,
        );

        // Write manifest to temp file
        let manifest_path = std::env::temp_dir().join(format!("gitfox-pod-{}.yaml", job.id));
        std::fs::write(&manifest_path, &pod_manifest).map_err(|e| {
            RunnerError::Execution(format!("Failed to write Pod manifest: {}", e))
        })?;

        log_callback(&format!("Creating Pod: {}\n", pod_name));
        info!("[K8s] Creating Pod: {}", pod_name);
        info!("[K8s] Image: {}", image);

        let base = self.kubectl_base_args();

        // Apply manifest
        let apply_out = Command::new("kubectl")
            .args(&base)
            .args(&["apply", "-f", manifest_path.to_str().unwrap()])
            .output()?;

        let _ = std::fs::remove_file(&manifest_path);

        if !apply_out.status.success() {
            let err = String::from_utf8_lossy(&apply_out.stderr);
            return Err(RunnerError::Execution(format!(
                "Failed to create Pod: {}",
                err
            )));
        }

        log_callback("Waiting for Pod to start...\n");
        if let Err(e) = self.wait_for_pod_running(&pod_name) {
            self.delete_pod(&pod_name);
            return Err(e);
        }

        log_callback(&format!("\n=== Executing stage: {} ===\n", job.stage));

        // Stream logs
        let mut logs_cmd = Command::new("kubectl");
        logs_cmd.args(&base);
        logs_cmd.args(&["logs", "-f", &pod_name]);
        logs_cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = logs_cmd.spawn()?;

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

        let _ = child.wait();

        // Get Pod exit code
        let exit_code = get_pod_exit_code(&base, &pod_name);

        // Cleanup
        log_callback(&format!("Cleaning up Pod {}...\n", pod_name));
        self.delete_pod(&pod_name);

        if exit_code != 0 && !job.allow_failure {
            return Err(RunnerError::Execution(format!(
                "Kubernetes job failed with exit code {}",
                exit_code
            )));
        }

        log_callback("\n=== Job completed successfully ===\n");
        Ok(exit_code)
    }
}

/// 获取 Pod 中容器的退出码（通过 jsonpath）
fn get_pod_exit_code(base_args: &[String], pod_name: &str) -> i32 {
    let out = Command::new("kubectl")
        .args(base_args)
        .args(&[
            "get",
            "pod",
            pod_name,
            "-o",
            "jsonpath={.status.containerStatuses[0].state.terminated.exitCode}",
        ])
        .output();

    if let Ok(out) = out {
        let s = String::from_utf8_lossy(&out.stdout);
        if let Ok(code) = s.trim().parse::<i32>() {
            return code;
        }
    }
    1
}

/// 对字符串进行简单的 YAML 双引号转义（以便嵌入到 YAML value 中）
fn shell_escape_yaml(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    format!("\"{}\"", escaped)
}
