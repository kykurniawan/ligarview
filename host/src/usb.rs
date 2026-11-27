//! USB transport over Android Open Accessory (AOA): no USB debugging, no adb.
//!
//! The host watches for Android devices, switches them into accessory mode and talks to the
//! Ligarview app over one bulk pipe. Wire format on the pipe, both directions:
//!
//!   channel u8 | payload_len u16 (big-endian) | payload
//!
//! Channels: 0 control, 1 video, 2 cursor, 3 input (app to host, one message per frame). The video and cursor channels carry exactly the
//! byte streams that the TCP transport uses. Frames are at most 16000 bytes, so a small cursor
//! message can slip in between the pieces of a large video frame.
//!
//! Control (payload byte 0): from the app, 1 = RESUME (start or restart streaming, also sent
//! when the app opens the accessory), 2 = PAUSE. From the host, 0 = SYNC (followed by the magic
//! below), sent before streaming starts so the app can find a frame boundary, and 1 = ASK,
//! sent while the app has not said anything yet, so that an app that is already waiting for a
//! stream (the host was restarted) asks again.

use crate::cursor_server::CursorHub;
use crate::server::VideoHub;
use anyhow::{anyhow, Result};
use rusb::{request_type, Context, Device, DeviceHandle, Direction, Recipient, RequestType, UsbContext};
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CH_CONTROL: u8 = 0;
const CH_VIDEO: u8 = 1;
const CH_CURSOR: u8 = 2;
const CH_INPUT: u8 = 3;

const CTL_RESUME: u8 = 1;
const CTL_PAUSE: u8 = 2;
const CTL_ASK: u8 = 1;

/// Largest frame on the pipe. Not a multiple of 512, see `MuxWriter::write`.
const MAX_FRAME: usize = 16000;
const MAX_PAYLOAD: usize = MAX_FRAME - 3;
const SYNC_MAGIC: &[u8; 16] = b"LGRV-SYNC-v1\0\0\0\0";

const CONTROL_TIMEOUT: Duration = Duration::from_millis(2000);
const WRITE_TIMEOUT: Duration = Duration::from_millis(3000);

const GOOGLE_VID: u16 = 0x18d1;
const ACCESSORY_PIDS: [u16; 2] = [0x2d00, 0x2d01]; // accessory, accessory + adb

/// Identification sent to the device; must match android/app/src/main/res/xml/accessory_filter.xml.
const ACCESSORY_STRINGS: [&str; 6] = [
    "Ligarview",
    "Ligarview",
    "Ligarview display",
    "1",
    "https://github.com/",
    "0001",
];

/// Vendor ids of common Android makers; only these are probed for AOA support.
const ANDROID_VENDORS: &[u16] = &[
    0x04e8, // Samsung
    0x18d1, // Google
    0x2717, // Xiaomi
    0x12d1, // Huawei
    0x22d9, // Oppo / Realme
    0x2a70, // OnePlus
    0x22b8, // Motorola
    0x1004, // LG
    0x0fce, // Sony
    0x0bb4, // HTC
    0x0b05, // Asus
    0x17ef, // Lenovo
    0x0955, // Nvidia
    0x19d2, // ZTE
    0x2b0e, // Nothing
    0x05c6, // Qualcomm reference devices
    0x0e8d, // MediaTek
];

fn is_accessory(d: &Device<Context>) -> bool {
    d.device_descriptor()
        .map(|dd| dd.vendor_id() == GOOGLE_VID && ACCESSORY_PIDS.contains(&dd.product_id()))
        .unwrap_or(false)
}

fn is_candidate(d: &Device<Context>) -> bool {
    !is_accessory(d)
        && d.device_descriptor().map(|dd| ANDROID_VENDORS.contains(&dd.vendor_id())).unwrap_or(false)
}

fn id_of(d: &Device<Context>) -> String {
    d.device_descriptor()
        .map(|dd| format!("{:04x}:{:04x}", dd.vendor_id(), dd.product_id()))
        .unwrap_or_default()
}

/// Asks a device for its AOA protocol version; only devices that support it answer.
fn aoa_version(h: &DeviceHandle<Context>) -> Option<u16> {
    let mut buf = [0u8; 2];
    let rt = request_type(Direction::In, RequestType::Vendor, Recipient::Device);
    h.read_control(rt, 51, 0, 0, &mut buf, CONTROL_TIMEOUT).ok()?;
    Some(u16::from_le_bytes(buf))
}

fn switch_to_accessory(h: &DeviceHandle<Context>) -> rusb::Result<()> {
    let rt = request_type(Direction::Out, RequestType::Vendor, Recipient::Device);
    for (index, s) in ACCESSORY_STRINGS.iter().enumerate() {
        let mut bytes = s.as_bytes().to_vec();
        bytes.push(0);
        h.write_control(rt, 52, 0, index as u16, &bytes, CONTROL_TIMEOUT)?;
    }
    h.write_control(rt, 53, 0, 0, &[], CONTROL_TIMEOUT)?;
    Ok(())
}

struct Link {
    handle: DeviceHandle<Context>,
    ep_out: u8,
    ep_in: u8,
    write_lock: Mutex<()>,
    dead: AtomicBool,
    /// True once the app has sent any control frame.
    heard: AtomicBool,
    /// Id of the video client this link attached, if any.
    video_client: Mutex<Option<u64>>,
    /// Stop flag of the cursor client currently attached, if any.
    cursor_stop: Mutex<Option<Arc<AtomicBool>>>,
}

impl Link {
    fn write_all(&self, mut data: &[u8]) -> io::Result<()> {
        while !data.is_empty() {
            match self.handle.write_bulk(self.ep_out, data, WRITE_TIMEOUT) {
                Ok(n) => data = &data[n..],
                Err(e) => {
                    self.dead.store(true, Ordering::SeqCst);
                    return Err(io::Error::new(io::ErrorKind::BrokenPipe, e.to_string()));
                }
            }
        }
        Ok(())
    }

    /// Sends a control frame that may be lost: while the app is not running nothing reads the
    /// pipe and the write times out, which must not be treated as a broken link.
    fn try_write_frame(&self, channel: u8, payload: &[u8]) {
        let mut frame = Vec::with_capacity(3 + payload.len());
        frame.push(channel);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        frame.extend_from_slice(payload);
        let _guard = self.write_lock.lock().unwrap();
        match self.handle.write_bulk(self.ep_out, &frame, Duration::from_millis(300)) {
            Ok(_) | Err(rusb::Error::Timeout) => {}
            Err(_) => self.dead.store(true, Ordering::SeqCst),
        }
    }

    fn write_frame(&self, channel: u8, payload: &[u8]) -> io::Result<()> {
        if self.dead.load(Ordering::SeqCst) {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "usb link closed"));
        }
        let mut frame = Vec::with_capacity(3 + payload.len());
        frame.push(channel);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        frame.extend_from_slice(payload);
        let _guard = self.write_lock.lock().unwrap();
        self.write_all(&frame)
    }
}

/// One logical channel of the pipe, written like a socket.
struct MuxWriter {
    link: Arc<Link>,
    channel: u8,
}

impl Write for MuxWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut rest = buf;
        while !rest.is_empty() {
            let mut n = rest.len().min(MAX_PAYLOAD);
            // A bulk transfer that ends exactly on a packet boundary (512 bytes at high speed)
            // needs a zero-length packet to be seen as finished; avoid that case altogether.
            if (n + 3) % 512 == 0 {
                n -= 1;
            }
            self.link.write_frame(self.channel, &rest[..n])?;
            rest = &rest[n..];
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct Hubs {
    video: Arc<VideoHub>,
    cursor: Option<Arc<CursorHub>>,
}

impl Hubs {
    /// Detaches only the clients this link attached (a TCP client must survive a USB link ending).
    fn detach(&self, link: &Link) {
        if let Some(id) = link.video_client.lock().unwrap().take() {
            self.video.detach(id);
        }
        if let Some(stop) = link.cursor_stop.lock().unwrap().take() {
            stop.store(true, Ordering::SeqCst);
        }
    }

    /// (Re)starts streaming: SYNC first so the app can find a frame boundary, then the clients.
    fn attach(&self, link: &Arc<Link>) {
        self.detach(link);
        let mut sync = vec![CH_CONTROL];
        sync.extend_from_slice(&(SYNC_MAGIC.len() as u16 + 1).to_be_bytes());
        sync.push(0);
        sync.extend_from_slice(SYNC_MAGIC);
        {
            let _guard = link.write_lock.lock().unwrap();
            if link.write_all(&sync).is_err() {
                return;
            }
        }
        let id = self.video.attach(MuxWriter { link: link.clone(), channel: CH_VIDEO }, "usb".into());
        *link.video_client.lock().unwrap() = Some(id);
        if let Some(cursor) = &self.cursor {
            let stop = cursor.attach(MuxWriter { link: link.clone(), channel: CH_CURSOR });
            *link.cursor_stop.lock().unwrap() = Some(stop);
        }
    }
}

/// Reads frames from the app; only control frames matter.
fn reader(link: Arc<Link>, hubs: Arc<Hubs>) {
    let mut buf = vec![0u8; 16384];
    let mut acc: Vec<u8> = Vec::new();
    while !link.dead.load(Ordering::SeqCst) {
        match link.handle.read_bulk(link.ep_in, &mut buf, Duration::from_millis(500)) {
            Ok(n) => {
                acc.extend_from_slice(&buf[..n]);
                while acc.len() >= 3 {
                    let len = u16::from_be_bytes([acc[1], acc[2]]) as usize;
                    if acc.len() < 3 + len {
                        break;
                    }
                    if acc[0] == CH_CONTROL && len >= 1 {
                        link.heard.store(true, Ordering::SeqCst);
                        match acc[3] {
                            CTL_RESUME => {
                                println!("usb: app asked for the stream");
                                hubs.attach(&link);
                            }
                            CTL_PAUSE => {
                                println!("usb: app paused the stream");
                                hubs.detach(&link);
                            }
                            _ => {}
                        }
                    }
                    if acc[0] == CH_INPUT {
                        crate::input::handle(&acc[3..3 + len]);
                    }
                    acc.drain(..3 + len);
                }
            }
            Err(rusb::Error::Timeout) => {}
            Err(_) => {
                link.dead.store(true, Ordering::SeqCst);
            }
        }
    }
    hubs.detach(&link);
    crate::input::release_all();
}

/// Talks to a device that is already in accessory mode. Returns when the link ends.
fn run_link(device: Device<Context>, hubs: &Arc<Hubs>) -> Result<()> {
    let handle = device.open().map_err(|e| anyhow!("cannot open the accessory device: {e} (is adb holding it? try `adb kill-server`)"))?;
    let cfg = device.active_config_descriptor()?;
    let (mut ep_in, mut ep_out, mut iface_no) = (None, None, 0);
    for iface in cfg.interfaces() {
        for desc in iface.descriptors() {
            for ep in desc.endpoint_descriptors() {
                if ep.transfer_type() == rusb::TransferType::Bulk {
                    iface_no = desc.interface_number();
                    match ep.direction() {
                        Direction::In => ep_in = Some(ep.address()),
                        Direction::Out => ep_out = Some(ep.address()),
                    }
                }
            }
        }
        if ep_in.is_some() && ep_out.is_some() {
            break;
        }
    }
    let (ep_in, ep_out) = (
        ep_in.ok_or_else(|| anyhow!("accessory has no bulk IN endpoint"))?,
        ep_out.ok_or_else(|| anyhow!("accessory has no bulk OUT endpoint"))?,
    );
    handle.claim_interface(iface_no)?;
    println!("usb: accessory mode, waiting for the Ligarview app (install it and accept the prompt on the tablet)");

    let link = Arc::new(Link {
        handle,
        ep_out,
        ep_in,
        write_lock: Mutex::new(()),
        dead: AtomicBool::new(false),
        heard: AtomicBool::new(false),
        video_client: Mutex::new(None),
        cursor_stop: Mutex::new(None),
    });
    let reader_link = link.clone();
    let reader_hubs = hubs.clone();
    std::thread::spawn(move || reader(reader_link, reader_hubs));

    let mut last_ask = Instant::now() - Duration::from_secs(10);
    while !link.dead.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(300));
        if !link.heard.load(Ordering::SeqCst) && last_ask.elapsed() >= Duration::from_secs(2) {
            last_ask = Instant::now();
            link.try_write_frame(CH_CONTROL, &[CTL_ASK]);
        }
    }
    println!("usb: link closed");
    Ok(())
}

/// Watches for Android devices and keeps a USB link up for as long as one is attached.
pub fn spawn(video: Arc<VideoHub>, cursor: Option<Arc<CursorHub>>) {
    let hubs = Arc::new(Hubs { video, cursor });
    std::thread::spawn(move || {
        let ctx = match Context::new() {
            Ok(c) => c,
            Err(e) => {
                println!("usb: libusb unavailable: {e}");
                return;
            }
        };
        let mut switched: HashMap<(u8, u8), Instant> = HashMap::new();
        let mut warned: HashSet<(u8, u8)> = HashSet::new();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let Ok(list) = ctx.devices() else { continue };

            if let Some(dev) = list.iter().find(is_accessory) {
                if let Err(e) = run_link(dev, &hubs) {
                    println!("usb: {e}");
                    std::thread::sleep(Duration::from_secs(3));
                }
                continue;
            }

            for dev in list.iter().filter(is_candidate) {
                let key = (dev.bus_number(), dev.address());
                if switched.get(&key).is_some_and(|t| t.elapsed() < Duration::from_secs(15)) {
                    continue;
                }
                match dev.open() {
                    Err(e) => {
                        if warned.insert(key) {
                            println!("usb: cannot open {} ({e}); if adb is running try `adb kill-server`", id_of(&dev));
                        }
                    }
                    Ok(h) => {
                        if let Some(v) = aoa_version(&h) {
                            println!("usb: Android device {} supports AOA v{v}, switching to accessory mode", id_of(&dev));
                            match switch_to_accessory(&h) {
                                Ok(()) => {
                                    switched.insert(key, Instant::now());
                                }
                                Err(e) => println!("usb: could not switch {}: {e}", id_of(&dev)),
                            }
                        }
                    }
                }
            }
        }
    });
}
