use crate::error::{Result, RunnerError};
use crate::security::SecurityContext;
use std::process::Command;
use std::path::Path;
use std::fs;
use std::ffi::CString;
use std::os::unix::process::CommandExt;

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

    // 纯 Rust 实现：在子进程 pre_exec 中完成 unshare + overlay 根挂载
    // 不再依赖外部 `unshare` 或 `env` 命令，也不会触发任何 shell 解析。
    // 环境变量通过 Command 的 env_clear/ env() 控制。

    // 准备新的命令，保留用户原始程序、参数和环境
    let mut new_cmd = Command::new(&original_program);
    new_cmd.args(&original_args);
    new_cmd.env_clear();

    for (k, v) in &original_envs {
        new_cmd.env(k, v);
    }

    new_cmd.current_dir(&work_dir);
    new_cmd.stdout(std::process::Stdio::piped());
    new_cmd.stderr(std::process::Stdio::piped());

    // 在 fork 之后、exec 之前执行隔离逻辑
    // 先在父进程中准备所有需要的目录和 C 字符串，避免 pre_exec 中的分配
    let work_dir_clone = work_dir.clone();

    let overlay_base = work_dir_clone.join(".overlay_root");
    let upper = overlay_base.join("upper");
    let work = overlay_base.join("work");
    fs::create_dir_all(&upper)?;
    fs::create_dir_all(&work)?;

    // 预构造所有 CStr/CString
    let c_overlay = CString::new("overlay").unwrap();
    let c_root = CString::new("/").unwrap();
    let opts = CString::new(format!(
        "lowerdir=/,upperdir={},workdir={}",
        upper.display(),
        work.display()
    ))
    .unwrap();

    unsafe {
        new_cmd.pre_exec(move || {
            // 1. 创建命名空间
            let mut flags = libc::CLONE_NEWNS
                | libc::CLONE_NEWPID
                | libc::CLONE_NEWIPC
                | libc::CLONE_NEWUTS;
            if isolate_net {
                flags |= libc::CLONE_NEWNET;
            }
            if unsafe { libc::unshare(flags) } != 0 {
                return Err(std::io::Error::last_os_error());
            }

            // 2. 将 / 标记为 private，防止挂载传播到宿主
            if unsafe {
                libc::mount(
                    std::ptr::null(),
                    c_root.as_ptr(),
                    std::ptr::null(),
                    libc::MS_REC | libc::MS_PRIVATE,
                    std::ptr::null(),
                )
            } != 0
            {
                return Err(std::io::Error::last_os_error());
            }

            // 3. 使用 overlayfs 覆盖整个根文件系统
            if unsafe {
                libc::mount(
                    c_overlay.as_ptr(),
                    c_root.as_ptr(),
                    c_overlay.as_ptr(),
                    0,
                    opts.as_ptr() as *const libc::c_void,
                )
            } != 0
            {
                return Err(std::io::Error::last_os_error());
            }

            Ok(())
        });
    }

    *cmd = new_cmd;

    log::info!("✓ Linux namespace isolation with overlayfs root (pure Rust, no shell)");
    Ok(())
}
