use crate::error::{Result, RunnerError};
use crate::security::SecurityContext;
use std::process::Command;
use std::path::Path;
use uuid::Uuid;

/// macOS 动态用户隔离 - 纯 Rust 实现，完全无 Shell
/// dscl 命令直接通过 Command API 调用，sudo -u env -i -C 直接 exec
/// 返回 MacosUserGuard，Drop 时自动删除临时用户
#[allow(dead_code)]
pub fn apply_isolation(ctx: &SecurityContext, cmd: &mut Command) -> Result<MacosUserGuard> {
    let work_dir = Path::new(&ctx.work_dir)
        .canonicalize()
        .map_err(|e| RunnerError::Execution(format!("Invalid work_dir: {}", e)))?;

    // 提取原始命令（OsStr，不经任何字符串转换）
    let original_program = cmd.get_program().to_owned();
    let original_args: Vec<_> = cmd.get_args().map(|a| a.to_owned()).collect();
    let original_envs: Vec<(_, _)> = cmd.get_envs()
        .filter_map(|(k, v)| v.map(|v| (k.to_owned(), v.to_owned())))
        .collect();

    // UUID 后 8 位生成用户名，避免与现有用户碰撞
    let uid_suffix = &Uuid::new_v4().simple().to_string()[..8];
    let username = format!("_gfxjob_{}", uid_suffix);

    // 在 Rust 中查询已用 UID，找第一个 20000-29999 范围内的空闲值
    let uid = find_available_uid(20000, 29999)?;

    // 创建用户（全部用 Command API，零 Shell 介入）
    create_user(&username, uid, &work_dir)?;

    // 构造执行命令：sudo -u <user> env -i -C <work_dir> [KEY=VAL...] <prog> [args...]
    let mut new_cmd = Command::new("sudo");
    new_cmd.args(["-u", &username, "env", "-i", "-C"]);
    new_cmd.arg(&work_dir);

    for (k, v) in &original_envs {
        let mut kv = k.clone();
        kv.push("=");
        kv.push(v);
        new_cmd.arg(kv);
    }

    new_cmd.arg(&original_program);
    new_cmd.args(&original_args);
    new_cmd.stdout(std::process::Stdio::piped());
    new_cmd.stderr(std::process::Stdio::piped());

    *cmd = new_cmd;

    log::info!("✓ macOS user isolation: user={} uid={}", username, uid);
    Ok(MacosUserGuard { username, work_dir_owned_by_user: work_dir.to_path_buf() })
}

/// 通过 dscl 查询已使用的 UID，在 [min, max] 范围内找第一个空闲值
fn find_available_uid(min: u32, max: u32) -> Result<u32> {
    // dscl . -list /Users UniqueID
    // 输出格式：<username>  <uid>
    let output = Command::new("dscl")
        .args([".", "-list", "/Users", "UniqueID"])
        .output()
        .map_err(|e| RunnerError::Execution(format!("dscl list failed: {}", e)))?;

    let used_uids: std::collections::HashSet<u32> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            line.split_whitespace().nth(1)?.parse::<u32>().ok()
        })
        .collect();

    (min..=max)
        .find(|uid| !used_uids.contains(uid))
        .ok_or_else(|| RunnerError::Execution(
            format!("No available UID in range {}-{}", min, max)
        ))
}

/// 用 dscl 创建隔离用户，并设置工作目录权限
/// 所有参数为 &str / &Path，通过 Command::arg 传递，零 Shell 注入
fn create_user(username: &str, uid: u32, work_dir: &std::path::Path) -> Result<()> {
    let uid_str = uid.to_string();
    let user_path = format!("/Users/{}", username);

    // dscl . -create /Users/<name>
    run_dscl(&[".", "-create", &user_path])?;
    // Shell 设为 /usr/bin/false：禁止登录
    run_dscl(&[".", "-create", &user_path, "UserShell", "/usr/bin/false"])?;
    run_dscl(&[".", "-create", &user_path, "RealName", "GitFox Job"])?;
    run_dscl(&[".", "-create", &user_path, "UniqueID", &uid_str])?;
    // PrimaryGroupID 20 = staff（macOS 标准 nobody-ish 组）
    run_dscl(&[".", "-create", &user_path, "PrimaryGroupID", "20"])?;
    run_dscl(&[".", "-create", &user_path, "NFSHomeDirectory", "/var/empty"])?;
    // IsHidden=1 不在登录界面显示
    run_dscl(&[".", "-create", &user_path, "IsHidden", "1"])?;

    // 工作目录归属临时用户，mode 700（只有该用户可读写执行）
    let status = Command::new("sudo")
        .args(["chown", "-R"])
        .arg(username)
        .arg(work_dir)
        .status()
        .map_err(|e| RunnerError::Execution(format!("chown failed: {}", e)))?;
    if !status.success() {
        return Err(RunnerError::Execution("chown work_dir failed".into()));
    }

    let status = Command::new("sudo")
        .args(["chmod", "700"])
        .arg(work_dir)
        .status()
        .map_err(|e| RunnerError::Execution(format!("chmod failed: {}", e)))?;
    if !status.success() {
        return Err(RunnerError::Execution("chmod work_dir failed".into()));
    }

    Ok(())
}

fn run_dscl(args: &[&str]) -> Result<()> {
    let status = Command::new("sudo")
        .arg("dscl")
        .args(args)
        .status()
        .map_err(|e| RunnerError::Execution(format!("dscl failed: {}", e)))?;
    if !status.success() {
        return Err(RunnerError::Execution(
            format!("dscl {} failed", args.join(" "))
        ));
    }
    Ok(())
}

/// RAII 守卫：Drop 时删除临时用户并还原工作目录所有权
pub struct MacosUserGuard {
    username: String,
    work_dir_owned_by_user: std::path::PathBuf,
}

impl Drop for MacosUserGuard {
    fn drop(&mut self) {
        log::info!("[macos cleanup] deleting user: {}", self.username);

        // 还原工作目录所有权给 runner 进程所有者
        // SAFETY: getuid() 纯读取 syscall
        let runner_uid = unsafe { libc::getuid() };
        let _ = Command::new("sudo")
            .args(["chown", "-R", &runner_uid.to_string()])
            .arg(&self.work_dir_owned_by_user)
            .status();

        // 删除用户
        let user_path = format!("/Users/{}", self.username);
        let _ = Command::new("sudo")
            .args(["dscl", ".", "-delete", &user_path])
            .status();

        log::info!("[macos cleanup] done: {}", self.username);
    }
}
