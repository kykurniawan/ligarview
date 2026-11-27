//! Who may connect: the sharing switch and the pairing code, both managed from the host window.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(true);
static CODE: AtomicU32 = AtomicU32::new(0);
static KICK: AtomicBool = AtomicBool::new(false);
static TABLET: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

pub fn init(enabled: bool, code: u32, tablet: String) {
    *TABLET.lock().unwrap() = tablet;
    ENABLED.store(enabled, Ordering::SeqCst);
    CODE.store(code, Ordering::SeqCst);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

/// Turning sharing off also asks the supervisor to drop whoever is connected.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::SeqCst);
    if !on {
        KICK.store(true, Ordering::SeqCst);
    }
}

pub fn code() -> u32 {
    CODE.load(Ordering::SeqCst)
}

/// Pairing a tablet (or unpairing, with 0) drops the current Wi-Fi client, which no longer matches.
pub fn set_code(code: u32) {
    CODE.store(code, Ordering::SeqCst);
    KICK.store(true, Ordering::SeqCst);
}

/// Name of the paired tablet (empty when none).
pub fn tablet() -> String {
    TABLET.lock().unwrap().clone()
}

pub fn set_tablet(name: String) {
    *TABLET.lock().unwrap() = name;
}

pub fn take_kick() -> bool {
    KICK.swap(false, Ordering::SeqCst)
}
