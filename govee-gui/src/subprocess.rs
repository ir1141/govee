//! Subprocess management for spawning and killing `govee` CLI processes.
//!
//! The GUI delegates continuous modes (themes, screen, audio, ambient) to CLI
//! subprocesses and sends SIGTERM for clean shutdown.

use std::process::{Child, Command};

/// Locate the `govee` CLI binary: prefer a sibling to the running executable,
/// fall back to PATH.
fn govee_binary() -> std::path::PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.parent().unwrap_or(exe.as_path()).join("govee");
        if sibling.exists() {
            return sibling;
        }
    }
    // Fall back to PATH
    "govee".into()
}

/// Spawn the `govee` CLI with the given arguments and optional device IP.
pub fn spawn_govee(args: &[&str], device_ip: Option<&str>) -> std::io::Result<Child> {
    let mut cmd = Command::new(govee_binary());
    if let Some(ip) = device_ip {
        cmd.arg("--ip").arg(ip);
    }
    cmd.args(args);
    cmd.spawn()
}

/// Send SIGTERM to gracefully stop a subprocess.
pub fn kill(child: &mut Child) {
    #[cfg(unix)]
    {
        // SAFETY: `child.id()` is the OS PID of a process we spawned and own.
        // Sending SIGTERM to a live child is safe; if already exited, it's a no-op.
        unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM); }
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
    }
}
