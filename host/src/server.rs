//! Video distribution: encoded packets go to one client at a time, over whatever transport
//! (TCP socket, USB pipe) the client was attached with. A new client replaces the previous
//! one and triggers a keyframe.

use crate::encoder::Packet;
use crate::protocol::{write_frame, write_hello, Hello, FLAG_HEARTBEAT, FLAG_KEYFRAME, FLAG_REPEAT};
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The current client's id and its packet queue.
type ClientSlot = Arc<Mutex<Option<(u64, SyncSender<Arc<Packet>>)>>>;

pub struct VideoHub {
    slot: ClientSlot,
    /// True until the current client has received a keyframe.
    waiting_key: Arc<AtomicBool>,
    key_request: Arc<AtomicBool>,
    hello: Hello,
    next_id: AtomicU64,
}

impl VideoHub {
    pub fn new(
        hello: Hello,
        packets: Receiver<Packet>,
        key_request: Arc<AtomicBool>,
        mut dump: Option<std::fs::File>,
    ) -> Arc<Self> {
        let slot: ClientSlot = Arc::new(Mutex::new(None));
        let waiting_key = Arc::new(AtomicBool::new(true));

        {
            let (slot, waiting_key, key_request) = (slot.clone(), waiting_key.clone(), key_request.clone());
            std::thread::spawn(move || {
                for pkt in packets {
                    if let Some(f) = dump.as_mut() {
                        let _ = f.write_all(&pkt.data);
                    }
                    if waiting_key.load(Ordering::SeqCst) && !pkt.keyframe {
                        continue;
                    }
                    let mut guard = slot.lock().unwrap();
                    let Some((_, tx)) = guard.as_ref() else { continue };
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
        }
        Arc::new(Self { slot, waiting_key, key_request, hello, next_id: AtomicU64::new(1) })
    }

    /// Starts streaming to `writer`, replacing any current client. Returns the client's id.
    pub fn attach<W: Write + Send + 'static>(&self, writer: W, peer: String) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        println!("client connected: {peer}");
        let (tx, rx) = sync_channel::<Arc<Packet>>(30);
        *self.slot.lock().unwrap() = Some((id, tx));
        self.waiting_key.store(true, Ordering::SeqCst);
        self.key_request.store(true, Ordering::SeqCst);
        let hello = self.hello;
        std::thread::spawn(move || {
            let r = client_loop(writer, hello, rx);
            println!("client disconnected: {peer} ({})", r.err().map(|e| e.to_string()).unwrap_or_default());
        });
        id
    }

    /// Stops streaming to client `id`, unless another client has replaced it in the meantime.
    pub fn detach(&self, id: u64) {
        let mut slot = self.slot.lock().unwrap();
        if slot.as_ref().is_some_and(|(current, _)| *current == id) {
            *slot = None;
        }
    }
}

fn client_loop<W: Write>(writer: W, hello: Hello, rx: Receiver<Arc<Packet>>) -> std::io::Result<()> {
    let mut w = BufWriter::with_capacity(256 * 1024, writer);
    write_hello(&mut w, hello)?;
    w.flush()?;
    loop {
        let pkt = match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(pkt) => pkt,
            Err(RecvTimeoutError::Timeout) => {
                write_frame(&mut w, FLAG_HEARTBEAT, 0, &[])?;
                w.flush()?;
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let flags = if pkt.keyframe { FLAG_KEYFRAME } else { 0 } | if pkt.repeat { FLAG_REPEAT } else { 0 };
        write_frame(&mut w, flags, pkt.pts_us, &pkt.data)?;
        w.flush()?;
    }
    Ok(())
}
