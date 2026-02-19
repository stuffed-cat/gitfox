use crate::error::{Result, RunnerError};
use crate::security::SecurityContext;
use std::process::Command;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// FreeBSD jail isolation - 纯 Rust 实现
/// 返回 JailContext 由调用方持有，Drop 时自动清理所有资源
/// 使用 jexec + env -C 直接 exec，完全无 Shell
#[allow(dead_code)]
pub fn apply_isolation(ctx: &SecurityContext, cmd: &mut Command) -> Result<JailContext> {
    let work_dir = Path::new(&ctx.work_dir)
        .canonicalize()
        .map_err(|e| RunnerError::Execution(format!("Invalid work_dir: {}", e)))?;

    let original_program = cmd.get_program().to_owned();
    let original_args: Vec<_> = cmd.get_args().map(|a| a.to_owned()).collect();
    let original_envs: Vec<(_, _)> = cmd.get_envs()
        .filter_map(|(k, v)| v.map(|v| (k.to_owned(), v.to_owned())))
        .collect();

    let jail_name = format!("gfx_{}", &Uuid::new_v4().simple().to_string()[..12]);

    let mut jail = JailContext::new(jail_name, work_dir.clone())?;
    if ctx.network_mode == "none" {
        jail.network_enabled = false;
    }

    // setup 失败时 Drop 自动清理已完成的挂载
    jail.setup()?;

    // 替换 cmd：jexec <name> /usr/bin/env -C <work_dir> <program> [args...]
    // 完全无 Shell：所有参数为独立 OsStr，零注入风险
    *cmd = jail.make_exec_command(&original_program, &original_args, &original_envs);

    Ok(jail)
    // 调用方必须持有返回的 JailContext 直到子进程结束
    // Drop 时自动执行：jail -r, umount, devfs delset, rm -rf
}

/// RAII Jail 上下文
/// Drop 保证所有资源释放，即使之前的步骤 panic
pub struct JailContext {
    name: String,
    root_path: PathBuf,
    work_dir: PathBuf,
    devfs_ruleset: u32,
    /// 已成功挂载的路径，Drop 时逆序卸载
    mounts: Vec<PathBuf>,
    network_enabled: bool,
    /// jail 是否已启动（未启动则 Drop 时跳过 jail -r）
    jail_started: bool,
}

impl JailContext {
    fn new(name: String, work_dir: PathBuf) -> Result<Self> {
        // UUID 保证规则集编号不碰撞（范围 1000..65535）
        let ruleset = (Uuid::new_v4().as_u128() % 64535 + 1000) as u32;
        let root_path = PathBuf::from("/var/gitfox/jails").join(&name);

        Ok(Self {
            name,
            root_path,
            work_dir,
            devfs_ruleset: ruleset,
            mounts: Vec::new(),
            network_enabled: true,
            jail_started: false,
        })
    }

    fn setup(&mut self) -> Result<()> {
        // 1. 创建目录结构
        std::fs::create_dir_all(&self.root_path)?;
        for dir in &["bin", "usr/bin", "usr/lib", "lib", "dev", "tmp"] {
            std::fs::create_dir_all(self.root_path.join(dir))?;
        }
        // sticky tmp
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                self.root_path.join("tmp"),
                std::fs::Permissions::from_mode(0o1777),
            )?;
        }

        // 2. 只读挂载系统目录
        for dir in &["/bin", "/usr/bin", "/usr/lib", "/lib"] {
            if Path::new(dir).exists() {
                let rel = &dir[1..]; // 去掉前导 /
                std::fs::create_dir_all(self.root_path.join(rel))?;
                self.mount_nullfs(Path::new(dir), rel, true)?;
            }
        }
        for dir in &["/libexec", "/usr/libexec"] {
            if Path::new(dir).exists() {
                let rel = &dir[1..];
                std::fs::create_dir_all(self.root_path.join(rel))?;
                self.mount_nullfs(Path::new(dir), rel, true)?;
            }
        }

        // 3. 读写挂载工作目录（构建产物需要写入）
        let work_rel = self.work_dir.strip_prefix("/")
            .unwrap_or(&self.work_dir)
            .to_owned();
        std::fs::create_dir_all(self.root_path.join(&work_rel))?;
        let work_dir = self.work_dir.clone();
        self.mount_nullfs(&work_dir, &work_rel, false)?;

        // 4. devfs 规则集（只暴露安全设备）
        self.setup_devfs()?;

        // 5. 启动 jail（最后一步，之前任何失败都由 Drop 清理已挂载的内容）
        self.start_jail()?;
        self.jail_started = true;

        Ok(())
    }

    fn mount_nullfs(&mut self, source: &Path, rel_target: impl AsRef<Path>, ro: bool) -> Result<()> {
        let target = self.root_path.join(rel_target);
        let opts = if ro { "ro" } else { "rw" };

        let status = privileged("mount")
            .args(["-t", "nullfs", "-o", opts])
            .arg(source)
            .arg(&target)
            .status()?;

        if !status.success() {
            return Err(RunnerError::Execution(format!(
                "nullfs mount failed: {} -> {}",
                source.display(), target.display()
            )));
        }

        // 只有挂载成功才记录，Drop 时才会卸载
        self.mounts.push(target);
        Ok(())
    }

    fn setup_devfs(&self) -> Result<()> {
        let s = self.devfs_ruleset.to_string();

        // 激活规则集
        run_checked(privileged("devfs").args(["ruleset", &s]))?;
        // 默认全部隐藏
        run_checked(privileged("devfs").args(["rule", "-s", &s, "add", "hide"]))?;
        // 只暴露安全设备
        for dev in &["null", "zero", "random", "urandom", "fd", "fd/*",
                     "stdin", "stdout", "stderr"] {
            run_checked(privileged("devfs").args(["rule", "-s", &s, "add", "path", dev, "unhide"]))?;
        }
        Ok(())
    }

    fn start_jail(&self) -> Result<()> {
        let name_arg   = format!("name={}", self.name);
        let path_arg   = format!("path={}", self.root_path.display());
        let host_arg   = format!("host.hostname={}", self.name);
        let rules_arg  = format!("devfs_ruleset={}", self.devfs_ruleset);
        let ip4_arg    = format!("ip4={}", if self.network_enabled { "inherit" } else { "disable" });

        run_checked(privileged("jail").args([
            "-c",
            &name_arg, &path_arg, &host_arg,
            "mount.devfs",
            &rules_arg,
            "securelevel=3",
            "children.max=0",
            "enforce_statfs=2",
            "allow.mount=false",
            "allow.set_hostname=false",
            "allow.sysvipc=false",
            "allow.raw_sockets=false",
            "allow.chflags=false",
            "allow.mlock=false",
            "allow.mount.devfs=false",
            "allow.mount.nullfs=false",
            "allow.mount.procfs=false",
            "allow.mount.tmpfs=false",
            "allow.reserved_ports=false",
            &ip4_arg,
            "exec.clean=true",
            "exec.timeout=7200",
            "persist",
        ]))
    }

    /// 构造在 jail 内执行的命令
    /// 使用 jexec + /usr/bin/env -C <dir>，完全无 Shell，所有参数为 OsStr
    fn make_exec_command(
        &self,
        program: &std::ffi::OsStr,
        args: &[std::ffi::OsString],
        envs: &[(std::ffi::OsString, std::ffi::OsString)],
    ) -> Command {
        // jexec <jail_name> /usr/bin/env -i -C <work_dir> [KEY=VAL...] <program> [args...]
        //   -i : 清空环境（不继承宿主环境）
        //   -C : chdir（FreeBSD env 原生支持，无需 shell）
        let work_dir_str = self.work_dir.as_os_str().to_owned();

        let mut cmd = privileged("jexec");
        cmd.args([&self.name, "/usr/bin/env", "-i", "-C"])
           .arg(&work_dir_str);

        // 显式传入作业所需的环境变量（KEY=VALUE 形式）
        for (k, v) in envs {
            let mut kv = k.clone();
            kv.push("=");
            kv.push(v);
            cmd.arg(kv);
        }

        cmd.arg(program).args(args);
        cmd
    }
}

impl Drop for JailContext {
    fn drop(&mut self) {
        log::info!("[jail cleanup] {}", self.name);

        // 1. 停止 jail（如果已启动）
        if self.jail_started {
            let _ = privileged("jail").args(["-r", &self.name]).status();
        }

        // 2. 逆序卸载挂载点（与挂载顺序相反）
        for mp in self.mounts.iter().rev() {
            let _ = privileged("umount").arg("-f").arg(mp).status();
        }

        // 3. 删除 devfs 规则集
        let s = self.devfs_ruleset.to_string();
        let _ = privileged("devfs").args(["rule", "-s", &s, "delset"]).status();

        // 4. 删除 jail 根目录
        let _ = std::fs::remove_dir_all(&self.root_path);

        log::info!("[jail cleanup] done: {}", self.name);
    }
}

/// 检查命令执行结果，非零退出码返回 Err
fn run_checked(cmd: &mut Command) -> Result<()> {
    let status = cmd.status()?;
    if !status.success() {
        let prog = cmd.get_program().to_string_lossy().to_string();
        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy().to_string()).collect();
        return Err(RunnerError::Execution(format!(
            "command failed: {} {}",
            prog, args.join(" ")
        )));
    }
    Ok(())
}

/// 如果当前进程是 root 则直接执行，否则通过 sudo 提权
/// program 参数是要执行的实际程序名（如 "mount", "jail", "jexec"...）
fn privileged(program: &str) -> Command {
    // SAFETY: getuid() 是纯读取 syscall，无副作用
    let uid = unsafe { libc::getuid() };
    if uid == 0 {
        // 已是 root，直接执行
        Command::new(program)
    } else {
        // 通过 sudo 提权，程序名作为第一个参数
        let mut cmd = Command::new("sudo");
        cmd.arg(program);
        cmd
    }
}