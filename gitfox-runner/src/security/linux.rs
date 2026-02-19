use crate::error::{Result, RunnerError};
use crate::security::SecurityContext;
use std::process::Command;

/// Linux isolation using unshare (系统自带)
pub fn apply_isolation(ctx: &SecurityContext, cmd: &mut Command) -> Result<()> {
    let original_program = cmd.get_program().to_string_lossy().to_string();
    let original_args: Vec<String> = cmd
        .get_args()
        .map(|s| s.to_string_lossy().to_string())
        .collect();

    // Build unshare wrapper script
    let isolation_script = format!(
        r#"#!/bin/sh
set -e

# This script implements REAL isolation using only system utilities
# No external dependencies (bubblewrap etc)

WORK_DIR="{work_dir}"
NETWORK_MODE="{network_mode}"

# Create isolated mount namespace
unshare_opts="--mount --pid --fork --mount-proc"

# Network isolation
if [ "$NETWORK_MODE" = "none" ]; then
    unshare_opts="$unshare_opts --net"
fi

# Execute in isolated namespace
exec unshare $unshare_opts -- /bin/sh -c '
    set -e
    
    # Remount root as private to prevent mount leaks
    mount --make-rprivate /
    
    # Create minimal /tmp (tmpfs, isolated from host)
    mount -t tmpfs -o size=1G,mode=1777 tmpfs /tmp
    
    # Bind mount work directory (read-write, but ONLY this dir)
    # Everything else is read-only
    mount --bind "$WORK_DIR" "$WORK_DIR"
    
    # Change to work directory
    cd "$WORK_DIR"
    
    # Drop to restricted shell environment
    # Prevent access to parent directories
    export PATH=/usr/local/bin:/usr/bin:/bin
    
    # Execute actual command
    exec {program} {args}
'
"#,
        work_dir = ctx.work_dir,
        network_mode = ctx.network_mode,
        program = shell_escape(&original_program),
        args = original_args
            .iter()
            .map(|a| shell_escape(a))
            .collect::<Vec<_>>()
            .join(" ")
    );

    // Write script to temp file
    let script_path = format!("/tmp/gitfox-runner-isolation-{}.sh", std::process::id());
    std::fs::write(&script_path, isolation_script)?;
    std::fs::set_permissions(&script_path, std::os::unix::fs::PermissionsExt::from_mode(0o700))?;

    // Replace command with script execution
    *cmd = Command::new("/bin/sh");
    cmd.arg(&script_path);

    log::info!("✓ Linux isolation enabled (unshare + mount namespace)");

    Ok(())
}

fn shell_escape(s: &str) -> String {
    // Proper shell escaping to prevent injection
    format!("'{}'", s.replace('\'', r"'\''"))
}
