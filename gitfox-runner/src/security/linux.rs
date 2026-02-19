use crate::error::{Result, RunnerError};
use crate::security::SecurityContext;
use std::process::Command;
use std::path::Path;

/// Linux namespace isolation - 纯 Rust 实现，完全无 Shell
/// unshare 创建隔离命名空间，env -i -C 完成 chdir + 清空环境，直接 exec
/// 所有参数为独立 OsStr，零注入风险，无临时文件，无 trap
#[allow(dead_code)]
pub fn apply_isolation(ctx: &SecurityContext, cmd: &mut Command) -> Result<()> {
    let work_dir = Path::new(&ctx.work_dir)
        .canonicalize()
        .map_err(|e| RunnerError::Execution(format!("Invalid work_dir: {}", e)))?;

    // 提取原始命令（OsStr，不经任何字符串转换）
    let original_program = cmd.get_program().to_owned();
    let original_args: Vec<_> = cmd.get_args().map(|a| a.to_owned()).collect();
    let original_envs: Vec<(_, _)> = cmd.get_envs()
        .filter_map(|(k, v)| v.map(|v| (k.to_owned(), v.to_owned())))
        .collect();

    let isolate_net = ctx.network_mode == "none";

    // unshare 本身需要 CAP_SYS_ADMIN（root 或 sudo）
    // SAFETY: getuid() 是纯读取 syscall
    let uid = unsafe { libc::getuid() };
    let mut new_cmd = if uid == 0 {
        Command::new("unshare")
    } else {
        let mut c = Command::new("sudo");
        c.arg("unshare");
        c
    };

    // 命名空间标志：
    //   --mount    : 隔离挂载表（子进程挂载不影响宿主）
    //   --pid      : 隔离 PID（子进程看不到宿主进程）
    //   --ipc      : 隔离 IPC（消息队列、共享内存）
    //   --uts      : 隔离主机名
    //   --fork     : fork 后 exec（PID namespace 要求）
    //   --mount-proc: 在新 pid ns 中挂载新 /proc
    //   --net      : 隔离网络（可选）
    new_cmd.args(["--mount", "--pid", "--ipc", "--uts", "--fork", "--mount-proc"]);
    if isolate_net {
        new_cmd.arg("--net");
    }

    // -- 分隔符后接 env -i -C work_dir
    // env 标志：
    //   -i      : 清空继承的宿主环境变量（不泄露 TOKEN/SECRET 等）
    //   -C <dir>: 在 exec 前 chdir（GNU coreutils env 原生支持）
    //
    // 整个参数链全为独立 OsStr，Rust 直接 execvp，
    // 内核不经过任何 shell 解析，注入面为零
    new_cmd.arg("--");
    new_cmd.arg("env");
    new_cmd.arg("-i");
    new_cmd.arg("-C").arg(&work_dir);

    // 显式传入作业所需的环境变量（KEY=VALUE，逐个 OsStr）
    for (k, v) in &original_envs {
        let mut kv = k.clone();
        kv.push("=");
        kv.push(v);
        new_cmd.arg(kv);
    }

    // 最后是原始程序和参数（全部 OsStr，零注入）
    new_cmd.arg(&original_program);
    new_cmd.args(&original_args);

    new_cmd.stdout(std::process::Stdio::piped());
    new_cmd.stderr(std::process::Stdio::piped());

    *cmd = new_cmd;

    log::info!("✓ Linux namespace isolation (unshare, pure Rust, no shell, no temp files)");
    Ok(())
}
