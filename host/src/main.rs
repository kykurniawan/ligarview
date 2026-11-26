mod capture;
mod encoder;
mod protocol;
mod server;
mod virtual_display;

use anyhow::Result;
use clap::Parser;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use virtual_display::VirtualDisplay;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value_t = 2560)]
    width: u32,
    #[arg(long, default_value_t = 1600)]
    height: u32,
    #[arg(long, default_value_t = 60)]
    fps: u32,
    #[arg(long, default_value_t = false)]
    hidpi: bool,
    /// Bitrate in Mbps
    #[arg(long, default_value_t = 30)]
    bitrate: u32,
    /// Record for N seconds, then exit. 0 = run until Ctrl+C.
    #[arg(long, default_value_t = 0)]
    seconds: u64,
    /// TCP port to listen on (use `adb reverse tcp:PORT tcp:PORT` for USB)
    #[arg(long, default_value_t = 7878)]
    port: u16,
    /// Also write the raw H.264 stream to this file (debugging)
    #[arg(long)]
    out: Option<String>,
    /// Test helper: move the cursor around the virtual display so frames change
    #[arg(long)]
    wiggle: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint { x: f64, y: f64 }
#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect { origin: CGPoint, size: CGPoint }

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGDisplayBounds(display: u32) -> CGRect;
    fn CGWarpMouseCursorPosition(p: CGPoint) -> i32;
}

fn wiggle(display: u32, until: Instant) {
    let b = unsafe { CGDisplayBounds(display) };
    let (cx, cy) = (b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
    let mut t = 0.0f64;
    while Instant::now() < until {
        let p = CGPoint { x: cx + 300.0 * t.cos(), y: cy + 200.0 * t.sin() };
        unsafe { CGWarpMouseCursorPosition(p) };
        t += 0.08;
        std::thread::sleep(Duration::from_millis(16));
    }
}

fn main() -> Result<()> {
    let a = Args::parse();
    let vd = VirtualDisplay::create(a.width, a.height, a.fps as f64, a.hidpi)?;
    println!("virtual display id={}", vd.id);
    for (id, w, h) in VirtualDisplay::active_displays() {
        println!("  display {id}: {w}x{h}{}", if id == vd.id { "  <- ligarview" } else { "" });
    }

    let (tx, rx) = mpsc::channel();
    let (info_tx, info_rx) = mpsc::channel();
    let key_request = Arc::new(AtomicBool::new(true));

    let dump = a.out.as_ref().map(std::fs::File::create).transpose()?;
    let hello = protocol::Hello { width: a.width as u16, height: a.height as u16, fps: a.fps as u16 };
    server::spawn(a.port, hello, rx, key_request.clone(), dump)?;

    // The display needs a moment before ScreenCaptureKit lists it.
    let mut capture = None;
    let mut last_err = None;
    for _ in 0..20 {
        let s = capture::Settings { width: a.width, height: a.height, fps: a.fps, bitrate_bps: a.bitrate * 1_000_000 };
        match capture::start(vd.id, s, tx.clone(), info_tx.clone(), key_request.clone()) {
            Ok(c) => { capture = Some(c); break; }
            Err(e) => { last_err = Some(e); std::thread::sleep(Duration::from_millis(500)); }
        }
    }
    let Some(_capture) = capture else { return Err(last_err.unwrap()) };
    println!("capturing {}x{} @ {} fps, {} Mbps", a.width, a.height, a.fps, a.bitrate);

    let deadline = Instant::now() + Duration::from_secs(if a.seconds == 0 { u64::MAX / 4 } else { a.seconds });
    if a.wiggle {
        let id = vd.id;
        std::thread::spawn(move || wiggle(id, deadline));
    }
    while Instant::now() < deadline {
        if let Ok(m) = info_rx.recv_timeout(Duration::from_millis(200)) {
            println!("{m}");
        }
    }
    Ok(())
}
