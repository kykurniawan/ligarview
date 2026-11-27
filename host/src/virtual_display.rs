//! Virtual display via the private CoreGraphics `CGVirtualDisplay` API.
//! Private API: Apple may change it in a macOS update.

use anyhow::{anyhow, Result};
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::msg_send;
use objc2_core_foundation::CGSize;
use objc2_foundation::{NSArray, NSString};

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGGetActiveDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayPixelsWide(display: u32) -> usize;
    fn CGDisplayPixelsHigh(display: u32) -> usize;
    fn CGDisplayCopyDisplayMode(display: u32) -> *const std::ffi::c_void;
    fn CGDisplayModeGetWidth(mode: *const std::ffi::c_void) -> usize;
    fn CGDisplayModeGetHeight(mode: *const std::ffi::c_void) -> usize;
    fn CGDisplayModeGetPixelWidth(mode: *const std::ffi::c_void) -> usize;
    fn CGDisplayModeGetPixelHeight(mode: *const std::ffi::c_void) -> usize;
    fn CGDisplayModeGetRefreshRate(mode: *const std::ffi::c_void) -> f64;
    fn CGDisplayModeRelease(mode: *const std::ffi::c_void);
    fn CGDisplayCopyAllDisplayModes(display: u32, options: *const std::ffi::c_void) -> *const std::ffi::c_void;
    fn CGBeginDisplayConfiguration(config: *mut *mut std::ffi::c_void) -> i32;
    fn CGConfigureDisplayWithDisplayMode(
        config: *mut std::ffi::c_void,
        display: u32,
        mode: *const std::ffi::c_void,
        options: *const std::ffi::c_void,
    ) -> i32;
    fn CGCompleteDisplayConfiguration(config: *mut std::ffi::c_void, option: u32) -> i32;
    fn CGCancelDisplayConfiguration(config: *mut std::ffi::c_void) -> i32;
    static kCGDisplayShowDuplicateLowResolutionModes: *const std::ffi::c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFRunLoopDefaultMode: *const std::ffi::c_void;
    fn CFRunLoopRunInMode(mode: *const std::ffi::c_void, seconds: f64, return_after_source: u8) -> i32;
    fn CFArrayGetCount(arr: *const std::ffi::c_void) -> isize;
    fn CFArrayGetValueAtIndex(arr: *const std::ffi::c_void, idx: isize) -> *const std::ffi::c_void;
    fn CFRelease(cf: *const std::ffi::c_void);
    static kCFBooleanTrue: *const std::ffi::c_void;
    static kCFTypeDictionaryKeyCallBacks: u8;
    static kCFTypeDictionaryValueCallBacks: u8;
    fn CFDictionaryCreate(
        alloc: *const std::ffi::c_void,
        keys: *const *const std::ffi::c_void,
        values: *const *const std::ffi::c_void,
        n: isize,
        kcb: *const std::ffi::c_void,
        vcb: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;
}

extern "C" {
    fn dispatch_get_global_queue(identifier: isize, flags: usize) -> *mut AnyObject;
}

static SERIAL: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

/// Sleeps while servicing the main run loop. Display configuration changes are delivered
/// through it, so a plain sleep on the main thread can leave a new display without modes.
pub fn pump_run_loop(duration: std::time::Duration) {
    if crate::gui::active() {
        crate::gui::pump(duration);
        return;
    }
    unsafe {
        CFRunLoopRunInMode(kCFRunLoopDefaultMode, duration.as_secs_f64(), 0);
    }
}

pub struct VirtualDisplay {
    // Keep alive: dropping the object removes the display.
    _display: Retained<AnyObject>,
    pub id: u32,
}

unsafe fn all_modes(id: u32) -> *const std::ffi::c_void {
    let keys = [kCGDisplayShowDuplicateLowResolutionModes];
    let vals = [kCFBooleanTrue];
    let opts = CFDictionaryCreate(
        std::ptr::null(),
        keys.as_ptr(),
        vals.as_ptr(),
        1,
        &kCFTypeDictionaryKeyCallBacks as *const u8 as *const std::ffi::c_void,
        &kCFTypeDictionaryValueCallBacks as *const u8 as *const std::ffi::c_void,
    );
    let arr = CGDisplayCopyAllDisplayModes(id, opts);
    CFRelease(opts);
    arr
}

fn class(name: &'static std::ffi::CStr) -> Result<&'static AnyClass> {
    AnyClass::get(name).ok_or_else(|| anyhow!("class {name:?} not found (macOS changed the private API?)"))
}

impl VirtualDisplay {
    /// `width`/`height` are the logical size in points; with `scale` 2 the framebuffer is
    /// twice as large in each direction (HiDPI, like a Retina display).
    pub fn create(width: u32, height: u32, scale: u32, refresh: f64) -> Result<Self> {
        let (px_w, px_h) = (width * scale, height * scale);
        let hidpi = scale > 1;
        // The mode is defined in points; with hiDPI the framebuffer is `scale` times larger.
        let (max_w, max_h) = (px_w, px_h);
        unsafe {
            let desc_cls = class(c"CGVirtualDisplayDescriptor")?;
            let mode_cls = class(c"CGVirtualDisplayMode")?;
            let settings_cls = class(c"CGVirtualDisplaySettings")?;
            let display_cls = class(c"CGVirtualDisplay")?;

            let desc: Retained<AnyObject> = msg_send![desc_cls, new];
            let name = NSString::from_str("Ligarview");
            let queue = dispatch_get_global_queue(0, 0);
            // ~11" panel: 2560x1600 at 274 ppi
            let size = CGSize::new(px_w as f64 * 25.4 / 274.0, px_h as f64 * 25.4 / 274.0);
            let _: () = msg_send![&*desc, setName: &*name];
            let _: () = msg_send![&*desc, setMaxPixelsWide: max_w];
            let _: () = msg_send![&*desc, setMaxPixelsHigh: max_h];
            let _: () = msg_send![&*desc, setSizeInMillimeters: size];
            let _: () = msg_send![&*desc, setProductID: 0x4C56u32];
            let _: () = msg_send![&*desc, setVendorID: 0x4C47u32];
            // A fresh serial per instance: with identical IDs macOS can treat a recreated display as
            // the old one and hand back a display without any modes.
            let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let _: () = msg_send![&*desc, setSerialNum: serial];
            let _: () = msg_send![&*desc, setQueue: queue];

            let alloc: *mut AnyObject = msg_send![display_cls, alloc];
            let raw: *mut AnyObject = msg_send![alloc, initWithDescriptor: &*desc];
            let display = Retained::from_raw(raw)
                .ok_or_else(|| anyhow!("CGVirtualDisplay init returned nil"))?;

            let alloc: *mut AnyObject = msg_send![mode_cls, alloc];
            let raw: *mut AnyObject =
                msg_send![alloc, initWithWidth: width, height: height, refreshRate: refresh];
            let mode = Retained::from_raw(raw)
                .ok_or_else(|| anyhow!("CGVirtualDisplayMode init failed"))?;
            let modes = NSArray::from_retained_slice(&[mode]);

            let settings: Retained<AnyObject> = msg_send![settings_cls, new];
            let _: () = msg_send![&*settings, setHiDPI: if hidpi { 1u32 } else { 0u32 }];
            let _: () = msg_send![&*settings, setModes: &*modes];
            let ok: bool = msg_send![&*display, applySettings: &*settings];
            if !ok {
                return Err(anyhow!("applySettings failed"));
            }
            let id: u32 = msg_send![&*display, displayID];
            Ok(Self { _display: display, id })
        }
    }

    /// All modes the display offers: (points_w, points_h, pixel_w, pixel_h, hz).
    #[allow(dead_code)]
    pub fn modes(id: u32) -> Vec<(usize, usize, usize, usize, f64)> {
        let mut out = vec![];
        unsafe {
            let arr = all_modes(id);
            if arr.is_null() {
                return out;
            }
            for i in 0..CFArrayGetCount(arr) {
                let m = CFArrayGetValueAtIndex(arr, i);
                out.push((
                    CGDisplayModeGetWidth(m),
                    CGDisplayModeGetHeight(m),
                    CGDisplayModeGetPixelWidth(m),
                    CGDisplayModeGetPixelHeight(m),
                    CGDisplayModeGetRefreshRate(m),
                ));
            }
            CFRelease(arr);
        }
        out
    }

    /// Switch the display to a mode with these exact point and pixel dimensions.
    /// Several modes can share the same dimensions, so every candidate is tried.
    pub fn select_mode(id: u32, points: (u32, u32), pixels: (u32, u32)) -> Result<()> {
        unsafe {
            let arr = all_modes(id);
            if arr.is_null() {
                return Err(anyhow!("no display modes"));
            }
            let mut found = 0;
            let mut last_err = String::new();
            let mut ok = false;
            for i in 0..CFArrayGetCount(arr) {
                let m = CFArrayGetValueAtIndex(arr, i);
                if CGDisplayModeGetWidth(m) != points.0 as usize
                    || CGDisplayModeGetHeight(m) != points.1 as usize
                    || CGDisplayModeGetPixelWidth(m) != pixels.0 as usize
                    || CGDisplayModeGetPixelHeight(m) != pixels.1 as usize
                {
                    continue;
                }
                found += 1;
                let mut cfg = std::ptr::null_mut();
                let rc = CGBeginDisplayConfiguration(&mut cfg);
                if rc != 0 {
                    last_err = format!("CGBeginDisplayConfiguration={rc}");
                    continue;
                }
                let rc = CGConfigureDisplayWithDisplayMode(cfg, id, m, std::ptr::null());
                if rc != 0 {
                    CGCancelDisplayConfiguration(cfg);
                    last_err = format!("CGConfigureDisplayWithDisplayMode={rc}");
                    continue;
                }
                let rc = CGCompleteDisplayConfiguration(cfg, 0);
                if rc != 0 {
                    last_err = format!("CGCompleteDisplayConfiguration={rc}");
                    continue;
                }
                ok = true;
                break;
            }
            CFRelease(arr);
            if ok {
                Ok(())
            } else if found == 0 {
                Err(anyhow!("no {}x{} points / {}x{} pixels mode offered", points.0, points.1, pixels.0, pixels.1))
            } else {
                Err(anyhow!("{found} candidate mode(s), all rejected: {last_err}"))
            }
        }
    }

    /// Global position and size of the display in points: (x, y, width, height).
    pub fn bounds(id: u32) -> (f64, f64, f64, f64) {
        crate::cursor::display_bounds(id)
    }

    /// Whether the display is still attached to the system.
    pub fn is_active(id: u32) -> bool {
        Self::active_displays().iter().any(|&(d, _, _)| d == id)
    }

    /// Human-readable current mode: points vs real pixels.
    pub fn describe(id: u32) -> String {
        unsafe {
            let m = CGDisplayCopyDisplayMode(id);
            if m.is_null() {
                return "no mode".into();
            }
            let r = format!(
                "{}x{} points, {}x{} pixels, {:.0} Hz",
                CGDisplayModeGetWidth(m),
                CGDisplayModeGetHeight(m),
                CGDisplayModeGetPixelWidth(m),
                CGDisplayModeGetPixelHeight(m),
                CGDisplayModeGetRefreshRate(m)
            );
            CGDisplayModeRelease(m);
            r
        }
    }

    /// (id, width, height) of all active displays, to verify the virtual one appears.
    pub fn active_displays() -> Vec<(u32, usize, usize)> {
        let mut ids = [0u32; 16];
        let mut n = 0u32;
        unsafe {
            if CGGetActiveDisplayList(16, ids.as_mut_ptr(), &mut n) != 0 {
                return vec![];
            }
            ids[..n as usize]
                .iter()
                .map(|&d| (d, CGDisplayPixelsWide(d), CGDisplayPixelsHigh(d)))
                .collect()
        }
    }
}
