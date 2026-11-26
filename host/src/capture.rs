//! ScreenCaptureKit capture of one display, feeding the encoder.
//!
//! ScreenCaptureKit only delivers frames when the screen changes. To serve a
//! keyframe to a newly connected client on a static screen, the last captured
//! frame is retained and re-encoded on request.

use crate::encoder::{Encoder, Packet};
use anyhow::{anyhow, Result};
use screencapturekit::cm::CMSampleBufferExt;
use screencapturekit::prelude::*;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRetain(cf: *const c_void) -> *const c_void;
    fn CFRelease(cf: *const c_void);
}

#[link(name = "CoreVideo", kind = "framework")]
extern "C" {
    fn CVPixelBufferGetWidth(pb: *const c_void) -> usize;
    fn CVPixelBufferGetHeight(pb: *const c_void) -> usize;
}

/// Frames delivered by ScreenCaptureKit (for diagnostics).
pub static FRAMES_CAPTURED: AtomicU64 = AtomicU64::new(0);

pub struct Settings {
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

struct Shared {
    settings: Settings,
    tx: Sender<Packet>,
    info: Sender<String>,
    encoder: Mutex<Option<Encoder>>,
    last: Mutex<Option<Held>>,
    start: Instant,
    key_request: Arc<AtomicBool>,
}

impl Shared {
    fn encode(&self, pb: *const c_void, force_key: bool) {
        let mut guard = self.encoder.lock().unwrap();
        if guard.is_none() {
            // Size the encoder from the real frame so it matches what SCK delivers.
            let (w, h) = unsafe { (CVPixelBufferGetWidth(pb) as u32, CVPixelBufferGetHeight(pb) as u32) };
            let _ = self.info.send(format!("first frame: {w}x{h}"));
            match Encoder::new(w, h, self.settings.fps, self.settings.bitrate_bps, self.tx.clone()) {
                Ok(e) => *guard = Some(e),
                Err(e) => {
                    let _ = self.info.send(format!("encoder init failed: {e}"));
                    return;
                }
            }
        }
        let pts = self.start.elapsed().as_micros() as i64;
        let force = force_key;
        if let Err(e) = guard.as_ref().unwrap().encode(pb, pts, 1_000_000, force) {
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
        let ptr = pb.as_ptr() as *const c_void;
        *self.0.last.lock().unwrap() = Some(Held(unsafe { CFRetain(ptr) }));
        let force = self.0.key_request.swap(false, Ordering::SeqCst);
        self.0.encode(ptr, force);
    }
}

pub struct Capture {
    _stream: SCStream,
    stop: Arc<AtomicBool>,
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
        .with_shows_cursor(true)
        .with_minimum_frame_interval(&screencapturekit::cm::CMTime::new(1, settings.fps as i32));

    let shared = Arc::new(Shared {
        settings,
        tx,
        info,
        encoder: Mutex::new(None),
        last: Mutex::new(None),
        start: Instant::now(),
        key_request,
    });

    let mut stream = SCStream::new(&filter, &config).map_err(|e| anyhow!("{e:?}"))?;
    let _ = stream.add_output_handler(Handler(shared.clone()), SCStreamOutputType::Screen);
    stream.start_capture().map_err(|e| anyhow!("{e:?}"))?;

    // Serves keyframe requests when the screen is static (no new frames arriving).
    let stop = Arc::new(AtomicBool::new(false));
    {
        let (shared, stop) = (shared.clone(), stop.clone());
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(30));
                if !shared.key_request.load(Ordering::SeqCst) {
                    continue;
                }
                let last = shared.last.lock().unwrap();
                if let Some(held) = last.as_ref() {
                    if shared.key_request.swap(false, Ordering::SeqCst) {
                        shared.encode(held.0, true);
                    }
                }
            }
        });
    }

    Ok(Capture { _stream: stream, stop })
}
