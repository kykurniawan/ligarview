mod capture;
mod cursor;
mod cursor_server;
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
    /// Logical width in points (framebuffer is width x scale pixels)
    #[arg(long, default_value_t = 1280)]
    width: u32,
    /// Logical height in points
    #[arg(long, default_value_t = 800)]
    height: u32,
    /// Pixel density: 2 = HiDPI (Retina-like), 1 = one pixel per point
    #[arg(long, default_value_t = 2)]
    scale: u32,
    #[arg(long, default_value_t = 60)]
    fps: u32,
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
    /// Keep the cursor inside the video instead of drawing it on the tablet (adds latency)
    #[arg(long)]
    cursor_in_video: bool,
    /// Print capture/encode rates once per second
    #[arg(long)]
    stats: bool,
    /// Diagnostic: print the current system cursor shape for a few seconds and exit
    #[arg(long)]
    cursor_test: bool,
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
    if a.cursor_test {
        let mut seed = -1;
        for _ in 0..80 {
            let s = cursor::shape_seed();
            if s != seed {
                seed = s;
                match cursor::current_shape() {
                    Some(sh) => {
                        println!("seed {s}: {:.0}x{:.0} pts, hot ({:.0},{:.0}), png {} bytes", sh.width_pts, sh.height_pts, sh.hot_x_pts, sh.hot_y_pts, sh.png.len());
                        std::fs::write(format!("out/cursor_{s}.png"), &sh.png)?;
                    }
                    None => println!("seed {s}: no shape"),
                }
            }
            let p = cursor::position(1);
            if seed == s { let _ = p; }
            std::thread::sleep(Duration::from_millis(100));
        }
        return Ok(());
    }
    let vd = VirtualDisplay::create(a.width, a.height, a.scale, a.fps as f64)?;
    let (px_w, px_h) = (a.width * a.scale, a.height * a.scale);
    println!("virtual display id={}: {}", vd.id, VirtualDisplay::describe(vd.id));
    // macOS needs a moment before it accepts a mode change on a new display.
    let mut selected = Err(anyhow::anyhow!("not tried"));
    for _ in 0..15 {
        std::thread::sleep(Duration::from_millis(300));
        selected = VirtualDisplay::select_mode(vd.id, (a.width, a.height), (px_w, px_h));
        if selected.is_ok() {
            break;
        }
    }
    match selected {
        Ok(()) => {
            let (x, y, w, h) = VirtualDisplay::bounds(vd.id);
            println!("selected mode: {} at ({x:.0},{y:.0}) {w:.0}x{h:.0} pts", VirtualDisplay::describe(vd.id));
        }
        Err(e) => println!("could not select mode: {e}"),
    }
    for (id, w, h) in VirtualDisplay::active_displays() {
        println!("  display {id}: {w}x{h}{}", if id == vd.id { "  <- ligarview" } else { "" });
    }

    let (tx, rx) = mpsc::channel();
    let (info_tx, info_rx) = mpsc::channel();
    let key_request = Arc::new(AtomicBool::new(true));

    let dump = a.out.as_ref().map(std::fs::File::create).transpose()?;
    let hello = protocol::Hello { width: px_w as u16, height: px_h as u16, fps: a.fps as u16 };
    server::spawn(a.port, hello, rx, key_request.clone(), dump)?;
    if !a.cursor_in_video {
        cursor_server::spawn(a.port + 1, vd.id, (a.width, a.height))?;
    }

    // The display needs a moment before ScreenCaptureKit lists it.
    let mut capture = None;
    let mut last_err = None;
    for _ in 0..20 {
        let s = capture::Settings { show_cursor: a.cursor_in_video, width: px_w, height: px_h, fps: a.fps, bitrate_bps: a.bitrate * 1_000_000 };
        match capture::start(vd.id, s, tx.clone(), info_tx.clone(), key_request.clone()) {
            Ok(c) => { capture = Some(c); break; }
            Err(e) => { last_err = Some(e); std::thread::sleep(Duration::from_millis(500)); }
        }
    }
    let Some(_capture) = capture else { return Err(last_err.unwrap()) };
    println!("capturing {px_w}x{px_h} @ {} fps, {} Mbps", a.fps, a.bitrate);

    let deadline = Instant::now() + Duration::from_secs(if a.seconds == 0 { u64::MAX / 4 } else { a.seconds });
    if a.wiggle {
        let id = vd.id;
        std::thread::spawn(move || wiggle(id, deadline));
    }
    let mut last = (0u64, 0u64, Instant::now());
    let mut lat_prev = (0u64, 0u64, 0u64, 0u64);
    while Instant::now() < deadline {
        if let Ok(m) = info_rx.recv_timeout(Duration::from_millis(200)) {
            println!("{m}");
        }
        if a.stats && last.2.elapsed() >= Duration::from_secs(1) {
            use std::sync::atomic::Ordering::Relaxed;
            let (c, e) = (capture::FRAMES_CAPTURED.load(Relaxed), encoder::PACKETS_ENCODED.load(Relaxed));
            let dt = last.2.elapsed().as_secs_f64();
            let cur = (
                capture::CAPTURE_US_SUM.load(Relaxed),
                capture::CAPTURE_COUNT.load(Relaxed),
                encoder::ENCODE_US_SUM.load(Relaxed),
                encoder::ENCODE_COUNT.load(Relaxed),
            );
            let avg = |sum: u64, n: u64| if n == 0 { 0.0 } else { sum as f64 / n as f64 / 1000.0 };
            println!(
                "captured {:.0} fps, encoded {:.0} fps | SCK delivery {:.1} ms, encode {:.1} ms",
                (c - last.0) as f64 / dt,
                (e - last.1) as f64 / dt,
                avg(cur.0 - lat_prev.0, cur.1 - lat_prev.1),
                avg(cur.2 - lat_prev.2, cur.3 - lat_prev.3),
            );
            lat_prev = cur;
            last = (c, e, Instant::now());
        }
    }
    Ok(())
}
