//! TCP server: streams encoded packets to one client at a time.
//! A new connection replaces the previous one and triggers a keyframe.

use crate::encoder::Packet;
use crate::protocol::{write_frame, write_hello, Hello, FLAG_KEYFRAME, FLAG_REPEAT};
use anyhow::Result;
use std::io::{BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

type ClientSlot = Arc<Mutex<Option<SyncSender<Arc<Packet>>>>>;

pub fn spawn(
    port: u16,
    hello: Hello,
    packets: Receiver<Packet>,
    key_request: Arc<AtomicBool>,
    mut dump: Option<std::fs::File>,
) -> Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    println!("listening on 0.0.0.0:{port}");

    let slot: ClientSlot = Arc::new(Mutex::new(None));
    // True until the current client has received a keyframe.
    let waiting_key = Arc::new(AtomicBool::new(true));

    {
        let (slot, waiting_key, key_request) = (slot.clone(), waiting_key.clone(), key_request.clone());
        std::thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(stream) = conn else { continue };
                let _ = stream.set_nodelay(true);
                let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
                println!("client connected: {peer}");
                let (tx, rx) = sync_channel::<Arc<Packet>>(30);
                *slot.lock().unwrap() = Some(tx);
                waiting_key.store(true, Ordering::SeqCst);
                key_request.store(true, Ordering::SeqCst);
                std::thread::spawn(move || {
                    let r = client_loop(stream, hello, rx);
                    println!("client disconnected: {peer} ({})", r.err().map(|e| e.to_string()).unwrap_or_default());
                });
            }
        });
    }

    std::thread::spawn(move || {
        for pkt in packets {
            if let Some(f) = dump.as_mut() {
                let _ = f.write_all(&pkt.data);
            }
            if waiting_key.load(Ordering::SeqCst) && !pkt.keyframe {
                continue;
            }
            let mut guard = slot.lock().unwrap();
            let Some(tx) = guard.as_ref() else { continue };
            let was_key = pkt.keyframe;
            match tx.try_send(Arc::new(pkt)) {
                Ok(()) => {
                    if was_key {
                        waiting_key.store(false, Ordering::SeqCst);
                    }
                }
                Err(TrySendError::Full(_)) => {
                    // Client too slow: drop until the next keyframe to avoid corrupt references.
                    waiting_key.store(true, Ordering::SeqCst);
                    key_request.store(true, Ordering::SeqCst);
                }
                Err(TrySendError::Disconnected(_)) => *guard = None,
            }
        }
    });
    Ok(())
}

fn client_loop(stream: TcpStream, hello: Hello, rx: Receiver<Arc<Packet>>) -> std::io::Result<()> {
    let mut w = BufWriter::with_capacity(256 * 1024, stream);
    write_hello(&mut w, hello)?;
    w.flush()?;
    for pkt in rx {
        let flags = if pkt.keyframe { FLAG_KEYFRAME } else { 0 } | if pkt.repeat { FLAG_REPEAT } else { 0 };
        write_frame(&mut w, flags, pkt.pts_us, &pkt.data)?;
        w.flush()?;
    }
    Ok(())
}
