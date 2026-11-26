//! H.264 / HEVC hardware encoder (VideoToolbox) producing Annex-B packets.

use anyhow::{anyhow, Result};
use std::ffi::c_void;
use std::sync::mpsc::Sender;

type CFTypeRef = *const c_void;
type OSStatus = i32;

#[repr(C)]
#[derive(Clone, Copy)]
struct CMTime {
    value: i64,
    timescale: i32,
    flags: u32,
    epoch: i64,
}

const K_CM_VIDEO_CODEC_H264: u32 = 0x6176_6331; // 'avc1'
const K_CM_VIDEO_CODEC_HEVC: u32 = 0x6876_6331; // 'hvc1'

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Codec {
    H264,
    Hevc,
}

impl Codec {
    /// Id used in the protocol hello.
    pub fn id(self) -> u8 {
        match self {
            Codec::H264 => crate::protocol::CODEC_H264,
            Codec::Hevc => crate::protocol::CODEC_HEVC,
        }
    }
}
const K_CF_NUMBER_SINT32: isize = 3;

type OutputCallback = extern "C" fn(*mut c_void, *mut c_void, OSStatus, u32, *const c_void);

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFBooleanTrue: CFTypeRef;
    static kCFBooleanFalse: CFTypeRef;
    static kCFTypeDictionaryKeyCallBacks: u8;
    static kCFTypeDictionaryValueCallBacks: u8;
    fn CFNumberCreate(alloc: CFTypeRef, ty: isize, value: *const c_void) -> CFTypeRef;
    fn CFRelease(cf: CFTypeRef);
    fn CFDictionaryCreate(
        alloc: CFTypeRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        n: isize,
        key_cb: *const c_void,
        val_cb: *const c_void,
    ) -> CFTypeRef;
    fn CFDictionaryContainsKey(dict: CFTypeRef, key: CFTypeRef) -> u8;
    fn CFArrayGetValueAtIndex(arr: CFTypeRef, idx: isize) -> CFTypeRef;
}

#[link(name = "CoreMedia", kind = "framework")]
extern "C" {
    static kCMSampleAttachmentKey_NotSync: CFTypeRef;
    fn CMSampleBufferGetSampleAttachmentsArray(sb: *const c_void, create: u8) -> CFTypeRef;
    fn CMSampleBufferGetPresentationTimeStamp(sb: *const c_void) -> CMTime;
    fn CMSampleBufferGetFormatDescription(sb: *const c_void) -> *const c_void;
    fn CMSampleBufferGetDataBuffer(sb: *const c_void) -> *const c_void;
    fn CMFormatDescriptionGetMediaSubType(desc: *const c_void) -> u32;
    fn CMVideoFormatDescriptionGetHEVCParameterSetAtIndex(
        desc: *const c_void,
        idx: usize,
        out: *mut *const u8,
        size: *mut usize,
        count: *mut usize,
        nal_header_len: *mut i32,
    ) -> OSStatus;
    fn CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
        desc: *const c_void,
        idx: usize,
        out: *mut *const u8,
        size: *mut usize,
        count: *mut usize,
        nal_header_len: *mut i32,
    ) -> OSStatus;
    fn CMBlockBufferGetDataPointer(
        bb: *const c_void,
        offset: usize,
        len_at_offset: *mut usize,
        total: *mut usize,
        out: *mut *const u8,
    ) -> OSStatus;
}

#[link(name = "VideoToolbox", kind = "framework")]
extern "C" {
    static kVTCompressionPropertyKey_RealTime: CFTypeRef;
    static kVTCompressionPropertyKey_AllowFrameReordering: CFTypeRef;
    static kVTCompressionPropertyKey_ProfileLevel: CFTypeRef;
    static kVTCompressionPropertyKey_AverageBitRate: CFTypeRef;
    static kVTCompressionPropertyKey_MaxKeyFrameInterval: CFTypeRef;
    static kVTCompressionPropertyKey_ExpectedFrameRate: CFTypeRef;
    static kVTCompressionPropertyKey_ReferenceBufferCount: CFTypeRef;
    static kVTProfileLevel_H264_High_AutoLevel: CFTypeRef;
    static kVTProfileLevel_HEVC_Main_AutoLevel: CFTypeRef;
    static kVTVideoEncoderSpecification_EnableLowLatencyRateControl: CFTypeRef;
    static kVTEncodeFrameOptionKey_ForceKeyFrame: CFTypeRef;

    fn VTCompressionSessionCreate(
        alloc: CFTypeRef,
        width: i32,
        height: i32,
        codec: u32,
        encoder_spec: CFTypeRef,
        source_attrs: CFTypeRef,
        compressed_alloc: CFTypeRef,
        callback: OutputCallback,
        refcon: *mut c_void,
        out: *mut *mut c_void,
    ) -> OSStatus;
    fn VTSessionSetProperty(session: *mut c_void, key: CFTypeRef, value: CFTypeRef) -> OSStatus;
    fn VTCompressionSessionPrepareToEncodeFrames(session: *mut c_void) -> OSStatus;
    fn VTCompressionSessionEncodeFrame(
        session: *mut c_void,
        image: *const c_void,
        pts: CMTime,
        duration: CMTime,
        frame_props: CFTypeRef,
        source_refcon: *mut c_void,
        info_flags: *mut u32,
    ) -> OSStatus;
    fn VTCompressionSessionInvalidate(session: *mut c_void);
}

/// Per-frame bookkeeping handed through VideoToolbox to the output callback.
struct FrameCtx {
    repeat: bool,
    submitted: std::time::Instant,
}

/// Sum (microseconds) and count of encoder latency for real frames (diagnostics).
pub static ENCODE_US_SUM: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static ENCODE_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Packets produced by the encoder (for diagnostics).
pub static PACKETS_ENCODED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub struct Packet {
    pub keyframe: bool,
    /// Re-encode of the previous picture (decoder flush), not new content.
    pub repeat: bool,
    pub pts_us: i64,
    /// Annex-B (start-code) NAL units; keyframes are prefixed with SPS+PPS.
    pub data: Vec<u8>,
}

pub struct Encoder {
    session: *mut c_void,
    tx: *mut Sender<Packet>,
}

// VTCompressionSession is thread-safe; the sender box is only read by the callback.
unsafe impl Send for Encoder {}
unsafe impl Sync for Encoder {}

fn check(status: OSStatus, what: &str) -> Result<()> {
    if status == 0 { Ok(()) } else { Err(anyhow!("{what} failed: OSStatus {status}")) }
}

unsafe fn cf_i32(v: i32) -> CFTypeRef {
    CFNumberCreate(std::ptr::null(), K_CF_NUMBER_SINT32, &v as *const i32 as *const c_void)
}

unsafe fn set_i32(session: *mut c_void, key: CFTypeRef, v: i32, what: &str) -> Result<()> {
    let n = cf_i32(v);
    let st = VTSessionSetProperty(session, key, n);
    CFRelease(n);
    check(st, what)
}

impl Encoder {
    pub fn new(width: u32, height: u32, fps: u32, bitrate_bps: u32, low_latency_rc: bool, codec: Codec, tx: Sender<Packet>) -> Result<Self> {
        unsafe {
            let spec_keys = [kVTVideoEncoderSpecification_EnableLowLatencyRateControl];
            let spec_vals = [kCFBooleanTrue];
            let spec = CFDictionaryCreate(
                std::ptr::null(),
                spec_keys.as_ptr(),
                spec_vals.as_ptr(),
                1,
                &kCFTypeDictionaryKeyCallBacks as *const u8 as *const c_void,
                &kCFTypeDictionaryValueCallBacks as *const u8 as *const c_void,
            );

            let tx = Box::into_raw(Box::new(tx));
            let mut session: *mut c_void = std::ptr::null_mut();
            let st = VTCompressionSessionCreate(
                std::ptr::null(),
                width as i32,
                height as i32,
                match codec {
                    Codec::H264 => K_CM_VIDEO_CODEC_H264,
                    Codec::Hevc => K_CM_VIDEO_CODEC_HEVC,
                },
                if low_latency_rc { spec } else { std::ptr::null() },
                std::ptr::null(),
                std::ptr::null(),
                output_callback,
                tx as *mut c_void,
                &mut session,
            );
            CFRelease(spec);
            if let Err(e) = check(st, "VTCompressionSessionCreate") {
                drop(Box::from_raw(tx));
                return Err(e);
            }

            let enc = Self { session, tx };
            check(VTSessionSetProperty(session, kVTCompressionPropertyKey_RealTime, kCFBooleanTrue), "RealTime")?;
            check(
                VTSessionSetProperty(session, kVTCompressionPropertyKey_AllowFrameReordering, kCFBooleanFalse),
                "AllowFrameReordering",
            )?;
            check(
                VTSessionSetProperty(
                    session,
                    kVTCompressionPropertyKey_ProfileLevel,
                    match codec {
                        Codec::H264 => kVTProfileLevel_H264_High_AutoLevel,
                        Codec::Hevc => kVTProfileLevel_HEVC_Main_AutoLevel,
                    },
                ),
                "ProfileLevel",
            )?;
            set_i32(session, kVTCompressionPropertyKey_AverageBitRate, bitrate_bps as i32, "AverageBitRate")?;
            set_i32(session, kVTCompressionPropertyKey_ExpectedFrameRate, fps as i32, "ExpectedFrameRate")?;
            set_i32(session, kVTCompressionPropertyKey_MaxKeyFrameInterval, (fps * 2) as i32, "MaxKeyFrameInterval")?;
            // By default VideoToolbox keeps up to 11 reference frames, which makes hardware
            // decoders buffer ~20+ frames before output (hundreds of ms of lag).
            // Only supported together with the low-latency rate control; ignore otherwise.
            let _ = set_i32(session, kVTCompressionPropertyKey_ReferenceBufferCount, 1, "ReferenceBufferCount");
            check(VTCompressionSessionPrepareToEncodeFrames(session), "PrepareToEncodeFrames")?;
            Ok(enc)
        }
    }

    /// `pixel_buffer` is a CVPixelBufferRef, borrowed for the duration of the call.
    pub fn encode(&self, pixel_buffer: *const c_void, pts_value: i64, pts_timescale: i32, force_key: bool, repeat: bool) -> Result<()> {
        unsafe {
            let pts = CMTime { value: pts_value, timescale: pts_timescale, flags: 1, epoch: 0 };
            let invalid = CMTime { value: 0, timescale: 0, flags: 0, epoch: 0 };
            let props = if force_key {
                let keys = [kVTEncodeFrameOptionKey_ForceKeyFrame];
                let vals = [kCFBooleanTrue];
                CFDictionaryCreate(
                    std::ptr::null(),
                    keys.as_ptr(),
                    vals.as_ptr(),
                    1,
                    &kCFTypeDictionaryKeyCallBacks as *const u8 as *const c_void,
                    &kCFTypeDictionaryValueCallBacks as *const u8 as *const c_void,
                )
            } else {
                std::ptr::null()
            };
            let mut flags = 0u32;
            let ctx = Box::into_raw(Box::new(FrameCtx { repeat, submitted: std::time::Instant::now() }));
            let st = VTCompressionSessionEncodeFrame(
                self.session,
                pixel_buffer,
                pts,
                invalid,
                props,
                ctx as *mut c_void,
                &mut flags,
            );
            if st != 0 {
                drop(Box::from_raw(ctx)); // the callback will never run for this frame
            }
            if !props.is_null() {
                CFRelease(props);
            }
            check(st, "EncodeFrame")
        }
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe {
            VTCompressionSessionInvalidate(self.session);
            CFRelease(self.session as CFTypeRef);
            drop(Box::from_raw(self.tx));
        }
    }
}

extern "C" fn output_callback(
    refcon: *mut c_void,
    frame_refcon: *mut c_void,
    status: OSStatus,
    _flags: u32,
    sb: *const c_void,
) {
    let ctx = (!frame_refcon.is_null()).then(|| unsafe { Box::from_raw(frame_refcon as *mut FrameCtx) });
    if status != 0 || sb.is_null() {
        return;
    }
    unsafe {
        let tx = &*(refcon as *const Sender<Packet>);
        let mut keyframe = true;
        let attachments = CMSampleBufferGetSampleAttachmentsArray(sb, 0);
        if !attachments.is_null() {
            let dict = CFArrayGetValueAtIndex(attachments, 0);
            if !dict.is_null() && CFDictionaryContainsKey(dict, kCMSampleAttachmentKey_NotSync) != 0 {
                keyframe = false;
            }
        }

        let mut out = Vec::new();
        if keyframe {
            // Parameter sets: SPS+PPS for H.264, VPS+SPS+PPS (+SEI) for HEVC.
            let fmt = CMSampleBufferGetFormatDescription(sb);
            let hevc = CMFormatDescriptionGetMediaSubType(fmt) == K_CM_VIDEO_CODEC_HEVC;
            let mut i = 0;
            let mut count = 1usize;
            while i < count {
                let (mut p, mut size, mut nal) = (std::ptr::null(), 0usize, 0i32);
                let st = if hevc {
                    CMVideoFormatDescriptionGetHEVCParameterSetAtIndex(fmt, i, &mut p, &mut size, &mut count, &mut nal)
                } else {
                    CMVideoFormatDescriptionGetH264ParameterSetAtIndex(fmt, i, &mut p, &mut size, &mut count, &mut nal)
                };
                if st != 0 {
                    break;
                }
                out.extend_from_slice(&[0, 0, 0, 1]);
                out.extend_from_slice(std::slice::from_raw_parts(p, size));
                i += 1;
            }
        }

        let bb = CMSampleBufferGetDataBuffer(sb);
        let (mut at, mut total, mut ptr) = (0usize, 0usize, std::ptr::null());
        if bb.is_null() || CMBlockBufferGetDataPointer(bb, 0, &mut at, &mut total, &mut ptr) != 0 || at != total {
            return;
        }
        let data = std::slice::from_raw_parts(ptr, total);
        let mut i = 0;
        while i + 4 <= data.len() {
            let n = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
            i += 4;
            if i + n > data.len() {
                break;
            }
            out.extend_from_slice(&[0, 0, 0, 1]);
            out.extend_from_slice(&data[i..i + n]);
            i += n;
        }
        let pts = CMSampleBufferGetPresentationTimeStamp(sb);
        let pts_us = if pts.timescale > 0 { pts.value * 1_000_000 / pts.timescale as i64 } else { 0 };
        PACKETS_ENCODED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let repeat = ctx.as_ref().is_some_and(|c| c.repeat);
        if let Some(c) = ctx.as_ref().filter(|c| !c.repeat) {
            use std::sync::atomic::Ordering::Relaxed;
            ENCODE_US_SUM.fetch_add(c.submitted.elapsed().as_micros() as u64, Relaxed);
            ENCODE_COUNT.fetch_add(1, Relaxed);
        }
        let _ = tx.send(Packet { keyframe, repeat, pts_us, data: out });
    }
}
