//! Keeps `adb reverse` tunnels alive so a USB cable can be unplugged and replugged freely.
//! Every few seconds each attached device is checked and missing tunnels are re-created.

use std::collections::HashSet;
use std::process::{Command, Stdio};
use std::time::Duration;

fn works(adb: &str) -> bool {
    Command::new(adb)
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn find_adb() -> Option<String> {
    let mut candidates = vec!["adb".to_string()];
    if let Ok(root) = std::env::var("ANDROID_HOME").or_else(|_| std::env::var("ANDROID_SDK_ROOT")) {
        candidates.push(format!("{root}/platform-tools/adb"));
    }
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(format!("{home}/Library/Android/sdk/platform-tools/adb"));
    }
    candidates.into_iter().find(|c| works(c))
}

fn run(adb: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(adb).args(args).stderr(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn devices(adb: &str) -> Vec<String> {
    run(adb, &["devices"])
        .unwrap_or_default()
        .lines()
        .skip(1)
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (serial, state) = (it.next()?, it.next()?);
            (state == "device").then(|| serial.to_string())
        })
        .collect()
}

pub fn spawn(ports: Vec<u16>) {
    std::thread::spawn(move || {
        let Some(adb) = find_adb() else {
            println!("adb not found: for USB run `adb reverse tcp:PORT tcp:PORT` yourself (or use --no-adb)");
            return;
        };
        let mut announced: HashSet<String> = HashSet::new();
        loop {
            let serials = devices(&adb);
            announced.retain(|s| serials.contains(s));
            for serial in &serials {
                let list = run(&adb, &["-s", serial, "reverse", "--list"]).unwrap_or_default();
                let missing: Vec<u16> =
                    ports.iter().copied().filter(|p| !list.contains(&format!("tcp:{p} "))).collect();
                if missing.is_empty() {
                    continue;
                }
                let ok = missing.iter().fold(true, |ok, p| {
                    let spec = format!("tcp:{p}");
                    ok & run(&adb, &["-s", serial, "reverse", &spec, &spec]).is_some()
                });
                if ok {
                    println!("adb: USB tunnel ready for {serial}");
                    announced.insert(serial.clone());
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}
