mod access;
mod adb;
mod bonjour;
mod capture;
mod cursor;
mod cursor_server;
mod config;
mod encoder;
mod gui;
mod input;
mod pairing;
mod protocol;
mod server;
mod tcp;
mod usb;
mod virtual_display;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use virtual_display::{pump_run_loop, VirtualDisplay};

#[derive(Clone, Copy, ValueEnum)]
enum CodecArg {
    H264,
    Hevc,
}

#[derive(Parser)]
struct Args {
    /// Video codec (HEVC needs Android 5+ hardware decoder support; all modern tablets have it)
    #[arg(long, value_enum, default_value_t = CodecArg::H264)]
    codec: CodecArg,
    /// Logical width in points (framebuffer is width x scale pixels)
    #[arg(long, default_value_t = 1280)]
    width: u32,
    /// Logical height in points
    #[arg(long, default_value_t = 800)]
    height: u32,
    /// Make the display upright (swaps width and height), e.g. for reading code on a tablet held vertically
    #[arg(long)]
    portrait: bool,
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
    /// Pair with a tablet by name from the command line (debugging): --pair NAME CODE
    #[arg(long, num_args = 2, value_names = ["TABLET", "CODE"])]
    pair: Option<Vec<String>>,
    /// Keep the cursor inside the video instead of drawing it on the tablet (adds latency)
    #[arg(long)]
    cursor_in_video: bool,
    /// Use VideoToolbox's low-latency rate control: ~10 ms less encode latency, but it caps
    /// throughput near 30 fps on busy screens. Off by default (smoother window dragging).
    #[arg(long)]
    low_latency_rc: bool,
    /// Print capture/encode rates once per second
    #[arg(long)]
    stats: bool,
    /// Also manage `adb reverse` tunnels (developer mode: needs USB debugging and adb)
    #[arg(long)]
    adb: bool,
    /// Do not use the USB (Android Open Accessory) transport
    #[arg(long)]
    no_usb: bool,
    /// Do not listen for TCP connections (Wi-Fi, adb)
    #[arg(long)]
    no_tcp: bool,
    /// Test helper: drop the virtual display after N seconds to exercise recovery
    #[arg(long)]
    debug_drop_display_after: Option<u64>,
    /// Diagnostic: render every standard AppKit cursor to out/cursors and exit
    #[arg(long)]
    cursor_test: bool,
    /// Diagnostic: print every change of the system cursor for 20 seconds and exit
    #[arg(long)]
    cursor_watch: bool,
    /// Run with the small settings window (default when started from Ligarview.app)
    #[arg(long)]
    gui: bool,
    /// With the window: start without showing it (used when launching at login)
    #[arg(long)]
    background: bool,
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

fn wiggle(display: Arc<AtomicU32>, until: Instant) {
    let mut t = 0.0f64;
    while Instant::now() < until {
        let b = unsafe { CGDisplayBounds(display.load(Ordering::Relaxed)) };
        let (cx, cy) = (b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
        let p = CGPoint { x: cx + 300.0 * t.cos(), y: cy + 200.0 * t.sin() };
        unsafe { CGWarpMouseCursorPosition(p) };
        t += 0.08;
        std::thread::sleep(Duration::from_millis(16));
    }
}

/// Creates the virtual display and switches it to the requested (HiDPI) mode.
///
/// With `strict`, failing to reach the requested mode is an error (the picture would be blurry);
/// at startup it is only reported.
fn create_display(a: &Args, strict: bool) -> Result<VirtualDisplay> {
    let (px_w, px_h) = (a.width * a.scale, a.height * a.scale);
    let vd = VirtualDisplay::create(a.width, a.height, a.scale, a.fps as f64)?;
    println!("virtual display id={}: {}", vd.id, VirtualDisplay::describe(vd.id));
    // macOS needs a moment before it accepts a mode change on a new display.
    let mut selected = Err(anyhow::anyhow!("not tried"));
    for _ in 0..15 {
        pump_run_loop(Duration::from_millis(300));
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
        Err(e) if strict => return Err(anyhow::anyhow!("could not select mode: {e}")),
        Err(e) => println!("could not select mode: {e}"),
    }
    Ok(vd)
}

/// Starts capturing `display`; ScreenCaptureKit needs a moment before it lists a new display.
fn start_capture(
    a: &Args,
    codec: encoder::Codec,
    display: u32,
    tx: &mpsc::Sender<encoder::Packet>,
    info_tx: &mpsc::Sender<String>,
    key_request: &Arc<AtomicBool>,
) -> Result<capture::Capture> {
    let (px_w, px_h) = (a.width * a.scale, a.height * a.scale);
    let mut last_err = None;
    for _ in 0..20 {
        let s = capture::Settings {
            codec,
            low_latency_rc: a.low_latency_rc,
            show_cursor: a.cursor_in_video,
            width: px_w,
            height: px_h,
            fps: a.fps,
            bitrate_bps: a.bitrate * 1_000_000,
        };
        match capture::start(display, s, tx.clone(), info_tx.clone(), key_request.clone()) {
            Ok(c) => return Ok(c),
            Err(e) => {
                last_err = Some(e);
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
    Err(last_err.unwrap())
}

fn main() -> Result<()> {
    let mut a = Args::parse();
    let gui = a.gui || gui::launched_from_bundle();
    if gui {
        let c = config::Config::load();
        a.portrait = c.portrait;
        a.fps = c.fps;
        a.bitrate = c.bitrate;
        a.codec = if c.hevc { CodecArg::Hevc } else { CodecArg::H264 };
        // Only the chosen transport runs: Wi-Fi listens on the network and pairs by code, USB
        // needs neither (and the host then does not listen on the network at all).
        a.no_tcp = !c.wifi;
        a.no_usb = c.wifi;
    }
    if a.portrait {
        std::mem::swap(&mut a.width, &mut a.height);
    }
    if a.cursor_test {
        std::fs::create_dir_all("out/cursors")?;
        for (name, shape) in cursor::standard_shapes() {
            match shape {
                Some(sh) => {
                    println!("{name:<20} {:>3.0}x{:<3.0} pts  hot ({:>2.0},{:>2.0})  png {:>5} bytes", sh.width_pts, sh.height_pts, sh.hot_x_pts, sh.hot_y_pts, sh.png.len());
                    std::fs::write(format!("out/cursors/{name}.png"), &sh.png)?;
                }
                None => println!("{name:<20} NO IMAGE"),
            }
        }
        return Ok(());
    }
    if let Some(p) = &a.pair {
        pairing::init(a.port);
        pairing::pair(p[0].clone(), p[1].parse()?);
        for _ in 0..100 {
            if let Some(m) = pairing::take_message() {
                println!("{m}");
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        println!("no answer");
        return Ok(());
    }
    if a.cursor_watch {
        let mut seed = -1;
        for _ in 0..200 {
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
            std::thread::sleep(Duration::from_millis(100));
        }
        return Ok(());
    }

    let (px_w, px_h) = (a.width * a.scale, a.height * a.scale);
    let codec = match a.codec {
        CodecArg::H264 => encoder::Codec::H264,
        CodecArg::Hevc => encoder::Codec::Hevc,
    };

    if gui {
        let c = config::Config::load();
        gui::init(&c, (px_w, px_h), if c.hevc { "HEVC" } else { "H.264" }, !a.background);
    }
    let mut vd = Some(create_display(&a, false)?);
    for (id, w, h) in VirtualDisplay::active_displays() {
        println!("  display {id}: {w}x{h}{}", if Some(id) == vd.as_ref().map(|v| v.id) { "  <- ligarview" } else { "" });
    }
    let display_id = Arc::new(AtomicU32::new(vd.as_ref().unwrap().id));

    let (tx, rx) = mpsc::channel();
    let (info_tx, info_rx) = mpsc::channel();
    let key_request = Arc::new(AtomicBool::new(true));

    let dump = a.out.as_ref().map(std::fs::File::create).transpose()?;
    let hello = protocol::Hello { codec: codec.id(), width: px_w as u16, height: px_h as u16, fps: a.fps as u16 };
    let video = server::VideoHub::new(hello, rx, key_request.clone(), dump);
    let cursor = (!a.cursor_in_video).then(|| cursor_server::CursorHub::new(display_id.clone(), (a.width, a.height)));
    {
        let c = config::Config::load();
        access::init(c.sharing || !gui, c.code, c.tablet);
        access::set_control(c.control);
        access::set_finger(c.finger);
        if gui && (c.control || c.finger) && !input::request_trust() {
            println!("input: waiting for the Accessibility permission");
        }
        input::init(display_id.clone());
    }
    if !a.no_tcp {
        tcp::spawn(a.port, video.clone(), cursor.clone())?;
        bonjour::start(a.port);
        pairing::init(a.port);
        if gui {
            pairing::start_browse();
        }
    }
    // adb and the USB accessory transport fight over the device (the link drops whenever adb
    // talks to it), and developer mode has no use for the accessory, so --adb replaces it.
    if !a.no_usb && !a.adb {
        usb::spawn(video.clone(), cursor.clone());
    } else if a.adb {
        println!("adb mode: the USB accessory transport is off");
    }
    if a.adb {
        let mut ports = vec![a.port];
        if cursor.is_some() {
            ports.push(a.port + 1);
        }
        adb::spawn(ports);
    }

    let mut capture = match start_capture(&a, codec, display_id.load(Ordering::Relaxed), &tx, &info_tx, &key_request) {
        Ok(c) => Some(c),
        // The menu bar app keeps running without the Screen Recording permission and says so.
        Err(e) if gui => {
            println!("capture not started: {e}");
            None
        }
        Err(e) => return Err(e),
    };
    println!("capturing {px_w}x{px_h} @ {} fps, {} Mbps, {codec:?}", a.fps, a.bitrate);

    let deadline = Instant::now() + Duration::from_secs(if a.seconds == 0 { u64::MAX / 4 } else { a.seconds });
    if a.wiggle {
        let id = display_id.clone();
        std::thread::spawn(move || wiggle(id, deadline));
    }
    let mut drop_at = a.debug_drop_display_after.map(|s| Instant::now() + Duration::from_secs(s));
    let mut last_health = Instant::now();
    let mut last = (0u64, 0u64, Instant::now());
    let mut lat_prev = (0u64, 0u64, 0u64, 0u64);
    while Instant::now() < deadline {
        while let Ok(m) = info_rx.try_recv() {
            println!("{m}");
        }
        pump_run_loop(Duration::from_millis(100));
        if access::take_kick() {
            video.kick();
            if let Some(c) = &cursor {
                c.kick();
            }
        }
        if gui {
            let status = if !access::enabled() {
                gui::Status::Off
            } else if capture.is_none() {
                gui::Status::Starting
            } else if video.has_client() {
                gui::Status::Streaming
            } else {
                gui::Status::Waiting
            };
            gui::tick(status);
        }

        // Supervisor: rebuild the display and capture if either went away
        // (system sleep/wake, display reconfiguration, ScreenCaptureKit stopping).
        if last_health.elapsed() >= Duration::from_millis(500) {
            last_health = Instant::now();
            let mut healthy = capture.as_ref().is_some_and(|c| c.is_alive())
                && vd.as_ref().is_some_and(|v| VirtualDisplay::is_active(v.id));
            if drop_at.is_some_and(|t| Instant::now() >= t) {
                drop_at = None;
                healthy = false;
                println!("(debug) dropping the display to test recovery");
            }
            if !healthy {
                println!("display or capture lost; recreating...");
                let old_id = vd.as_ref().map(|v| v.id);
                capture = None;
                vd = None;
                // macOS releases the old display asynchronously; creating the new one too early
                // yields a display without any modes.
                if let Some(id) = old_id {
                    for _ in 0..50 {
                        if !VirtualDisplay::is_active(id) {
                            break;
                        }
                        pump_run_loop(Duration::from_millis(100));
                    }
                }
                pump_run_loop(Duration::from_secs(1));
                while Instant::now() < deadline {
                    let attempt = create_display(&a, true).and_then(|v| {
                        let c = start_capture(&a, codec, v.id, &tx, &info_tx, &key_request)?;
                        Ok((v, c))
                    });
                    match attempt {
                        Ok((v, c)) => {
                            display_id.store(v.id, Ordering::Relaxed);
                            vd = Some(v);
                            capture = Some(c);
                            key_request.store(true, Ordering::SeqCst);
                            println!("recovered: display {}", display_id.load(Ordering::Relaxed));
                            break;
                        }
                        Err(e) => {
                            println!("recovery failed: {e}; retrying in 2 s");
                            // Keep the menu alive while waiting (e.g. for the Screen Recording permission).
                            for _ in 0..20 {
                                pump_run_loop(Duration::from_millis(100));
                                if gui {
                                    gui::tick(gui::Status::Starting);
                                }
                            }
                        }
                    }
                }
            }
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
