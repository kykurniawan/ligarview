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
}

extern "C" {
    fn dispatch_get_global_queue(identifier: isize, flags: usize) -> *mut AnyObject;
}

pub struct VirtualDisplay {
    // Keep alive: dropping the object removes the display.
    _display: Retained<AnyObject>,
    pub id: u32,
}

fn class(name: &'static std::ffi::CStr) -> Result<&'static AnyClass> {
    AnyClass::get(name).ok_or_else(|| anyhow!("class {name:?} not found (macOS changed the private API?)"))
}

impl VirtualDisplay {
    pub fn create(width: u32, height: u32, refresh: f64, hidpi: bool) -> Result<Self> {
        unsafe {
            let desc_cls = class(c"CGVirtualDisplayDescriptor")?;
            let mode_cls = class(c"CGVirtualDisplayMode")?;
            let settings_cls = class(c"CGVirtualDisplaySettings")?;
            let display_cls = class(c"CGVirtualDisplay")?;

            let desc: Retained<AnyObject> = msg_send![desc_cls, new];
            let name = NSString::from_str("Ligarview");
            let queue = dispatch_get_global_queue(0, 0);
            // ~11" panel: 2560x1600 at 274 ppi
            let size = CGSize::new(width as f64 * 25.4 / 274.0, height as f64 * 25.4 / 274.0);
            let _: () = msg_send![&*desc, setName: &*name];
            let _: () = msg_send![&*desc, setMaxPixelsWide: width];
            let _: () = msg_send![&*desc, setMaxPixelsHigh: height];
            let _: () = msg_send![&*desc, setSizeInMillimeters: size];
            let _: () = msg_send![&*desc, setProductID: 0x4C56u32];
            let _: () = msg_send![&*desc, setVendorID: 0x4C47u32];
            let _: () = msg_send![&*desc, setSerialNum: 1u32];
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
