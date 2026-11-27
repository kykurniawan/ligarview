//! TCP transport: video on `port`, cursor on `port + 1`. Used for Wi-Fi, or over USB through
//! `adb reverse`.

use crate::cursor_server::CursorHub;
use crate::server::VideoHub;
use anyhow::Result;
use std::net::TcpListener;
use std::sync::Arc;

pub fn spawn(port: u16, video: Arc<VideoHub>, cursor: Option<Arc<CursorHub>>) -> Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    println!("listening on 0.0.0.0:{port}");
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(stream) = conn else { continue };
            let _ = stream.set_nodelay(true);
            let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
            let _ = video.attach(stream, peer);
        }
    });

    if let Some(cursor) = cursor {
        let listener = TcpListener::bind(("0.0.0.0", port + 1))?;
        println!("cursor channel on 0.0.0.0:{}", port + 1);
        std::thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(stream) = conn else { continue };
                let _ = stream.set_nodelay(true);
                cursor.attach(stream);
            }
        });
    }
    Ok(())
}
