mod linux;
mod windows;
mod macos;
mod freebsd;

use crate::error::Result;
use std::path::Path;
use std::process::Command;

pub struct SecurityContext {
    pub enabled: bool,
    pub work_dir: String,
    pub network_mode: String,
}

impl SecurityContext {
    pub fn new(enabled: bool, work_dir: &Path, network_mode: &str) -> Self {
        Self {
            enabled,
            work_dir: work_dir.to_string_lossy().to_string(),
            network_mode: network_mode.to_string(),
        }
    }

    /// Wrap command with platform-specific security isolation
    pub fn wrap_command(&self, cmd: &mut Command) -> Result<()> {
        if !self.enabled {
            log::error!("🚨 SECURITY WARNING: Isolation is DISABLED!");
            log::error!("   This is EXTREMELY DANGEROUS in production!");
            log::error!("   Set security_enabled=true in config.toml");
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            linux::apply_isolation(self, cmd)?;
        }

        #[cfg(target_os = "windows")]
        {
            windows::apply_isolation(self, cmd)?;
        }

        #[cfg(target_os = "macos")]
        {
            macos::apply_isolation(self, cmd)?;
        }

        #[cfg(target_os = "freebsd")]
        {
            freebsd::apply_isolation(self, cmd)?;
        }

        Ok(())
    }
}

/// Filter sensitive environment variables
pub fn filter_environment(
    vars: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, String> {
    let mut filtered = std::collections::HashMap::new();

    let sensitive_patterns = [
        "TOKEN", "SECRET", "PASSWORD", "KEY", "CREDENTIAL",
        "AWS_", "AZURE_", "GCP_", "GITHUB_TOKEN",
    ];

    for (key, value) in vars {
        if key == "RUNNER_TOKEN" || key == "CI_JOB_TOKEN" {
            filtered.insert(key.clone(), "***REDACTED***".to_string());
            continue;
        }

        let is_sensitive = sensitive_patterns
            .iter()
            .any(|pattern| key.to_uppercase().contains(pattern));

        if is_sensitive {
            log::warn!("⚠️  Redacting sensitive variable: {}", key);
            filtered.insert(key.clone(), "***FILTERED***".to_string());
        } else {
            filtered.insert(key.clone(), value.clone());
        }
    }

    filtered
}
