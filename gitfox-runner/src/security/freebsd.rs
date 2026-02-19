use crate::error::{Result, RunnerError};
use crate::security::SecurityContext;
use std::process::Command;

/// FreeBSD isolation: jail + nullfs（系统自带）
pub fn apply_isolation(ctx: &SecurityContext, cmd: &mut Command) -> Result<()> {
    let original_program = cmd.get_program().to_string_lossy().to_string();
    let original_args: Vec<String> = cmd
        .get_args()
        .map(|s| s.to_string_lossy().to_string())
        .collect();

    let jail_name = format!("gitfox_job_{}", std::process::id());
    let work_dir = &ctx.work_dir;

    let script = format!(
        r#"#!/bin/sh
set -e

JAIL_NAME='{}'
WORK_DIR='{}'
JAIL_PATH="/tmp/$JAIL_NAME"

echo "Creating jail..."
mkdir -p "$JAIL_PATH"
mkdir -p "$JAIL_PATH/bin" "$JAIL_PATH/usr/bin" "$JAIL_PATH/usr/lib"
mkdir -p "$JAIL_PATH/libexec" "$JAIL_PATH/lib"

mount -t nullfs -o ro /bin "$JAIL_PATH/bin"
mount -t nullfs -o ro /usr/bin "$JAIL_PATH/usr/bin"
mount -t nullfs -o ro /usr/lib "$JAIL_PATH/usr/lib"
mount -t nullfs -o ro /lib "$JAIL_PATH/lib"

mkdir -p "$JAIL_PATH$WORK_DIR"
mount -t nullfs "$WORK_DIR" "$JAIL_PATH$WORK_DIR"

echo "✓ Jail created"

jail -c \
    name="$JAIL_NAME" \
    path="$JAIL_PATH" \
    host.hostname="$JAIL_NAME" \
    ip4=inherit \
    mount.devfs \
    exec.start="/bin/sh -c 'cd $WORK_DIR && {} {}'"

EXIT_CODE=$?

echo "Cleaning up jail..."
jail -r "$JAIL_NAME" 2>/dev/null || true
umount "$JAIL_PATH$WORK_DIR" 2>/dev/null || true
umount "$JAIL_PATH/lib" 2>/dev/null || true
umount "$JAIL_PATH/usr/lib" 2>/dev/null || true
umount "$JAIL_PATH/usr/bin" 2>/dev/null || true
umount "$JAIL_PATH/bin" 2>/dev/null || true
rm -rf "$JAIL_PATH"

exit $EXIT_CODE
"#,
        jail_name,
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

    log::info!("✓ FreeBSD isolation: jail with nullfs");
    log::warn!("⚠️  Requires root privileges");
    Ok(())
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}
