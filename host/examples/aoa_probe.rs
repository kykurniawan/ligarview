//! Experiment: switch an Android device into accessory mode (AOA) and measure the USB link.
//!
//!   cargo run --release --example aoa_probe
//!
//! No USB debugging is needed. The Android side is `AccessoryProbeActivity`.

use rusb::{request_type, Context, Device, DeviceHandle, Direction, Recipient, RequestType, UsbContext};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_millis(2000);
const GOOGLE_VID: u16 = 0x18d1;
const ACCESSORY_PIDS: [u16; 2] = [0x2d00, 0x2d01]; // accessory, accessory + adb

// Accessory identification, must match android/app/src/main/res/xml/accessory_filter.xml
const MANUFACTURER: &str = "Ligarview";
const MODEL: &str = "Ligarview";
const DESCRIPTION: &str = "Ligarview display";
const VERSION: &str = "1";
const URI: &str = "https://github.com/";
const SERIAL: &str = "0001";

fn is_accessory(d: &Device<Context>) -> bool {
    d.device_descriptor()
        .map(|dd| dd.vendor_id() == GOOGLE_VID && ACCESSORY_PIDS.contains(&dd.product_id()))
        .unwrap_or(false)
}

/// Asks a device for its AOA protocol version; only Android devices answer.
fn aoa_version(h: &DeviceHandle<Context>) -> Option<u16> {
    let mut buf = [0u8; 2];
    let rt = request_type(Direction::In, RequestType::Vendor, Recipient::Device);
    h.read_control(rt, 51, 0, 0, &mut buf, TIMEOUT).ok()?;
    Some(u16::from_le_bytes(buf))
}

fn switch_to_accessory(h: &DeviceHandle<Context>) -> rusb::Result<()> {
    let rt = request_type(Direction::Out, RequestType::Vendor, Recipient::Device);
    for (index, s) in [MANUFACTURER, MODEL, DESCRIPTION, VERSION, URI, SERIAL].iter().enumerate() {
        let mut bytes = s.as_bytes().to_vec();
        bytes.push(0);
        h.write_control(rt, 52, 0, index as u16, &bytes, TIMEOUT)?;
    }
    h.write_control(rt, 53, 0, 0, &[], TIMEOUT)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let ctx = Context::new()?;

    // 1. Already in accessory mode? Otherwise find an Android device and switch it.
    let mut accessory = ctx.devices()?.iter().find(is_accessory);
    if accessory.is_none() {
        println!("USB devices:");
        let mut switched = false;
        for d in ctx.devices()?.iter() {
            let dd = d.device_descriptor()?;
            print!("  {:04x}:{:04x} class {:02x}", dd.vendor_id(), dd.product_id(), dd.class_code());
            if dd.class_code() == 9 || dd.vendor_id() == 0x05ac {
                println!("  (hub/Apple, skipped)");
                continue;
            }
            match d.open() {
                Err(e) => println!("  cannot open: {e}"),
                Ok(h) => match aoa_version(&h) {
                    None => println!("  not an AOA device"),
                    Some(v) => {
                        println!("  AOA protocol v{v} -> switching to accessory mode");
                        switch_to_accessory(&h)?;
                        switched = true;
                    }
                },
            }
        }
        if !switched {
            anyhow::bail!("no Android device answered the AOA query (is `adb` holding it? try `adb kill-server`)");
        }
        println!("waiting for the device to re-enumerate in accessory mode...");
        let t0 = Instant::now();
        while accessory.is_none() && t0.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(250));
            accessory = ctx.devices()?.iter().find(is_accessory);
        }
    }
    let device = accessory.ok_or_else(|| anyhow::anyhow!("device did not re-enumerate as an accessory"))?;
    let dd = device.device_descriptor()?;
    println!("accessory device found: {:04x}:{:04x}", dd.vendor_id(), dd.product_id());

    // 2. Open it and find the bulk endpoints.
    let mut handle = device.open()?;
    let cfg = device.active_config_descriptor()?;
    let (mut ep_in, mut ep_out) = (None, None);
    let mut iface_no = 0;
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
    let (ep_in, ep_out) = (ep_in.ok_or_else(|| anyhow::anyhow!("no bulk IN"))?, ep_out.ok_or_else(|| anyhow::anyhow!("no bulk OUT"))?);
    println!("interface {iface_no}: bulk IN 0x{ep_in:02x}, bulk OUT 0x{ep_out:02x}");
    handle.claim_interface(iface_no)?;
    println!("interface claimed. Waiting for the app to open the accessory (tap OK on the tablet if asked)...");

    // 3. The app takes a moment (permission dialog, activity start): ping until it answers.
    let mut buf = vec![0u8; 16384];
    let t0 = Instant::now();
    loop {
        let mut m = vec![b'P'];
        m.extend_from_slice(&0u64.to_be_bytes());
        if handle.write_bulk(ep_out, &m, Duration::from_millis(500)).is_ok() {
            if let Ok(n) = handle.read_bulk(ep_in, &mut buf, Duration::from_millis(500)) {
                if n == 9 && buf[0] == b'P' {
                    break;
                }
            }
        }
        if t0.elapsed() > Duration::from_secs(60) {
            anyhow::bail!("the app never answered");
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    println!("the app answered after {:.1} s", t0.elapsed().as_secs_f64());

    // 4. Latency: small round trips.
    let mut rtts = Vec::new();
    for i in 0..500u64 {
        let mut m = vec![b'P'];
        m.extend_from_slice(&i.to_be_bytes());
        let t = Instant::now();
        handle.write_bulk(ep_out, &m, TIMEOUT)?;
        let n = handle.read_bulk(ep_in, &mut buf, TIMEOUT)?;
        assert!(n == 9 && buf[0] == b'P');
        rtts.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    rtts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pct = |p: f64| rtts[((rtts.len() - 1) as f64 * p) as usize];
    println!("round trip (500 pings): min {:.2} ms, median {:.2} ms, p95 {:.2} ms, p99 {:.2} ms, max {:.2} ms", rtts[0], pct(0.5), pct(0.95), pct(0.99), rtts[rtts.len() - 1]);

    // 5. Throughput host -> device.
    let total: usize = 64 * 1024 * 1024;
    let chunk = vec![0xA5u8; 16384];
    let mut header = vec![b'B'];
    header.extend_from_slice(&(total as u32).to_be_bytes());
    let t = Instant::now();
    handle.write_bulk(ep_out, &header, TIMEOUT)?;
    let mut sent = 0;
    while sent < total {
        let n = handle.write_bulk(ep_out, &chunk[..chunk.len().min(total - sent)], Duration::from_secs(5))?;
        sent += n;
    }
    handle.write_bulk(ep_out, b"D", TIMEOUT)?;
    let n = handle.read_bulk(ep_in, &mut buf, Duration::from_secs(10))?;
    let secs = t.elapsed().as_secs_f64();
    let counted = u64::from_be_bytes(buf[1..9].try_into()?);
    assert!(n == 9 && buf[0] == b'R');
    println!(
        "throughput host -> device: {} MiB in {:.2} s = {:.1} MB/s ({:.0} Mbit/s); device counted {} bytes",
        total / (1024 * 1024), secs, total as f64 / secs / 1e6, total as f64 * 8.0 / secs / 1e6, counted
    );
    handle.release_interface(iface_no).ok();
    Ok(())
}
