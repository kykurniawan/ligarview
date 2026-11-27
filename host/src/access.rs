//! Who may connect: the sharing switch and the pairing code, both managed from the host window.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(true);
static CODE: AtomicU32 = AtomicU32::new(0);
static KICK: AtomicBool = AtomicBool::new(false);

pub fn init(enabled: bool, code: u32) {
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

/// A new code also drops the current Wi-Fi client, which no longer matches it.
pub fn set_code(code: u32) {
    CODE.store(code, Ordering::SeqCst);
    KICK.store(true, Ordering::SeqCst);
}

pub fn take_kick() -> bool {
    KICK.swap(false, Ordering::SeqCst)
}

/// Random six-digit code (100000–999999).
pub fn generate_code() -> u32 {
    use std::io::Read;
    let mut b = [0u8; 4];
    let ok = std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_ok();
    let n = if ok {
        u32::from_le_bytes(b)
    } else {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1)
    };
    100_000 + n % 900_000
}
