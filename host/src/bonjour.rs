//! Announces the host on the local network (`_ligarview._tcp`) so tablets find it without typing
//! an address. Uses the system `dns-sd` tool; a small watcher shell stops it when the host exits.

use std::process::{Command, Stdio};

fn computer_name() -> String {
    Command::new("scutil")
        .args(["--get", "ComputerName"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Mac".into())
}

pub fn start(port: u16) {
    let script = "dns-sd -R \"$LGRV_NAME\" _ligarview._tcp . \"$LGRV_PORT\" >/dev/null 2>&1 & pid=$!; \
                  while kill -0 $PPID 2>/dev/null; do sleep 2; done; kill $pid";
    let r = Command::new("sh")
        .args(["-c", script])
        .env("LGRV_NAME", computer_name())
        .env("LGRV_PORT", port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match r {
        Ok(_) => println!("announcing _ligarview._tcp on port {port}"),
        Err(e) => println!("bonjour not started: {e}"),
    }
}
