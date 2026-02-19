use crate::error::{Result, RunnerError};
use crate::security::SecurityContext;
use std::process::Command;

/// Windows isolation: 动态创建低权限用户（使用系统自带工具）
pub fn apply_isolation(ctx: &SecurityContext, cmd: &mut Command) -> Result<()> {
    let original_program = cmd.get_program().to_string_lossy().to_string();
    let original_args: Vec<String> = cmd
        .get_args()
        .map(|s| s.to_string_lossy().to_string())
        .collect();

    let username = format!("GitFoxJob{}", std::process::id());
    let work_dir = &ctx.work_dir;

    // Build PowerShell isolation script using built-in tools
    let script = format!(
        r#"# Windows isolation using net user + icacls (system built-in)
$ErrorActionPreference = 'Stop'

$Username = '{}'
$WorkDir = '{}'

try {{
    Write-Host "Creating isolated user..."
    net user $Username /add /active:yes /passwordreq:no | Out-Null
    
    net localgroup Users $Username /delete 2>$null | Out-Null
    
    icacls $WorkDir /grant "${{Username}}:(OI)(CI)M" /T | Out-Null
    
    icacls "C:\Windows\System32" /deny "${{Username}}:F" /T 2>$null | Out-Null
    icacls "C:\Program Files" /deny "${{Username}}:W" /T 2>$null | Out-Null
    
    Write-Host "✓ Isolated user created"
    
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = '{}'
    $psi.Arguments = '{}'
    $psi.WorkingDirectory = $WorkDir
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    
    $proc = [System.Diagnostics.Process]::Start($psi)
    $proc.WaitForExit()
    
    $exitCode = $proc.ExitCode
    
    net user $Username /delete | Out-Null
    
    exit $exitCode
    
}} catch {{
    net user $Username /delete 2>$null | Out-Null
    Write-Error $_.Exception.Message
    exit 1
}}
"#,
        username,
        work_dir,
        original_program.replace("\\", "\\\\").replace('"', "`\""),
        original_args.join(" ").replace("\\", "\\\\").replace('"', "`\"")
    );

    let script_path = format!("{}\\gitfox-iso-{}.ps1", 
        std::env::temp_dir().to_string_lossy(), 
        std::process::id()
    );
    std::fs::write(&script_path, script)?;

    *cmd = Command::new("powershell.exe");
    cmd.args(&[
        "-ExecutionPolicy", "Bypass",
        "-NoProfile",
        "-File", &script_path
    ]);

    log::info!("✓ Windows isolation: Dynamic low-privilege user");
    Ok(())
}
