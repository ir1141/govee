use std::process::{Child, Command};

fn govee_binary() -> std::path::PathBuf {
    // Look for `govee` next to the running executable (same target/debug or target/release dir)
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.parent().unwrap_or(exe.as_path()).join("govee");
        if sibling.exists() {
            return sibling;
        }
    }
    // Fall back to PATH
    "govee".into()
}

pub fn spawn_govee(args: &[&str], device_ip: Option<&str>) -> std::io::Result<Child> {
    let mut cmd = Command::new(govee_binary());
    if let Some(ip) = device_ip {
        cmd.arg("--ip").arg(ip);
    }
    cmd.args(args);
    cmd.spawn()
}

pub fn kill(child: &mut Child) {
    #[cfg(unix)]
    {
        unsafe { libc::kill(child.id() as i32, libc::SIGTERM); }
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
    }
}
