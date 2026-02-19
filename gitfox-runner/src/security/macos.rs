use crate::error::{Result, RunnerError};
use crate::security::SecurityContext;
use std::process::Command;

/// macOS isolation: 动态创建用户（dscl + sudo，系统自带）
pub fn apply_isolation(ctx: &SecurityContext, cmd: &mut Command) -> Result<()> {
    let original_program = cmd.get_program().to_string_lossy().to_string();
    let original_args: Vec<String> = cmd
        .get_args()
        .map(|s| s.to_string_lossy().to_string())
        .collect();

    let username = format!("_gitfoxjob{}", std::process::id());
    let work_dir = &ctx.work_dir;

    let script = format!(
        r#"#!/bin/sh
set -e

USERNAME='{}'
WORK_DIR='{}'

echo "Creating isolated user..."
sudo dscl . -create /Users/$USERNAME
sudo dscl . -create /Users/$USERNAME UserShell /usr/bin/false
sudo dscl . -create /Users/$USERNAME RealName "GitFox Job"
sudo dscl . -create /Users/$USERNAME UniqueID $(($RANDOM + 20000))
sudo dscl . -create /Users/$USERNAME PrimaryGroupID 20
sudo dscl . -create /Users/$USERNAME NFSHomeDirectory /var/empty

sudo chmod 700 "$WORK_DIR"
sudo chown $USERNAME "$WORK_DIR"

echo "✓ Isolated user created"

sudo -u $USERNAME sh -c "cd '$WORK_DIR' && {} {}"
EXIT_CODE=$?

echo "Cleaning up..."
sudo dscl . -delete /Users/$USERNAME 2>/dev/null || true

exit $EXIT_CODE
"#,
        username,
        work_dir,
        shell_escape(&original_program),
        original_args.iter().map(|a| shell_escape(a)).collect::<Vec<_>>().join(" ")
    );

    let script_path = format!("/tmp/gitfox-iso-{}.sh", std::process::id());
    std::fs::write(&script_path, script)?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o700))?;
    }

    *cmd = Command::new("/bin/sh");
    cmd.arg(&script_path);

    log::info!("✓ macOS isolation: Dynamic user with dscl");
    log::warn!("⚠️  Requires sudo privileges");
    Ok(())
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}
