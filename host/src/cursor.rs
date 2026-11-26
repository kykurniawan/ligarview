//! System cursor tracking: position (every few ms) and shape (when it changes).
//!
//! The cursor is sent separately from the video and drawn by the tablet, so it does
//! not have to wait for capture + encode + decode.

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::msg_send;
use objc2_core_foundation::{CGPoint, CGSize};
use std::ffi::c_void;

#[link(name = "AppKit", kind = "framework")]
extern "C" {}

#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    origin: CGPoint,
    size: CGSize,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventCreate(source: *const c_void) -> *const c_void;
    fn CGEventGetLocation(event: *const c_void) -> CGPoint;
    fn CGDisplayBounds(display: u32) -> Rect;
    /// Private but long-standing: changes whenever the system cursor image changes.
    fn CGSCurrentCursorSeed() -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
}

/// Cursor position in the display's own coordinate space, 0..1 on both axes.
#[derive(Clone, Copy, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub inside: bool,
}

pub fn position(display: u32) -> Position {
    unsafe {
        let ev = CGEventCreate(std::ptr::null());
        if ev.is_null() {
            return Position { x: 0.0, y: 0.0, inside: false };
        }
        let p = CGEventGetLocation(ev);
        CFRelease(ev);
        let b = CGDisplayBounds(display);
        let (w, h) = (b.size.width, b.size.height);
        let (x, y) = ((p.x - b.origin.x) / w, (p.y - b.origin.y) / h);
        Position { x: x as f32, y: y as f32, inside: (0.0..1.0).contains(&x) && (0.0..1.0).contains(&y) }
    }
}

/// Global position and size of a display in points: (x, y, width, height).
pub fn display_bounds(display: u32) -> (f64, f64, f64, f64) {
    let b = unsafe { CGDisplayBounds(display) };
    (b.origin.x, b.origin.y, b.size.width, b.size.height)
}

pub fn shape_seed() -> i32 {
    unsafe { CGSCurrentCursorSeed() }
}

/// A cursor image as PNG, with sizes in points (the tablet scales them to its pixels).
pub struct Shape {
    pub png: Vec<u8>,
    pub width_pts: f32,
    pub height_pts: f32,
    pub hot_x_pts: f32,
    pub hot_y_pts: f32,
}

/// Reads the image of the cursor currently shown by the system (any app).
pub fn current_shape() -> Option<Shape> {
    unsafe {
        let cursor_cls = AnyClass::get(c"NSCursor")?;
        let cursor: *mut AnyObject = msg_send![cursor_cls, currentSystemCursor];
        shape_of(cursor)
    }
}

/// Renders an `NSCursor` to a PNG.
unsafe fn shape_of(cursor: *mut AnyObject) -> Option<Shape> {
    let rep_cls = AnyClass::get(c"NSBitmapImageRep")?;
    let dict_cls = AnyClass::get(c"NSDictionary")?;
    if cursor.is_null() {
        return None;
    }
    let image: *mut AnyObject = msg_send![cursor, image];
    if image.is_null() {
        return None;
    }
    let hot: CGPoint = msg_send![cursor, hotSpot];
    let size: CGSize = msg_send![image, size];

    // Best (usually Retina) bitmap for the image.
    let cg: *const c_void = msg_send![
        image,
        CGImageForProposedRect: std::ptr::null_mut::<c_void>(),
        context: std::ptr::null_mut::<c_void>(),
        hints: std::ptr::null_mut::<c_void>()
    ];
    if cg.is_null() {
        return None;
    }
    let alloc: *mut AnyObject = msg_send![rep_cls, alloc];
    let rep: *mut AnyObject = msg_send![alloc, initWithCGImage: cg];
    let rep = Retained::from_raw(rep)?;
    let props: *mut AnyObject = msg_send![dict_cls, dictionary];
    // NSBitmapImageFileTypePNG = 4
    let data: *mut AnyObject = msg_send![&*rep, representationUsingType: 4usize, properties: props];
    if data.is_null() {
        return None;
    }
    let len: usize = msg_send![data, length];
    let ptr: *const u8 = msg_send![data, bytes];
    let png = std::slice::from_raw_parts(ptr, len).to_vec();
    Some(Shape {
        png,
        width_pts: size.width as f32,
        height_pts: size.height as f32,
        hot_x_pts: hot.x as f32,
        hot_y_pts: hot.y as f32,
    })
}

/// Every standard AppKit cursor, for diagnosing shapes that fail to render.
pub fn standard_shapes() -> Vec<(&'static str, Option<Shape>)> {
    let selectors: [(&str, objc2::runtime::Sel); 18] = [
        ("arrow", objc2::sel!(arrowCursor)),
        ("iBeam", objc2::sel!(IBeamCursor)),
        ("crosshair", objc2::sel!(crosshairCursor)),
        ("closedHand", objc2::sel!(closedHandCursor)),
        ("openHand", objc2::sel!(openHandCursor)),
        ("pointingHand", objc2::sel!(pointingHandCursor)),
        ("resizeLeft", objc2::sel!(resizeLeftCursor)),
        ("resizeRight", objc2::sel!(resizeRightCursor)),
        ("resizeLeftRight", objc2::sel!(resizeLeftRightCursor)),
        ("resizeUp", objc2::sel!(resizeUpCursor)),
        ("resizeDown", objc2::sel!(resizeDownCursor)),
        ("resizeUpDown", objc2::sel!(resizeUpDownCursor)),
        ("disappearingItem", objc2::sel!(disappearingItemCursor)),
        ("iBeamVertical", objc2::sel!(IBeamCursorForVerticalLayout)),
        ("operationNotAllowed", objc2::sel!(operationNotAllowedCursor)),
        ("dragLink", objc2::sel!(dragLinkCursor)),
        ("dragCopy", objc2::sel!(dragCopyCursor)),
        ("contextualMenu", objc2::sel!(contextualMenuCursor)),
    ];
    let Some(cls) = AnyClass::get(c"NSCursor") else { return vec![] };
    selectors
        .into_iter()
        .map(|(name, sel)| unsafe {
            let cursor: *mut AnyObject = objc2::msg_send![cls, performSelector: sel];
            (name, shape_of(cursor))
        })
        .collect()
}
