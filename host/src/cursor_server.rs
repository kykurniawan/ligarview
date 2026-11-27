//! Cursor channel (TCP port + 1, or the USB pipe's cursor channel). Messages (big-endian), each prefixed by
//! `len u32` (type byte + body):
//!   1 Position: x f32 | y f32 | inside u8        (x, y in 0..1 of the display)
//!   2 Shape:    width f32 | height f32 | hot_x f32 | hot_y f32 | PNG bytes
//!               (sizes as a fraction of the display width/height)

use crate::cursor;
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Wall-clock time of day with milliseconds, to correlate with the tablet's logcat.
fn ts() -> String {
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let ms = d.as_millis() % 86_400_000;
    // Local timezone offset is not needed to compare lines within one session.
    format!("{:02}:{:02}:{:02}.{:03}", ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000)
}

#[derive(Default)]
struct State {
    pos: Option<cursor::Position>,
    pos_seq: u64,
    /// Ready-to-send Shape message (without the length prefix).
    shape: Option<Vec<u8>>,
    shape_seq: u64,
}

/// Samples the system cursor and feeds every attached client. Works with any transport.
pub struct CursorHub {
    state: Arc<Mutex<State>>,
    clients: Arc<AtomicUsize>,
    stops: Mutex<Vec<Arc<AtomicBool>>>,
}

impl CursorHub {
    /// `display` can change when the virtual display is recreated.
    pub fn new(display: Arc<AtomicU32>, display_points: (u32, u32)) -> Arc<Self> {
        let state = Arc::new(Mutex::new(State::default()));
        let clients = Arc::new(AtomicUsize::new(0));

        // Producer: samples the system cursor while somebody is listening.
        {
            let (state, clients) = (state.clone(), clients.clone());
            std::thread::spawn(move || {
                let mut seed = i32::MIN;
                let mut last_png: Vec<u8> = Vec::new();
                let mut tick = 0u32;
                let mut was_inside: Option<bool> = None;
                loop {
                    if clients.load(Ordering::Relaxed) == 0 {
                        seed = i32::MIN; // resend the shape to the next client
                        last_png.clear();
                        std::thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    let pos = cursor::position(display.load(Ordering::Relaxed));
                    if was_inside != Some(pos.inside) {
                        was_inside = Some(pos.inside);
                        println!(
                            "{} cursor: {} the virtual display at ({:.3}, {:.3})",
                            ts(),
                            if pos.inside { "entered" } else { "left" },
                            pos.x,
                            pos.y
                        );
                    }
                    let new_seed = cursor::shape_seed();
                    tick = tick.wrapping_add(1);
                    // The seed does not catch every change (e.g. cursors hidden and shown again),
                    // so the image is also re-read every ~100 ms and sent when it differs.
                    let shape = (new_seed != seed || tick % 25 == 0)
                        .then(|| {
                            seed = new_seed;
                            let shape = cursor::current_shape();
                            if shape.is_none() {
                                println!("cursor: no image for shape seed {new_seed}");
                            }
                            shape.filter(|s| s.png != last_png).map(|s| {
                                println!(
                                    "{} cursor: shape {:.0}x{:.0} pts, hot ({:.0},{:.0}), {} bytes",
                                    ts(),
                                    s.width_pts, s.height_pts, s.hot_x_pts, s.hot_y_pts, s.png.len()
                                );
                                last_png = s.png.clone();
                                encode_shape(&s, display_points)
                            })
                        })
                        .flatten();
                    let mut st = state.lock().unwrap();
                    if st.pos != Some(pos) {
                        st.pos = Some(pos);
                        st.pos_seq += 1;
                    }
                    if let Some(msg) = shape {
                        st.shape = Some(msg);
                        st.shape_seq += 1;
                    }
                    drop(st);
                    std::thread::sleep(Duration::from_millis(4));
                }
            });
        }
        Arc::new(Self { state, clients, stops: Mutex::new(Vec::new()) })
    }

    /// Ends every attached cursor client.
    pub fn kick(&self) {
        for s in self.stops.lock().unwrap().drain(..) {
            s.store(true, Ordering::Relaxed);
        }
    }

    /// Starts sending to `writer`. Returns a flag that ends the client when set.
    pub fn attach<W: Write + Send + 'static>(&self, writer: W) -> Arc<AtomicBool> {
        let stop = Arc::new(AtomicBool::new(!crate::access::enabled()));
        if stop.load(Ordering::Relaxed) {
            return stop;
        }
        {
            let mut stops = self.stops.lock().unwrap();
            stops.retain(|s| !s.load(Ordering::Relaxed));
            stops.push(stop.clone());
        }
        let (state, clients, flag) = (self.state.clone(), self.clients.clone(), stop.clone());
        clients.fetch_add(1, Ordering::Relaxed);
        std::thread::spawn(move || {
            let _ = client_loop(writer, &state, &flag);
            clients.fetch_sub(1, Ordering::Relaxed);
        });
        stop
    }
}

fn encode_shape(s: &cursor::Shape, (dw, dh): (u32, u32)) -> Vec<u8> {
    let (dw, dh) = (dw as f32, dh as f32);
    let mut m = Vec::with_capacity(17 + s.png.len());
    m.push(2);
    for v in [s.width_pts / dw, s.height_pts / dh, s.hot_x_pts / dw, s.hot_y_pts / dh] {
        m.extend_from_slice(&v.to_be_bytes());
    }
    m.extend_from_slice(&s.png);
    m
}

fn send(w: &mut impl Write, body: &[u8]) -> std::io::Result<()> {
    w.write_all(&(body.len() as u32).to_be_bytes())?;
    w.write_all(body)?;
    w.flush()
}

fn client_loop<W: Write>(writer: W, state: &Mutex<State>, stop: &AtomicBool) -> std::io::Result<()> {
    let mut w = BufWriter::new(writer);
    let (mut pos_seq, mut shape_seq) = (0u64, 0u64);
    let mut last_pos_sent = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        let (pos, shape) = {
            let st = state.lock().unwrap();
            (
                // Also resend once a second so the client can detect a dead connection.
                (st.pos_seq != pos_seq || last_pos_sent.elapsed() >= Duration::from_secs(1))
                    .then(|| (st.pos, st.pos_seq)),
                (st.shape_seq != shape_seq).then(|| (st.shape.clone(), st.shape_seq)),
            )
        };
        if let Some((msg, seq)) = shape {
            shape_seq = seq;
            if let Some(msg) = msg {
                send(&mut w, &msg)?;
            }
        }
        if let Some((Some(p), seq)) = pos {
            pos_seq = seq;
            last_pos_sent = Instant::now();
            let mut m = [0u8; 10];
            m[0] = 1;
            m[1..5].copy_from_slice(&p.x.to_be_bytes());
            m[5..9].copy_from_slice(&p.y.to_be_bytes());
            m[9] = p.inside as u8;
            send(&mut w, &m)?;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}
