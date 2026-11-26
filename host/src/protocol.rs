//! Wire protocol (all integers big-endian).
//!
//! Host -> client, once on connect (12 bytes):
//!   magic "LGRV" | version u8 | codec u8 (1 = H.264, 2 = HEVC) | width u16 | height u16 | fps u16
//! Then a stream of frames:
//!   payload_len u32 | flags u8 | pts_us u64 | payload (Annex-B H.264 or HEVC)
//! flags bit 0 = keyframe (payload starts with the parameter sets - SPS+PPS, or VPS+SPS+PPS
//!               for HEVC - then the IDR picture).
//! flags bit 1 = repeat: re-encode of the previous picture, sent only to flush the
//!               decoder's output queue; clients may ignore it for statistics.

use std::io::{self, Write};

pub const MAGIC: &[u8; 4] = b"LGRV";
pub const VERSION: u8 = 1;
pub const CODEC_H264: u8 = 1;
pub const CODEC_HEVC: u8 = 2;
pub const FLAG_KEYFRAME: u8 = 1;
pub const FLAG_REPEAT: u8 = 2;

#[derive(Clone, Copy)]
pub struct Hello {
    pub codec: u8,
    pub width: u16,
    pub height: u16,
    pub fps: u16,
}

pub fn write_hello(w: &mut impl Write, h: Hello) -> io::Result<()> {
    let mut b = Vec::with_capacity(12);
    b.extend_from_slice(MAGIC);
    b.push(VERSION);
    b.push(h.codec);
    b.extend_from_slice(&h.width.to_be_bytes());
    b.extend_from_slice(&h.height.to_be_bytes());
    b.extend_from_slice(&h.fps.to_be_bytes());
    w.write_all(&b)
}

pub fn write_frame(w: &mut impl Write, flags: u8, pts_us: i64, payload: &[u8]) -> io::Result<()> {
    let mut head = [0u8; 13];
    head[0..4].copy_from_slice(&(payload.len() as u32).to_be_bytes());
    head[4] = flags;
    head[5..13].copy_from_slice(&(pts_us as u64).to_be_bytes());
    w.write_all(&head)?;
    w.write_all(payload)
}
