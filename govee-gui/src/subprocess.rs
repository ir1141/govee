use std::process::{Child, Command};

pub fn spawn_govee(args: &[&str], device_ip: Option<&str>) -> std::io::Result<Child> {
    let mut cmd = Command::new("govee");
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
