//! Cursor channel (TCP, video port + 1). Messages (big-endian), each prefixed by
//! `len u32` (type byte + body):
//!   1 Position: x f32 | y f32 | inside u8        (x, y in 0..1 of the display)
//!   2 Shape:    width f32 | height f32 | hot_x f32 | hot_y f32 | PNG bytes
//!               (sizes as a fraction of the display width/height)

use crate::cursor;
use std::io::{BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
struct State {
    pos: Option<cursor::Position>,
    pos_seq: u64,
    /// Ready-to-send Shape message (without the length prefix).
    shape: Option<Vec<u8>>,
    shape_seq: u64,
}

pub fn spawn(port: u16, display: u32, display_points: (u32, u32)) -> anyhow::Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    println!("cursor channel on 0.0.0.0:{port}");

    let state = Arc::new(Mutex::new(State::default()));
    let clients = Arc::new(AtomicUsize::new(0));

    // Producer: samples the system cursor while somebody is listening.
    {
        let (state, clients) = (state.clone(), clients.clone());
        std::thread::spawn(move || {
            let mut seed = i32::MIN;
            loop {
                if clients.load(Ordering::Relaxed) == 0 {
                    seed = i32::MIN; // resend the shape to the next client
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                let pos = cursor::position(display);
                let new_seed = cursor::shape_seed();
                let shape = (new_seed != seed).then(|| {
                    seed = new_seed;
                    cursor::current_shape().map(|s| encode_shape(&s, display_points))
                });
                let mut st = state.lock().unwrap();
                if st.pos != Some(pos) {
                    st.pos = Some(pos);
                    st.pos_seq += 1;
                }
                if let Some(Some(msg)) = shape {
                    st.shape = Some(msg);
                    st.shape_seq += 1;
                }
                drop(st);
                std::thread::sleep(Duration::from_millis(4));
            }
        });
    }

    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(stream) = conn else { continue };
            let _ = stream.set_nodelay(true);
            let (state, clients) = (state.clone(), clients.clone());
            clients.fetch_add(1, Ordering::Relaxed);
            std::thread::spawn(move || {
                let _ = client_loop(stream, &state);
                clients.fetch_sub(1, Ordering::Relaxed);
            });
        }
    });
    Ok(())
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

fn client_loop(stream: TcpStream, state: &Mutex<State>) -> std::io::Result<()> {
    let mut w = BufWriter::new(stream);
    let (mut pos_seq, mut shape_seq) = (0u64, 0u64);
    loop {
        let (pos, shape) = {
            let st = state.lock().unwrap();
            (
                (st.pos_seq != pos_seq).then(|| (st.pos, st.pos_seq)),
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
            let mut m = [0u8; 10];
            m[0] = 1;
            m[1..5].copy_from_slice(&p.x.to_be_bytes());
            m[5..9].copy_from_slice(&p.y.to_be_bytes());
            m[9] = p.inside as u8;
            send(&mut w, &m)?;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
