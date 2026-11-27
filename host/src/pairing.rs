//! Pairing from the Mac: tablets announce `_ligarview-tablet._tcp` and show a six-digit code. The
//! user picks the tablet in the window and types the code; the Mac sends it to the tablet, and when
//! the tablet accepts, the code becomes the secret the tablet presents when it connects here.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{mpsc, Mutex};
use std::time::Duration;

const TABLET_TYPE: &str = "_ligarview-tablet._tcp";

static TABLETS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static MESSAGE: Mutex<Option<String>> = Mutex::new(None);
static PORT: AtomicU16 = AtomicU16::new(7878);

pub fn init(port: u16) {
    PORT.store(port, Ordering::Relaxed);
}

pub fn tablets() -> Vec<String> {
    TABLETS.lock().unwrap().clone()
}

pub fn take_message() -> Option<String> {
    MESSAGE.lock().unwrap().take()
}

fn say(text: impl Into<String>) {
    *MESSAGE.lock().unwrap() = Some(text.into());
}

/// Keeps the list of tablets on the network up to date.
pub fn start_browse() {
    std::thread::spawn(|| loop {
        // `script` gives dns-sd a terminal, so its output arrives line by line instead of in big blocks.
        // The wrapper shell stops the browser when the host exits, so none are left behind.
        let child = Command::new("sh")
            .args([
                "-c",
                "script -q /dev/null dns-sd -B \"$1\" local. & pid=$!; \
                 while kill -0 $PPID 2>/dev/null; do sleep 2; done; kill $pid",
                "sh",
                TABLET_TYPE,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        if let Ok(mut child) = child {
            if let Some(out) = child.stdout.take() {
                for line in BufReader::new(out).lines().map_while(Result::ok) {
                    let Some((_, rest)) = line.split_once(&format!("{TABLET_TYPE}.")) else { continue };
                    let name = rest.trim().to_string();
                    if name.is_empty() {
                        continue;
                    }
                    let mut list = TABLETS.lock().unwrap();
                    if line.contains(" Add ") {
                        if !list.contains(&name) {
                            list.push(name);
                        }
                    } else if line.contains(" Rmv ") {
                        list.retain(|n| *n != name);
                    }
                }
            }
            let _ = child.kill();
        }
        std::thread::sleep(Duration::from_secs(2));
    });
}

fn kill(mut c: Child) {
    let _ = c.kill();
    let _ = c.wait();
}

/// `host:port` of a tablet, from `dns-sd -L`.
fn resolve(name: &str) -> Option<String> {
    let mut child = Command::new("dns-sd")
        .args(["-L", name, TABLET_TYPE, "local."])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let out = child.stdout.take()?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Some((_, rest)) = line.split_once("can be reached at ") {
                let _ = tx.send(rest.split_whitespace().next().unwrap_or_default().to_string());
                break;
            }
        }
    });
    let r = rx.recv_timeout(Duration::from_secs(5)).ok();
    kill(child);
    r
}

/// Sends `code` to the tablet called `name`; the result shows up through [`take_message`].
pub fn pair(name: String, code: u32) {
    std::thread::spawn(move || {
        let Some(addr) = resolve(&name) else {
            return say(format!("Could not find {name} on this network."));
        };
        let Some(mut stream) = addr
            .trim_end_matches('.')
            .to_socket_addrs()
            .ok()
            .and_then(|it| {
                // IPv4 first: the tablet connects back to the address it sees here, and the host listens on IPv4.
                let mut addrs: Vec<_> = it.collect();
                addrs.sort_by_key(|a| a.is_ipv6());
                addrs.into_iter().find_map(|a| TcpStream::connect_timeout(&a, Duration::from_secs(3)).ok())
            })
        else {
            return say(format!("Could not reach {name}."));
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut msg = b"LGRQ".to_vec();
        msg.extend_from_slice(&code.to_be_bytes());
        msg.extend_from_slice(&PORT.load(Ordering::Relaxed).to_be_bytes());
        let mut reply = [0u8; 1];
        if stream.write_all(&msg).is_err() || stream.read_exact(&mut reply).is_err() {
            return say(format!("{name} did not answer."));
        }
        if reply[0] != 1 {
            return say("Wrong code. Check the code on the tablet.");
        }
        crate::access::set_code(code);
        crate::access::set_enabled(true);
        let mut c = crate::config::Config::load();
        c.code = code;
        c.sharing = true;
        c.save();
        say(format!("Paired with {name}."));
    });
}
