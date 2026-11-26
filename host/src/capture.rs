//! ScreenCaptureKit capture of one display, feeding the encoder.

use crate::encoder::{Encoder, Packet};
use anyhow::{anyhow, Result};
use screencapturekit::cm::CMSampleBufferExt;
use screencapturekit::prelude::*;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub struct Settings {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_bps: u32,
}

struct Handler {
    settings: Settings,
    tx: Sender<Packet>,
    encoder: Mutex<Option<Arc<Encoder>>>,
    info: Sender<String>,
}

impl SCStreamOutputTrait for Handler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, kind: SCStreamOutputType) {
        if !matches!(kind, SCStreamOutputType::Screen) {
            return;
        }
        let Some(pb) = sample.pixel_buffer() else { return };

        let mut guard = self.encoder.lock().unwrap();
        if guard.is_none() {
            // Size the encoder from the real frame so it always matches what SCK delivers.
            let (w, h) = (pb.width() as u32, pb.height() as u32);
            let _ = self.info.send(format!("first frame: {w}x{h}"));
            match Encoder::new(w, h, self.settings.fps, self.settings.bitrate_bps, self.tx.clone()) {
                Ok(e) => *guard = Some(Arc::new(e)),
                Err(e) => {
                    let _ = self.info.send(format!("encoder init failed: {e}"));
                    return;
                }
            }
        }
        let enc = guard.as_ref().unwrap();
        let ts = sample.presentation_timestamp();
        if let Err(e) = enc.encode(pb.as_ptr(), ts.value, ts.timescale, false) {
            let _ = self.info.send(format!("encode error: {e}"));
        }
    }
}

pub fn start(display_id: u32, settings: Settings, tx: Sender<Packet>, info: Sender<String>) -> Result<SCStream> {
    let content = SCShareableContent::get().map_err(|e| anyhow!("SCShareableContent: {e:?} (Screen Recording permission?)"))?;
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

    let handler = Handler { settings, tx, encoder: Mutex::new(None), info };
    let mut stream = SCStream::new(&filter, &config).map_err(|e| anyhow!("{e:?}"))?;
    let _ = stream.add_output_handler(handler, SCStreamOutputType::Screen);
    stream.start_capture().map_err(|e| anyhow!("{e:?}"))?;
    Ok(stream)
}
