//! ScreenCaptureKit capture of one display, feeding the encoder.
//!
//! ScreenCaptureKit only delivers frames when the screen changes. To serve a
//! keyframe to a newly connected client on a static screen, the last captured
//! frame is retained and re-encoded on request.
//!
//! The retained frame is also re-sent twice shortly after the screen goes idle:
//! hardware decoders hold back the newest frame, so without these repeats the
//! final cursor position would stay invisible until the next screen change.

use crate::encoder::{Codec, Encoder, Packet};
use anyhow::{anyhow, Result};
use screencapturekit::cm::CMSampleBufferExt;
use screencapturekit::prelude::*;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRetain(cf: *const c_void) -> *const c_void;
    fn CFRelease(cf: *const c_void);
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CmTime {
    value: i64,
    timescale: i32,
    flags: u32,
    epoch: i64,
}

#[link(name = "CoreMedia", kind = "framework")]
extern "C" {
    fn CMClockGetHostTimeClock() -> *const c_void;
    fn CMClockGetTime(clock: *const c_void) -> CmTime;
}

/// Sum (microseconds) and count of how late frames reach us after ScreenCaptureKit stamps them.
pub static CAPTURE_US_SUM: AtomicU64 = AtomicU64::new(0);
pub static CAPTURE_COUNT: AtomicU64 = AtomicU64::new(0);

#[link(name = "CoreVideo", kind = "framework")]
extern "C" {
    fn CVPixelBufferGetWidth(pb: *const c_void) -> usize;
    fn CVPixelBufferGetHeight(pb: *const c_void) -> usize;
}

/// Frames delivered by ScreenCaptureKit (for diagnostics).
pub static FRAMES_CAPTURED: AtomicU64 = AtomicU64::new(0);

pub struct Settings {
    pub codec: Codec,
    pub low_latency_rc: bool,
    pub show_cursor: bool,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_bps: u32,
}

/// An owned (retained) CVPixelBufferRef.
struct Held(*const c_void);
unsafe impl Send for Held {}
impl Drop for Held {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) }
    }
}

/// Extra copies of the last frame sent once the screen goes idle.
const FLUSH_REPEATS: u32 = 2;

struct Shared {
    settings: Settings,
    tx: Sender<Packet>,
    info: Sender<String>,
    encoder: Mutex<Option<Encoder>>,
    /// Newest captured frame not yet encoded. A newer frame replaces (drops) an older one.
    latest: Mutex<Option<Held>>,
    wake: Condvar,
    /// The frame most recently encoded; re-encoded for keyframe requests and idle flushes.
    last: Mutex<Option<Held>>,
    start: Instant,
    key_request: Arc<AtomicBool>,
    /// Microseconds since `start` of the last real frame from ScreenCaptureKit.
    last_real_us: AtomicU64,
    repeats: AtomicU32,
}

impl Shared {
    fn encode(&self, pb: *const c_void, force_key: bool, repeat: bool) {
        let mut guard = self.encoder.lock().unwrap();
        if guard.is_none() {
            // Size the encoder from the real frame so it matches what SCK delivers.
            let (w, h) = unsafe { (CVPixelBufferGetWidth(pb) as u32, CVPixelBufferGetHeight(pb) as u32) };
            let _ = self.info.send(format!("first frame: {w}x{h}"));
            match Encoder::new(w, h, self.settings.fps, self.settings.bitrate_bps, self.settings.low_latency_rc, self.settings.codec, self.tx.clone()) {
                Ok(e) => *guard = Some(e),
                Err(e) => {
                    let _ = self.info.send(format!("encoder init failed: {e}"));
                    return;
                }
            }
        }
        let pts = self.start.elapsed().as_micros() as i64;
        let force = force_key;
        if let Err(e) = guard.as_ref().unwrap().encode(pb, pts, 1_000_000, force, repeat) {
            let _ = self.info.send(format!("encode error: {e}"));
        }
    }
}

struct Handler(Arc<Shared>);

impl SCStreamOutputTrait for Handler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, kind: SCStreamOutputType) {
        if !matches!(kind, SCStreamOutputType::Screen) {
            return;
        }
        let Some(pb) = sample.pixel_buffer() else { return };
        FRAMES_CAPTURED.fetch_add(1, Ordering::Relaxed);
        let ts = sample.presentation_timestamp();
        if ts.timescale > 0 {
            let now = unsafe { CMClockGetTime(CMClockGetHostTimeClock()) };
            if now.timescale > 0 {
                let late_us = (now.value as f64 / now.timescale as f64 - ts.value as f64 / ts.timescale as f64) * 1e6;
                if (0.0..1e6).contains(&late_us) {
                    CAPTURE_US_SUM.fetch_add(late_us as u64, Ordering::Relaxed);
                    CAPTURE_COUNT.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        let ptr = pb.as_ptr() as *const c_void;
        *self.0.latest.lock().unwrap() = Some(Held(unsafe { CFRetain(ptr) }));
        self.0.last_real_us.store(self.0.start.elapsed().as_micros() as u64, Ordering::SeqCst);
        self.0.repeats.store(0, Ordering::SeqCst);
        self.0.wake.notify_one();
    }
}

pub struct Capture {
    _stream: SCStream,
    stop: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
}

impl Capture {
    /// False once ScreenCaptureKit has stopped the stream (e.g. after sleep or display loss).
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

pub fn start(
    display_id: u32,
    settings: Settings,
    tx: Sender<Packet>,
    info: Sender<String>,
    key_request: Arc<AtomicBool>,
) -> Result<Capture> {
    let content = SCShareableContent::get()
        .map_err(|e| anyhow!("SCShareableContent: {e:?} (Screen Recording permission?)"))?;
    let display = content
        .displays()
        .into_iter()
        .find(|d| d.display_id() == display_id)
        .ok_or_else(|| anyhow!("display {display_id} not visible to ScreenCaptureKit yet"))?;

    let filter = SCContentFilter::create()
        .with_display(&display)
        .with_excluding_windows(&[])
        .build()
        .map_err(|e| anyhow!("{e:?}"))?;

    let config = SCStreamConfiguration::new()
        .with_width(settings.width)
        .with_height(settings.height)
        .with_pixel_format(PixelFormat::YCbCr_420v)
        .with_queue_depth(3)
        .with_shows_cursor(settings.show_cursor)
        .with_minimum_frame_interval(&screencapturekit::cm::CMTime::new(1, settings.fps as i32));

    let shared = Arc::new(Shared {
        settings,
        tx,
        info,
        encoder: Mutex::new(None),
        latest: Mutex::new(None),
        wake: Condvar::new(),
        last: Mutex::new(None),
        start: Instant::now(),
        key_request,
        last_real_us: AtomicU64::new(0),
        repeats: AtomicU32::new(FLUSH_REPEATS),
    });

    let alive = Arc::new(AtomicBool::new(true));
    let alive_flag = alive.clone();
    let delegate = screencapturekit::stream::delegate_trait::ErrorHandler::new(move |e| {
        eprintln!("capture stopped: {e}");
        alive_flag.store(false, Ordering::SeqCst);
    });
    let mut stream = SCStream::new_with_delegate(&filter, &config, delegate).map_err(|e| anyhow!("{e:?}"))?;
    let _ = stream.add_output_handler(Handler(shared.clone()), SCStreamOutputType::Screen);
    stream.start_capture().map_err(|e| anyhow!("{e:?}"))?;

    // Encoder thread: always encodes the newest frame, so a slow encoder never builds a queue
    // of stale frames (and never blocks ScreenCaptureKit's callback).
    let stop = Arc::new(AtomicBool::new(false));
    {
        let (shared, stop) = (shared.clone(), stop.clone());
        std::thread::spawn(move || loop {
            let held = {
                let mut slot = shared.latest.lock().unwrap();
                while slot.is_none() {
                    if stop.load(Ordering::SeqCst) {
                        return;
                    }
                    slot = shared.wake.wait_timeout(slot, Duration::from_millis(50)).unwrap().0;
                }
                slot.take().unwrap()
            };
            let force = shared.key_request.swap(false, Ordering::SeqCst);
            shared.encode(held.0, force, false);
            *shared.last.lock().unwrap() = Some(held);
        });
    }

    // Serves keyframe requests when the screen is static (no new frames arriving).
    {
        let (shared, stop) = (shared.clone(), stop.clone());
        std::thread::spawn(move || {
            let interval_us = 1_000_000 / u64::from(shared.settings.fps.max(1));
            while !stop.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(3));
                if shared.key_request.load(Ordering::SeqCst) {
                    let last = shared.last.lock().unwrap();
                    if let Some(held) = last.as_ref() {
                        if shared.key_request.swap(false, Ordering::SeqCst) {
                            shared.encode(held.0, true, true);
                        }
                    }
                    continue;
                }
                let n = shared.repeats.load(Ordering::SeqCst);
                if n >= FLUSH_REPEATS {
                    continue;
                }
                let idle_us = (shared.start.elapsed().as_micros() as u64).saturating_sub(shared.last_real_us.load(Ordering::SeqCst));
                if idle_us > interval_us * (u64::from(n) + 2) {
                    let last = shared.last.lock().unwrap();
                    if let Some(held) = last.as_ref() {
                        shared.repeats.store(n + 1, Ordering::SeqCst);
                        shared.encode(held.0, false, true);
                    }
                }
            }
        });
    }

    Ok(Capture { _stream: stream, stop, alive })
}
