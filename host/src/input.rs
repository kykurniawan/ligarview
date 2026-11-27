//! Pen (and later touch) input from the tablet, turned into mouse events on the virtual display.
//!
//! The tablet sends messages on the same connection as the cursor channel, in the other
//! direction. On TCP each message is `len u16 | body`; on the USB pipe a frame of the input
//! channel carries exactly one body. A body is `type u8 | fields` (big-endian):
//!
//!   1 Pointer: action u8 | tool u8 | buttons u8 | x f32 | y f32 | pressure f32
//!       action  0 hover move, 1 down, 2 move (touching), 3 up, 4 cancel
//!       tool    1 finger, 2 pen, 3 eraser
//!       buttons bit 0 = pen barrel button (acts as the right button)
//!       x, y    0..1 of the display, pressure 0..1
//!
//!   2 Scroll: phase u8 | x f32 | y f32 | dx f32 | dy f32
//!       phase   0 begin, 1 change, 2 end, 3 cancel
//!       x, y    0..1 of the display (where the fingers are; the pointer is moved there on begin)
//!       dx, dy  movement since the last message, as a fraction of the display (a finger drag
//!               moves the content by the same distance, like a trackpad with natural scrolling)
//!
//! Nothing is injected unless the user switched on "Allow pen control" (pointer) or "Allow
//! finger scrolling" (scroll) in the window and macOS granted the Accessibility permission.

use crate::access;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const MSG_POINTER: u8 = 1;
pub const MSG_SCROLL: u8 = 2;
const POINTER_LEN: usize = 16;
const SCROLL_LEN: usize = 18;

const PH_BEGIN: u8 = 0;
const PH_CHANGE: u8 = 1;
#[allow(dead_code)] // documents the protocol; end and cancel both fall into the catch-all
const PH_END: u8 = 2;
const PH_CANCEL: u8 = 3;

const ACT_HOVER: u8 = 0;
const ACT_DOWN: u8 = 1;
const ACT_MOVE: u8 = 2;
const ACT_UP: u8 = 3;
const ACT_CANCEL: u8 = 4;

const BTN_BARREL: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pointer {
    pub action: u8,
    pub tool: u8,
    pub buttons: u8,
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

/// Parses one message body. Anything malformed is ignored (the tablet is not trusted to be
/// well-behaved), and coordinates are clamped into the display.
pub fn parse(body: &[u8]) -> Option<Pointer> {
    if body.len() != POINTER_LEN || body[0] != MSG_POINTER {
        return None;
    }
    let f = |i: usize| f32::from_be_bytes([body[i], body[i + 1], body[i + 2], body[i + 3]]);
    let unit = |v: f32| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
    let p = Pointer {
        action: body[1],
        tool: body[2],
        buttons: body[3],
        x: unit(f(4)),
        y: unit(f(8)),
        pressure: unit(f(12)),
    };
    (p.action <= ACT_CANCEL).then_some(p)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scroll {
    pub phase: u8,
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
}

pub fn parse_scroll(body: &[u8]) -> Option<Scroll> {
    if body.len() != SCROLL_LEN || body[0] != MSG_SCROLL {
        return None;
    }
    let f = |i: usize| f32::from_be_bytes([body[i], body[i + 1], body[i + 2], body[i + 3]]);
    let unit = |v: f32| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
    // A single message never needs to move more than a screen.
    let delta = |v: f32| if v.is_finite() { v.clamp(-1.0, 1.0) } else { 0.0 };
    let s = Scroll { phase: body[1], x: unit(f(2)), y: unit(f(6)), dx: delta(f(10)), dy: delta(f(14)) };
    (s.phase <= PH_CANCEL).then_some(s)
}

// Quartz event types and fields.
const LEFT_DOWN: u32 = 1;
const LEFT_UP: u32 = 2;
const RIGHT_DOWN: u32 = 3;
const RIGHT_UP: u32 = 4;
const MOVED: u32 = 5;
const LEFT_DRAGGED: u32 = 6;
const RIGHT_DRAGGED: u32 = 7;
const FIELD_CLICK_STATE: u32 = 1;
const FIELD_PRESSURE: u32 = 3;
const FIELD_SCROLL_CONTINUOUS: u32 = 88;
const HID_EVENT_TAP: u32 = 0;
const BUTTON_LEFT: u32 = 0;
const BUTTON_RIGHT: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGPoint,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGDisplayBounds(display: u32) -> CGRect;
    fn CGEventCreateMouseEvent(source: *const (), kind: u32, at: CGPoint, button: u32) -> *mut ();
    fn CGEventSetIntegerValueField(event: *mut (), field: u32, value: i64);
    fn CGEventSetDoubleValueField(event: *mut (), field: u32, value: f64);
    fn CGEventCreateScrollWheelEvent(source: *const (), units: u32, wheel_count: u32, wheel1: i32, wheel2: i32) -> *mut ();
    fn CGEventPost(tap: u32, event: *mut ());
    fn CFRelease(cf: *const std::ffi::c_void);
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
    static kAXTrustedCheckOptionPrompt: *const std::ffi::c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFBooleanTrue: *const std::ffi::c_void;
    static kCFTypeDictionaryKeyCallBacks: [u8; 0];
    static kCFTypeDictionaryValueCallBacks: [u8; 0];
    fn CFDictionaryCreate(
        allocator: *const std::ffi::c_void,
        keys: *const *const std::ffi::c_void,
        values: *const *const std::ffi::c_void,
        count: isize,
        key_callbacks: *const std::ffi::c_void,
        value_callbacks: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;
}

/// Asks macOS for the Accessibility permission. This is what adds Ligarview to the list in
/// System Settings and shows the system prompt; merely checking never does. Returns the current state.
pub fn request_trust() -> bool {
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            kCFTypeDictionaryKeyCallBacks.as_ptr() as *const _,
            kCFTypeDictionaryValueCallBacks.as_ptr() as *const _,
        );
        let trusted = AXIsProcessTrustedWithOptions(options);
        if !options.is_null() {
            CFRelease(options);
        }
        trusted
    }
}

/// True when macOS lets this process post input events.
pub fn trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Opens the Accessibility pane of System Settings, where the user allows Ligarview.
pub fn open_accessibility_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

static DISPLAY: OnceLock<Arc<AtomicU32>> = OnceLock::new();
static WARNED: AtomicBool = AtomicBool::new(false);
static DENIED_LOGGED: AtomicBool = AtomicBool::new(false);

struct State {
    /// Button currently held down by the pen, if any.
    held: Option<u32>,
    last_down: Option<(Instant, f64, f64)>,
    /// Where the pointer was last placed, for lifting a stuck button.
    last_at: Option<CGPoint>,
    clicks: i64,
    /// Scroll distance (in points) not yet sent because it was less than a whole pixel.
    scroll_rest: (f64, f64),
}

static STATE: Mutex<State> = Mutex::new(State { held: None, last_down: None, last_at: None, clicks: 0, scroll_rest: (0.0, 0.0) });

/// Tells the input handler which display is the virtual one (it changes when it is recreated).
pub fn init(display: Arc<AtomicU32>) {
    let _ = DISPLAY.set(display);
}

fn post(kind: u32, at: CGPoint, button: u32, clicks: i64, pressure: f64) {
    unsafe {
        let e = CGEventCreateMouseEvent(std::ptr::null(), kind, at, button);
        if e.is_null() {
            return;
        }
        if clicks > 0 {
            CGEventSetIntegerValueField(e, FIELD_CLICK_STATE, clicks);
        }
        CGEventSetDoubleValueField(e, FIELD_PRESSURE, pressure);
        CGEventPost(HID_EVENT_TAP, e);
        CFRelease(e as *const std::ffi::c_void);
    }
}

fn post_scroll(dx: i32, dy: i32) {
    unsafe {
        let e = CGEventCreateScrollWheelEvent(std::ptr::null(), 0, 2, dy, dx);
        if e.is_null() {
            return;
        }
        CGEventSetIntegerValueField(e, FIELD_SCROLL_CONTINUOUS, 1);
        CGEventPost(HID_EVENT_TAP, e);
        CFRelease(e as *const std::ffi::c_void);
    }
}

/// Lifts a button that is still held, e.g. when the tablet disconnects mid-stroke.
pub fn release_all() {
    let mut st = STATE.lock().unwrap();
    if let (Some(button), Some(at)) = (st.held.take(), st.last_at) {
        post(if button == BUTTON_RIGHT { RIGHT_UP } else { LEFT_UP }, at, button, st.clicks, 0.0);
    }
}

/// Applies one message body from the tablet.
pub fn handle(body: &[u8]) {
    let scroll = body.first() == Some(&MSG_SCROLL);
    let pointer = if scroll { None } else { parse(body) };
    let scroll_msg = if scroll { parse_scroll(body) } else { None };
    if pointer.is_none() && scroll_msg.is_none() {
        return;
    }
    let allowed = if scroll { access::finger_enabled() } else { access::control_enabled() };
    if !access::enabled() || !allowed {
        if !DENIED_LOGGED.swap(true, Ordering::Relaxed) {
            println!("input: ignored, {} is not allowed in the Ligarview window", if scroll { "finger scrolling" } else { "pen control" });
        }
        return;
    }
    if !trusted() {
        if !WARNED.swap(true, Ordering::Relaxed) {
            println!("input: ignored, Ligarview is not allowed in System Settings > Privacy & Security > Accessibility");
        }
        return;
    }
    WARNED.store(false, Ordering::Relaxed);
    let Some(display) = DISPLAY.get() else { return };
    let b = unsafe { CGDisplayBounds(display.load(Ordering::Relaxed)) };
    if b.size.x <= 0.0 || b.size.y <= 0.0 {
        return;
    }
    let place = |x: f32, y: f32| CGPoint {
        x: b.origin.x + x as f64 * (b.size.x - 1.0),
        y: b.origin.y + y as f64 * (b.size.y - 1.0),
    };
    if let Some(sc) = scroll_msg {
        let at = place(sc.x, sc.y);
        let mut st = STATE.lock().unwrap();
        match sc.phase {
            PH_BEGIN => {
                st.scroll_rest = (0.0, 0.0);
                // The wheel acts where the pointer is, so bring the pointer under the fingers.
                if st.held.is_none() {
                    post(MOVED, at, BUTTON_LEFT, 0, 0.0);
                    st.last_at = Some(at);
                }
            }
            PH_CHANGE => {
                let x = st.scroll_rest.0 + sc.dx as f64 * b.size.x;
                let y = st.scroll_rest.1 + sc.dy as f64 * b.size.y;
                let (ix, iy) = (x.trunc(), y.trunc());
                st.scroll_rest = (x - ix, y - iy);
                if ix != 0.0 || iy != 0.0 {
                    post_scroll(ix as i32, iy as i32);
                }
            }
            // End or cancel: nothing to send (no momentum), just forget the sub-point remainder.
            _ => st.scroll_rest = (0.0, 0.0),
        }
        return;
    }
    let Some(p) = pointer else { return };
    let at = place(p.x, p.y);
    let pressure = p.pressure as f64;
    let mut st = STATE.lock().unwrap();
    st.last_at = Some(at);
    match p.action {
        ACT_HOVER => {
            // A hover while a button is held means the "up" got lost; let go first.
            if let Some(button) = st.held.take() {
                post(if button == BUTTON_RIGHT { RIGHT_UP } else { LEFT_UP }, at, button, st.clicks, 0.0);
            }
            post(MOVED, at, BUTTON_LEFT, 0, 0.0);
        }
        ACT_DOWN => {
            let button = if p.buttons & BTN_BARREL != 0 { BUTTON_RIGHT } else { BUTTON_LEFT };
            let now = Instant::now();
            // Double and triple clicks: same place, quickly after the previous press.
            st.clicks = match st.last_down {
                Some((t, x, y)) if now.duration_since(t) < Duration::from_millis(400) && (x - at.x).abs() < 6.0 && (y - at.y).abs() < 6.0 => st.clicks + 1,
                _ => 1,
            };
            st.last_down = Some((now, at.x, at.y));
            st.held = Some(button);
            post(if button == BUTTON_RIGHT { RIGHT_DOWN } else { LEFT_DOWN }, at, button, st.clicks, pressure.max(0.01));
        }
        ACT_MOVE => match st.held {
            Some(button) => post(if button == BUTTON_RIGHT { RIGHT_DRAGGED } else { LEFT_DRAGGED }, at, button, st.clicks, pressure),
            None => post(MOVED, at, BUTTON_LEFT, 0, 0.0),
        },
        ACT_UP | ACT_CANCEL => {
            if let Some(button) = st.held.take() {
                post(if button == BUTTON_RIGHT { RIGHT_UP } else { LEFT_UP }, at, button, st.clicks, 0.0);
            }
        }
        _ => {}
    }
}

/// Reads length-prefixed messages (TCP) until `stop` is set or the stream ends.
pub fn read_stream(mut stream: impl Read, stop: &AtomicBool) {
    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let mut len = [0u8; 2];
        match stream.read_exact(&mut len) {
            Ok(()) => {}
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => continue,
            Err(_) => break,
        }
        let mut body = vec![0u8; u16::from_be_bytes(len) as usize];
        let mut filled = 0;
        while filled < body.len() {
            if stop.load(Ordering::Relaxed) {
                release_all();
                return;
            }
            match stream.read(&mut body[filled..]) {
                Ok(0) => {
                    release_all();
                    return;
                }
                Ok(n) => filled += n,
                Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => continue,
                Err(_) => {
                    release_all();
                    return;
                }
            }
        }
        handle(&body);
    }
    release_all();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(action: u8, tool: u8, buttons: u8, x: f32, y: f32, pressure: f32) -> Vec<u8> {
        let mut m = vec![MSG_POINTER, action, tool, buttons];
        for v in [x, y, pressure] {
            m.extend_from_slice(&v.to_be_bytes());
        }
        m
    }

    #[test]
    fn parses_a_pointer() {
        let p = parse(&msg(ACT_DOWN, 2, 1, 0.25, 0.75, 0.5)).unwrap();
        assert_eq!((p.action, p.tool, p.buttons), (ACT_DOWN, 2, 1));
        assert_eq!((p.x, p.y, p.pressure), (0.25, 0.75, 0.5));
    }

    #[test]
    fn parses_a_scroll() {
        let mut m = vec![MSG_SCROLL, PH_CHANGE];
        for v in [0.5f32, 0.25, -0.1, 0.2] {
            m.extend_from_slice(&v.to_be_bytes());
        }
        let s = parse_scroll(&m).unwrap();
        assert_eq!((s.phase, s.x, s.y, s.dx, s.dy), (PH_CHANGE, 0.5, 0.25, -0.1, 0.2));
        assert!(parse_scroll(&m[..17]).is_none());
        m[1] = 9;
        assert!(parse_scroll(&m).is_none());
        // Not a pointer message.
        assert!(parse(&m).is_none());
    }

    #[test]
    fn clamps_and_rejects_garbage() {
        let p = parse(&msg(ACT_MOVE, 2, 0, 3.0, f32::NAN, -1.0)).unwrap();
        assert_eq!((p.x, p.y, p.pressure), (1.0, 0.0, 0.0));
        assert!(parse(&[]).is_none());
        assert!(parse(&[MSG_POINTER, 0]).is_none());
        assert!(parse(&msg(9, 2, 0, 0.5, 0.5, 0.5)).is_none());
        let mut wrong_type = msg(ACT_DOWN, 2, 0, 0.5, 0.5, 0.5);
        wrong_type[0] = 77;
        assert!(parse(&wrong_type).is_none());
    }
}
