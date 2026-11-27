//! TCP transport: video on `port`, cursor on `port + 1`. Used for Wi-Fi, or over USB through
//! `adb reverse`.

use crate::cursor_server::CursorHub;
use crate::server::VideoHub;
use anyhow::Result;
use crate::access;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use std::sync::Arc;

pub fn spawn(port: u16, video: Arc<VideoHub>, cursor: Option<Arc<CursorHub>>) -> Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    println!("listening on 0.0.0.0:{port}");
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(stream) = conn else { continue };
            let video = video.clone();
            std::thread::spawn(move || {
                let Some(stream) = admit(stream) else { return };
                let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
                let _ = video.attach(stream, peer);
            });
        }
    });

    if let Some(cursor) = cursor {
        let listener = TcpListener::bind(("0.0.0.0", port + 1))?;
        println!("cursor channel on 0.0.0.0:{}", port + 1);
        std::thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(stream) = conn else { continue };
                let cursor = cursor.clone();
                std::thread::spawn(move || {
                    if let Some(stream) = admit(stream) {
                        // The tablet's pen input comes back on the same connection.
                        let reader = stream.try_clone();
                        let stop = cursor.attach(stream);
                        if let Ok(reader) = reader {
                            let _ = reader.set_read_timeout(Some(Duration::from_millis(500)));
                            crate::input::read_stream(reader, &stop);
                        }
                    }
                });
            }
        });
    }
    Ok(())
}

/// Every connection starts with `LGRP` + the pairing code (u32, big-endian). Connections from this
/// Mac itself (adb reverse) skip the code check. Returns the stream when it may proceed.
fn admit(mut stream: TcpStream) -> Option<TcpStream> {
    let _ = stream.set_nodelay(true);
    let reject = |why: &str, stream: &TcpStream| {
        println!("connection refused ({why}): {}", stream.peer_addr().map(|a| a.to_string()).unwrap_or_default());
        std::thread::sleep(Duration::from_millis(300));
    };
    if !access::enabled() {
        reject("sharing is off", &stream);
        return None;
    }
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut buf = [0u8; 8];
    if stream.read_exact(&mut buf).is_err() || &buf[..4] != b"LGRP" {
        reject("no pairing preamble", &stream);
        return None;
    }
    let loopback = stream.peer_addr().is_ok_and(|a| a.ip().is_loopback());
    let code = access::code();
    if !loopback && (code == 0 || u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]) != code) {
        reject("wrong pairing code", &stream);
        return None;
    }
    let _ = stream.set_read_timeout(None);
    Some(stream)
}
