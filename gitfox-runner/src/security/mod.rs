mod linux;
mod windows;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(not(target_os = "macos"))]
mod macos;
#[cfg(target_os = "freebsd")]
pub mod freebsd;
#[cfg(not(target_os = "freebsd"))]
mod freebsd;

use crate::error::Result;
use std::path::Path;
use std::process::Command;

pub struct SecurityContext {
    pub enabled: bool,
    pub work_dir: String,
    pub network_mode: String,
}

/// 隔离后需要持有的守卫，Drop 时自动清理资源
/// - Linux:   空守卫（namespace 随进程退出自动销毁）
/// - Windows: 持有 WindowsUserGuard，Drop 时删除临时用户 + 还原 ACL
/// - macOS:   持有 MacosUserGuard，Drop 时删除临时用户
/// - FreeBSD: 持有 JailContext，Drop 时销毁 jail + 卸载挂载点
pub struct IsolationGuard {
    #[cfg(target_os = "windows")]
    _windows: windows::WindowsUserGuard,
    #[cfg(target_os = "macos")]
    _macos: macos::MacosUserGuard,
    #[cfg(target_os = "freebsd")]
    _jail: freebsd::JailContext,
}

impl IsolationGuard {
    /// Linux 空守卫（namespace 随进程退出自动销毁，无需主动清理）
    pub fn none() -> Self {
        IsolationGuard {
            #[cfg(target_os = "windows")]
            _windows: unreachable!(),
            #[cfg(target_os = "macos")]
            _macos: unreachable!(),
            #[cfg(target_os = "freebsd")]
            _jail: unreachable!(),
        }
    }

    #[cfg(target_os = "windows")]
    fn windows(guard: windows::WindowsUserGuard) -> Self {
        IsolationGuard { _windows: guard }
    }

    #[cfg(target_os = "macos")]
    fn macos(guard: macos::MacosUserGuard) -> Self {
        IsolationGuard { _macos: guard }
    }

    #[cfg(target_os = "freebsd")]
    fn jail(ctx: freebsd::JailContext) -> Self {
        IsolationGuard { _jail: ctx }
    }
}

impl SecurityContext {
    pub fn new(enabled: bool, work_dir: &Path, network_mode: &str) -> Self {
        Self {
            enabled,
            work_dir: work_dir.to_string_lossy().to_string(),
            network_mode: network_mode.to_string(),
        }
    }

    /// 包装命令并返回资源守卫
    /// 调用方必须在子进程 wait() 返回后才能 drop 守卫
    pub fn wrap_command(&self, cmd: &mut Command) -> Result<IsolationGuard> {
        if !self.enabled {
            log::error!("🚨 SECURITY WARNING: Isolation is DISABLED!");
            log::error!("   Set security_enabled=true in config.toml");
            return Ok(IsolationGuard::none());
        }

        #[cfg(target_os = "linux")]
        {
            linux::apply_isolation(self, cmd)?;
            return Ok(IsolationGuard::none());
        }

        #[cfg(target_os = "windows")]
        {
            let guard = windows::apply_isolation(self, cmd)?;
            return Ok(IsolationGuard::windows(guard));
        }

        #[cfg(target_os = "macos")]
        {
            let guard = macos::apply_isolation(self, cmd)?;
            return Ok(IsolationGuard::macos(guard));
        }

        #[cfg(target_os = "freebsd")]
        {
            let jail = freebsd::apply_isolation(self, cmd)?;
            return Ok(IsolationGuard::jail(jail));
        }

        #[allow(unreachable_code)]
        Ok(IsolationGuard::none())
    }
}

/// Filter sensitive environment variables
#[allow(dead_code)]
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
