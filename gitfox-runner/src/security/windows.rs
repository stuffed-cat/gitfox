/// Windows 动态用户隔离 - 纯 Rust Win32 extern，RAII 保证清理
///
/// 设计原则：
/// - 零 PowerShell 脚本：所有操作通过 Win32 extern "system" FFI 调用
/// - 零磁盘文件：无临时 .ps1 文件，无 race condition 窗口期
/// - RAII 守卫：WindowsUserGuard 的 Drop 无论 panic/OOM 都会执行
/// - 零密码明文传递：密码通过进程内 Mutex<HashMap> 传递，
///   取走后立即 write_volatile 清零，不经磁盘/环境变量
/// - LogonUserW + CreateProcessAsUserW 直接以隔离用户启动子进程
/// - 零外部依赖：仅 extern "system" 链接 advapi32/netapi32，
///   不依赖 windows-sys crate，避免 feature 版本问题
use crate::error::{Result, RunnerError};
use crate::security::SecurityContext;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(target_os = "windows")]
use uuid::Uuid;

// ============================================================
// Win32 类型别名 + extern 声明（仅 Windows 下编译）
// ============================================================
#[cfg(target_os = "windows")]
mod ffi {
    use std::ffi::c_void;

    pub type HANDLE = *mut c_void;
    pub type BOOL = i32;
    pub type DWORD = u32;
    pub type LPDWORD = *mut DWORD;
    pub type LPVOID = *mut c_void;
    pub type LPCWSTR = *const u16;
    pub type LPWSTR = *mut u16;
    pub type NTSTATUS = i32;

    pub const FALSE: BOOL = 0;
    pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

    // LogonUser dwLogonType
    pub const LOGON32_LOGON_INTERACTIVE: DWORD = 2;
    pub const LOGON32_PROVIDER_DEFAULT: DWORD = 0;

    // CreateProcess dwCreationFlags
    pub const CREATE_NO_WINDOW: DWORD = 0x08000000;
    pub const CREATE_UNICODE_ENVIRONMENT: DWORD = 0x00000400;

    // NetUser flags
    pub const USER_PRIV_USER: DWORD = 1;
    pub const UF_DONT_EXPIRE_PASSWD: DWORD = 0x10000;
    pub const UF_PASSWD_CANT_CHANGE: DWORD = 0x0040;
    // NERR_UserNotFound
    pub const NERR_USER_NOT_FOUND: u32 = 2221;

    // ACL flags
    pub const DACL_SECURITY_INFORMATION: DWORD = 0x4;
    pub const PROTECTED_DACL_SECURITY_INFORMATION: DWORD = 0x80000000;
    pub const UNPROTECTED_DACL_SECURITY_INFORMATION: DWORD = 0x20000000;
    pub const SE_FILE_OBJECT: DWORD = 1;
    pub const GRANT_ACCESS: DWORD = 1;
    pub const OBJECT_INHERIT_ACE: DWORD = 0x1;
    pub const CONTAINER_INHERIT_ACE: DWORD = 0x2;
    pub const FILE_ALL_ACCESS: DWORD = 0x001F01FF;

    // BCryptGenRandom flags
    pub const BCRYPT_USE_SYSTEM_PREFERRED_RNG: DWORD = 0x00000002;

    #[repr(C)]
    pub struct STARTUPINFOW {
        pub cb: DWORD,
        pub lp_reserved: LPWSTR,
        pub lp_desktop: LPWSTR,
        pub lp_title: LPWSTR,
        pub dw_x: DWORD,
        pub dw_y: DWORD,
        pub dw_x_size: DWORD,
        pub dw_y_size: DWORD,
        pub dw_x_count_chars: DWORD,
        pub dw_y_count_chars: DWORD,
        pub dw_fill_attribute: DWORD,
        pub dw_flags: DWORD,
        pub w_show_window: u16,
        pub cb_reserved2: u16,
        pub lp_reserved2: *mut u8,
        pub h_std_input: HANDLE,
        pub h_std_output: HANDLE,
        pub h_std_error: HANDLE,
    }

    #[repr(C)]
    pub struct PROCESS_INFORMATION {
        pub h_process: HANDLE,
        pub h_thread: HANDLE,
        pub dw_process_id: DWORD,
        pub dw_thread_id: DWORD,
    }

    /// USER_INFO_1 for NetUserAdd level=1
    #[repr(C)]
    pub struct USER_INFO_1 {
        pub usri1_name: LPWSTR,
        pub usri1_password: LPWSTR,
        pub usri1_password_age: DWORD,
        pub usri1_priv: DWORD,
        pub usri1_home_dir: LPWSTR,
        pub usri1_comment: LPWSTR,
        pub usri1_flags: DWORD,
        pub usri1_script_path: LPWSTR,
    }

    /// EXPLICIT_ACCESS_W for SetEntriesInAclW
    /// Trustee 内嵌（只用 TRUSTEE_W::ptstrName，其余清零）
    #[repr(C)]
    pub struct TRUSTEE_W {
        pub p_multiple_trustee: *mut TRUSTEE_W,
        pub multiple_trustee_operation: i32,
        pub trustee_form: i32,   // TRUSTEE_IS_NAME = 1
        pub trustee_type: i32,   // TRUSTEE_IS_UNKNOWN = 0
        pub ptstr_name: LPWSTR,
    }

    #[repr(C)]
    pub struct EXPLICIT_ACCESS_W {
        pub grf_access_permissions: DWORD,
        pub grf_access_mode: DWORD,
        pub grf_inheritance: DWORD,
        pub trustee: TRUSTEE_W,
    }

    #[link(name = "advapi32")]
    extern "system" {
        pub fn LogonUserW(
            lp_username: LPCWSTR,
            lp_domain: LPCWSTR,
            lp_password: LPCWSTR,
            dw_logon_type: DWORD,
            dw_logon_provider: DWORD,
            ph_token: *mut HANDLE,
        ) -> BOOL;

        pub fn CreateProcessAsUserW(
            h_token: HANDLE,
            lp_application_name: LPCWSTR,
            lp_command_line: LPWSTR,
            lp_process_attributes: *mut c_void,
            lp_thread_attributes: *mut c_void,
            b_inherit_handles: BOOL,
            dw_creation_flags: DWORD,
            lp_environment: LPVOID,
            lp_current_directory: LPCWSTR,
            lp_startup_info: *mut STARTUPINFOW,
            lp_process_information: *mut PROCESS_INFORMATION,
        ) -> BOOL;

        pub fn CloseHandle(h_object: HANDLE) -> BOOL;

        pub fn GetLastError() -> DWORD;

        pub fn SetNamedSecurityInfoW(
            p_object_name: LPWSTR,
            object_type: DWORD,
            security_info: DWORD,
            psid_owner: *mut c_void,
            psid_group: *mut c_void,
            p_dacl: *mut c_void,
            p_sacl: *mut c_void,
        ) -> DWORD;

        pub fn SetEntriesInAclW(
            c_count_of_explicit_entries: DWORD,
            p_list_of_explicit_entries: *mut EXPLICIT_ACCESS_W,
            old_acl: *mut c_void,
            new_acl: *mut *mut c_void,
        ) -> DWORD;

        pub fn LocalFree(h_mem: *mut c_void) -> *mut c_void;

        pub fn WaitForSingleObject(h_handle: HANDLE, dw_milliseconds: DWORD) -> DWORD;

        pub fn GetExitCodeProcess(h_process: HANDLE, lp_exit_code: *mut DWORD) -> BOOL;
    }

    #[link(name = "netapi32")]
    extern "system" {
        pub fn NetUserAdd(
            server_name: LPCWSTR,
            level: DWORD,
            buf: *const u8,
            parm_err: *mut DWORD,
        ) -> u32;

        pub fn NetUserDel(server_name: LPCWSTR, username: LPCWSTR) -> u32;
    }

    #[link(name = "bcrypt")]
    extern "system" {
        pub fn BCryptGenRandom(
            h_algorithm: *mut c_void,
            pb_buffer: *mut u8,
            cb_buffer: DWORD,
            dw_flags: DWORD,
        ) -> NTSTATUS;
    }

    pub fn to_wide(s: &str) -> Vec<u16> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0u16))
            .collect()
    }
}

// ============================================================
// Win32 操作封装（仅 Windows 下编译）
// ============================================================
#[cfg(target_os = "windows")]
mod win {
    use super::ffi::*;
    use super::*;
    use std::ptr;

    /// BCryptGenRandom → 映射到可打印 ASCII（32 字节随机 → 32 字节密码）
    pub fn generate_password() -> Result<[u8; 64]> {
        let mut raw = [0u8; 32];
        let status = unsafe {
            BCryptGenRandom(ptr::null_mut(), raw.as_mut_ptr(), 32, BCRYPT_USE_SYSTEM_PREFERRED_RNG)
        };
        if status != 0 {
            return Err(RunnerError::Execution(format!("BCryptGenRandom: 0x{:X}", status)));
        }
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz\
                                  ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                  0123456789!@#$";
        let mut out = [0u8; 64];
        for (i, &b) in raw.iter().enumerate() {
            out[i] = CHARSET[(b as usize) % CHARSET.len()];
        }
        Ok(out)
    }

    /// NetUserAdd，无 net.exe，直接 Win32 API
    pub fn net_user_add(username: &str, password: &[u8; 64]) -> Result<()> {
        let name_w = to_wide(username);
        let pw_str = std::str::from_utf8(password)
            .map_err(|_| RunnerError::Execution("password is not UTF-8".into()))?;
        let pw_w = to_wide(pw_str);

        let ui = USER_INFO_1 {
            usri1_name: name_w.as_ptr() as *mut u16,
            usri1_password: pw_w.as_ptr() as *mut u16,
            usri1_password_age: 0,
            usri1_priv: USER_PRIV_USER,
            usri1_home_dir: ptr::null_mut(),
            usri1_comment: ptr::null_mut(),
            usri1_flags: UF_DONT_EXPIRE_PASSWD | UF_PASSWD_CANT_CHANGE,
            usri1_script_path: ptr::null_mut(),
        };

        let err = unsafe {
            NetUserAdd(ptr::null(), 1, &ui as *const _ as *const u8, ptr::null_mut())
        };
        if err != 0 {
            return Err(RunnerError::Execution(format!("NetUserAdd: error {}", err)));
        }
        Ok(())
    }

    /// NetUserDel，幂等（2221 = 用户不存在视为成功）
    pub fn net_user_del(username: &str) -> Result<()> {
        let name_w = to_wide(username);
        let err = unsafe { NetUserDel(ptr::null(), name_w.as_ptr()) };
        if err != 0 && err != NERR_USER_NOT_FOUND {
            return Err(RunnerError::Execution(format!("NetUserDel: error {}", err)));
        }
        Ok(())
    }

    /// SetNamedSecurityInfo 设置独占 DACL（移除继承权，仅指定用户有权）
    pub fn set_exclusive_acl(dir: &Path, username: &str) -> Result<()> {
        let name_w = to_wide(username);
        let dir_w = to_wide(&dir.to_string_lossy());

        let mut trustee = TRUSTEE_W {
            p_multiple_trustee: ptr::null_mut(),
            multiple_trustee_operation: 0,
            trustee_form: 1, // TRUSTEE_IS_NAME
            trustee_type: 0, // TRUSTEE_IS_UNKNOWN
            ptstr_name: name_w.as_ptr() as *mut u16,
        };

        let mut access = EXPLICIT_ACCESS_W {
            grf_access_permissions: FILE_ALL_ACCESS,
            grf_access_mode: GRANT_ACCESS,
            grf_inheritance: OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE,
            trustee,
        };

        let mut new_dacl: *mut std::ffi::c_void = ptr::null_mut();
        let err = unsafe {
            SetEntriesInAclW(1, &mut access as *mut _, ptr::null_mut(), &mut new_dacl)
        };
        if err != 0 {
            return Err(RunnerError::Execution(format!("SetEntriesInAclW: {}", err)));
        }

        let flags = DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION;
        let err = unsafe {
            SetNamedSecurityInfoW(
                dir_w.as_ptr() as *mut u16,
                SE_FILE_OBJECT,
                flags,
                ptr::null_mut(),
                ptr::null_mut(),
                new_dacl,
                ptr::null_mut(),
            )
        };
        unsafe { LocalFree(new_dacl) };
        if err != 0 {
            return Err(RunnerError::Execution(format!("SetNamedSecurityInfo: {}", err)));
        }
        Ok(())
    }

    /// 还原 DACL 为允许继承（Drop 时调用）
    pub fn restore_acl_inheritance(dir: &Path) {
        let dir_w = to_wide(&dir.to_string_lossy());
        let flags = DACL_SECURITY_INFORMATION | UNPROTECTED_DACL_SECURITY_INFORMATION;
        unsafe {
            SetNamedSecurityInfoW(
                dir_w.as_ptr() as *mut u16,
                SE_FILE_OBJECT,
                flags,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
        }
    }

    /// 按 CommandLineToArgvW 规则加引号
    fn quote_arg(s: &str) -> String {
        if s.is_empty() { return "\"\"".into(); }
        if !s.contains([' ', '\t', '"']) { return s.to_string(); }
        let mut out = String::with_capacity(s.len() + 2);
        out.push('"');
        let mut bs = 0usize;
        for c in s.chars() {
            match c {
                '\\' => bs += 1,
                '"' => {
                    for _ in 0..bs { out.push('\\'); }
                    out.push_str("\\\"");
                    bs = 0;
                }
                _ => { for _ in 0..bs { out.push('\\'); } out.push(c); bs = 0; }
            }
        }
        for _ in 0..bs { out.push('\\'); }
        out.push('"');
        out
    }

    /// 构建 UTF-16 环境块（KEY=VAL\0...\0\0）
    fn build_env_block(vars: &[(String, String)]) -> Vec<u16> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        let mut block = Vec::<u16>::new();
        for (k, v) in vars {
            let entry = format!("{}={}", k, v);
            block.extend(OsStr::new(&entry).encode_wide());
            block.push(0);
        }
        block.push(0); // double-NUL
        block
    }

    /// LogonUserW + CreateProcessAsUserW，返回 (hProcess, hThread)
    pub fn create_process_as_user(
        username: &str,
        password: &[u8; 64],
        exe: &str,
        args: &[String],
        work_dir: &Path,
        env_vars: &[(String, String)],
    ) -> Result<(HANDLE, HANDLE)> {
        let name_w = to_wide(username);
        let domain_w = to_wide(".");  // "." = 本机
        let pw_str = std::str::from_utf8(password)
            .map_err(|_| RunnerError::Execution("password not UTF-8".into()))?;
        let pw_w = to_wide(pw_str);

        let mut token: HANDLE = ptr::null_mut();
        let ok = unsafe {
            LogonUserW(
                name_w.as_ptr(),
                domain_w.as_ptr(),
                pw_w.as_ptr(),
                LOGON32_LOGON_INTERACTIVE,
                LOGON32_PROVIDER_DEFAULT,
                &mut token,
            )
        };
        if ok == FALSE {
            let e = unsafe { GetLastError() };
            return Err(RunnerError::Execution(format!("LogonUserW failed: {}", e)));
        }

        // 构建命令行
        let mut parts = vec![quote_arg(exe)];
        parts.extend(args.iter().map(|a| quote_arg(a)));
        let cmdline = parts.join(" ");
        let mut cmdline_w = to_wide(&cmdline);

        let env_block = build_env_block(env_vars);
        let work_w = to_wide(&work_dir.to_string_lossy());

        let mut si = STARTUPINFOW {
            cb: std::mem::size_of::<STARTUPINFOW>() as DWORD,
            lp_reserved: ptr::null_mut(),
            lp_desktop: ptr::null_mut(),
            lp_title: ptr::null_mut(),
            dw_x: 0, dw_y: 0,
            dw_x_size: 0, dw_y_size: 0,
            dw_x_count_chars: 0, dw_y_count_chars: 0,
            dw_fill_attribute: 0, dw_flags: 0,
            w_show_window: 0, cb_reserved2: 0,
            lp_reserved2: ptr::null_mut(),
            h_std_input: ptr::null_mut(),
            h_std_output: ptr::null_mut(),
            h_std_error: ptr::null_mut(),
        };
        let mut pi = PROCESS_INFORMATION {
            h_process: ptr::null_mut(),
            h_thread: ptr::null_mut(),
            dw_process_id: 0,
            dw_thread_id: 0,
        };

        let ok = unsafe {
            CreateProcessAsUserW(
                token,
                ptr::null(),
                cmdline_w.as_mut_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
                FALSE,
                CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT,
                env_block.as_ptr() as *mut _,
                work_w.as_ptr(),
                &mut si,
                &mut pi,
            )
        };
        unsafe { CloseHandle(token) };

        if ok == FALSE {
            let e = unsafe { GetLastError() };
            return Err(RunnerError::Execution(format!("CreateProcessAsUserW failed: {}", e)));
        }
        Ok((pi.h_process, pi.h_thread))
    }
}

// ============================================================
// 密码进程内传递（不经环境变量/磁盘）
// ============================================================
#[cfg(target_os = "windows")]
static PASSWORD_STORE: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, [u8; 64]>>,
> = std::sync::OnceLock::new();

#[cfg(target_os = "windows")]
fn store_password(username: &str, pw: [u8; 64]) {
    PASSWORD_STORE
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
        .lock()
        .unwrap()
        .insert(username.to_string(), pw);
}

/// executor.rs 调用：取走后立即在调用端清零
#[cfg(target_os = "windows")]
pub fn take_password(username: &str) -> Option<[u8; 64]> {
    PASSWORD_STORE.get()?.lock().ok()?.remove(username)
}

/// write_volatile 清零，防止编译器优化掉（跨平台支持 Drop）
#[allow(dead_code)]
pub fn secure_zero(buf: &mut [u8]) {
    for b in buf.iter_mut() {
        unsafe { std::ptr::write_volatile(b, 0u8) };
    }
}

// ============================================================
// RAII 守卫
// ============================================================

/// Windows 隔离用户守卫
/// Drop 时：还原工作目录 ACL → NetUserDel 删除临时用户
#[allow(dead_code)]
pub struct WindowsUserGuard {
    pub username: String,
    pub work_dir: PathBuf,
}

impl Drop for WindowsUserGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        {
            win::restore_acl_inheritance(&self.work_dir);
            if let Err(e) = win::net_user_del(&self.username) {
                log::warn!("[win-guard] drop: failed to delete user {}: {}", self.username, e);
            } else {
                log::info!("[win-guard] drop: deleted isolated user {}", self.username);
            }
        }
    }
}

impl WindowsUserGuard {
    /// 以隔离用户身份执行命令，等待退出，返回退出码
    /// executor.rs 在 Windows 下应调用此方法替代 cmd.spawn() + child.wait()
    #[cfg(target_os = "windows")]
    pub fn spawn_isolated(
        &self,
        exe: &str,
        args: &[String],
        env_vars: &[(String, String)],
    ) -> Result<i32> {
        let mut password = take_password(&self.username)
            .ok_or_else(|| RunnerError::Execution("password not found in store".into()))?;

        let result = win::create_process_as_user(
            &self.username,
            &password,
            exe,
            args,
            &self.work_dir,
            env_vars,
        );
        // 无论成功与否立即清零
        secure_zero(&mut password);

        let (hproc, hthread) = result?;

        const INFINITE: u32 = 0xFFFFFFFF;
        unsafe { ffi::WaitForSingleObject(hproc, INFINITE) };
        let mut code: u32 = 1;
        unsafe { ffi::GetExitCodeProcess(hproc, &mut code) };
        unsafe { ffi::CloseHandle(hproc) };
        unsafe { ffi::CloseHandle(hthread) };

        Ok(code as i32)
    }
}

// ============================================================
// 公共入口
// ============================================================

/// 应用 Windows 隔离（返回 RAII 守卫）：
/// 1. BCryptGenRandom 生成随机密码（零明文接触）
/// 2. NetUserAdd 创建本地用户（直接 Win32 API，无 net.exe）
/// 3. SetNamedSecurityInfo 设置工作目录独占 DACL
/// 4. 密码存入进程内 PASSWORD_STORE（Mutex<HashMap>，不经磁盘/env）
/// 5. 返回 WindowsUserGuard（Drop = restore_acl + NetUserDel）
///
/// executor.rs 在 Windows 下应通过 guard.spawn_isolated() 执行命令，
/// 而非直接 cmd.spawn()（cmd 被替换为无害占位符）
#[allow(dead_code)]
#[cfg_attr(not(target_os = "windows"), allow(unused_variables, unreachable_code))]
pub fn apply_isolation(ctx: &SecurityContext, cmd: &mut Command) -> Result<WindowsUserGuard> {
    let work_path = Path::new(&ctx.work_dir);
    if !work_path.is_absolute() {
        return Err(RunnerError::Execution("work_dir must be absolute".into()));
    }
    let work_dir = work_path
        .canonicalize()
        .map_err(|e| RunnerError::Execution(format!("canonicalize work_dir: {}", e)))?;

    #[cfg(target_os = "windows")]
    {
        // UUID 后 16 hex 字符作后缀；Windows 用户名上限 20 字符
        let suffix = Uuid::new_v4().as_simple().to_string();
        let username = format!("Gfx{}", &suffix[..16]);

        log::info!("[win] creating isolated user {}", username);

        let mut password = win::generate_password()?;
        win::net_user_add(&username, &password)?;
        win::set_exclusive_acl(&work_dir, &username)?;

        // 密码存入进程内存，不经环境变量/磁盘
        store_password(&username, password);
        // 清零栈上副本
        secure_zero(&mut password);

        // 替换 cmd 为无害占位符（executor 走 spawn_isolated 路径）
        *cmd = Command::new("cmd.exe");
        cmd.args(["/c", "exit", "0"]);

        log::info!("✓ Windows isolation: user={}", username);
        return Ok(WindowsUserGuard { username, work_dir });
    }

    #[cfg(not(target_os = "windows"))]
    Err(RunnerError::Execution(
        "windows::apply_isolation called on non-Windows platform".into(),
    ))
}
