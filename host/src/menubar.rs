//! Menu bar app: a status item with a menu to see the state and change the settings.
//!
//! The host's supervisor loop stays on the main thread and pumps AppKit events through
//! [`pump`] instead of a bare run loop, so the menu works without a separate UI thread.
//! Changing a setting saves it and relaunches the host, which rebuilds the display from scratch.

use crate::config::Config;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, ProtocolObject};
use objc2::{define_class, msg_send, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSEvent, NSEventMask, NSImage, NSMenu, NSMenuItem,
    NSStatusBar, NSStatusItem, NSControlStateValueOff, NSControlStateValueOn,
};
use objc2_foundation::{NSData, NSObjectProtocol, NSDate, NSDefaultRunLoopMode, NSSize, NSString};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

static REOPEN: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static ACTIONS: Mutex<Vec<isize>> = Mutex::new(Vec::new());

pub fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// What the menu shows on its first line.
#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Starting,
    Waiting,
    Streaming,
}

const ORIENTATION: isize = 10; // +0 landscape, +1 portrait
const FPS: isize = 20; // + fps
const BITRATE: isize = 100; // + Mbps
const CODEC: isize = 40; // +0 H.264, +1 HEVC
const LOGIN: isize = 50;
const PERMISSION: isize = 60;
const QUIT: isize = 99;

const FPS_CHOICES: [u32; 3] = [30, 60, 90];
const BITRATE_CHOICES: [(u32, &str); 4] = [(20, "Standard"), (30, "High"), (40, "Higher"), (60, "Maximum")];

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "LigarviewMenuHandler"]
    struct Handler;

    impl Handler {
        #[unsafe(method(act:))]
        fn act(&self, sender: &NSMenuItem) {
            ACTIONS.lock().unwrap().push(sender.tag());
        }
    }
);

define_class!(
    /// Opening Ligarview.app again while it runs pops the menu up at the pointer, for menu bars
    /// too crowded to show the status item (the notch hides whatever does not fit).
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "LigarviewAppDelegate"]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl NSApplicationDelegate for Delegate {
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn reopen(&self, _app: &NSApplication, _visible: bool) -> bool {
            println!("menubar: reopen requested");
            REOPEN.store(true, Ordering::Relaxed);
            true
        }
    }
);

impl Handler {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

struct Menubar {
    mtm: MainThreadMarker,
    _item: Retained<NSStatusItem>,
    _handler: Retained<Handler>,
    _delegate: Retained<Delegate>,
    menu: Retained<NSMenu>,
    status: Retained<NSMenuItem>,
    permission: Retained<NSMenuItem>,
    login: Retained<NSMenuItem>,
    /// Items whose check mark follows the current config: (tag, item).
    checks: Vec<(isize, Retained<NSMenuItem>)>,
    config: Config,
    shown: Option<(Status, bool)>,
}

thread_local! {
    static MENU: RefCell<Option<Menubar>> = const { RefCell::new(None) };
}

fn title(s: &str) -> Retained<NSString> {
    NSString::from_str(s)
}

fn item(
    mtm: MainThreadMarker,
    handler: &Handler,
    text: &str,
    tag: isize,
) -> Retained<NSMenuItem> {
    let it = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), &title(text), Some(sel!(act:)), &title(""))
    };
    unsafe { it.setTarget(Some(handler)) };
    it.setTag(tag);
    it
}

fn label(mtm: MainThreadMarker, text: &str) -> Retained<NSMenuItem> {
    let it = unsafe { NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), &title(text), None, &title("")) };
    it.setEnabled(false);
    it
}

fn submenu(mtm: MainThreadMarker, menu: &NSMenu, name: &str, entries: &[Retained<NSMenuItem>]) {
    let parent = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), &title(name), None, &title(""))
    };
    let sub = NSMenu::new(mtm);
    sub.setAutoenablesItems(false);
    for e in entries {
        sub.addItem(e);
    }
    parent.setSubmenu(Some(&sub));
    menu.addItem(&parent);
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

pub fn has_screen_access() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

/// Creates the status item. Call once, on the main thread, before the supervisor loop.
pub fn init(config: &Config, resolution: (u32, u32), codec: &str) {
    let mtm = MainThreadMarker::new().expect("menu bar must be set up on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    app.finishLaunching();

    let delegate: Retained<Delegate> = {
        let this = mtm.alloc::<Delegate>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    };
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    let handler = Handler::new(mtm);
    let bar = NSStatusBar::systemStatusBar();
    let status_item = bar.statusItemWithLength(-1.0); // NSVariableStatusItemLength
    status_item.setAutosaveName(Some(&title("LigarviewStatusItem")));
    status_item.setVisible(true);
    if let Some(button) = status_item.button(mtm) {
        let data = NSData::with_bytes(include_bytes!("../assets/menubar.png"));
        if let Some(img) = NSImage::initWithData(mtm.alloc(), &data) {
            img.setSize(NSSize { width: 56.0 / 2.0, height: 44.0 / 2.0 });
            img.setTemplate(true);
            button.setImage(Some(&img));
        } else {
            button.setTitle(&title("LV"));
        }
    }

    let menu = NSMenu::new(mtm);
    menu.setAutoenablesItems(false);
    let status = label(mtm, "Starting…");
    menu.addItem(&status);
    menu.addItem(&label(mtm, &format!("{}x{} · {} fps · {codec}", resolution.0, resolution.1, config.fps)));
    let permission = item(mtm, &handler, "Allow Screen Recording…", PERMISSION);
    permission.setHidden(true);
    menu.addItem(&permission);
    menu.addItem(&NSMenuItem::separatorItem(mtm));

    let mut checks = Vec::new();
    let mut group = |name: &str, entries: Vec<(isize, String)>| {
        let items: Vec<_> = entries.iter().map(|(t, n)| item(mtm, &handler, n, *t)).collect();
        for (i, (t, _)) in entries.iter().enumerate() {
            checks.push((*t, items[i].clone()));
        }
        submenu(mtm, &menu, name, &items);
    };
    group("Orientation", vec![(ORIENTATION, "Landscape".into()), (ORIENTATION + 1, "Portrait".into())]);
    group("Frame rate", FPS_CHOICES.iter().map(|f| (FPS + *f as isize, format!("{f} fps"))).collect());
    group(
        "Quality",
        BITRATE_CHOICES.iter().map(|(m, n)| (BITRATE + *m as isize, format!("{n} ({m} Mbps)"))).collect(),
    );
    group("Codec", vec![(CODEC, "H.264".into()), (CODEC + 1, "HEVC".into())]);

    menu.addItem(&NSMenuItem::separatorItem(mtm));
    let login = item(mtm, &handler, "Start at login", LOGIN);
    menu.addItem(&login);
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    menu.addItem(&item(mtm, &handler, "Quit Ligarview", QUIT));
    status_item.setMenu(Some(&menu));

    if !has_screen_access() {
        unsafe { CGRequestScreenCaptureAccess() };
    }

    ACTIVE.store(true, Ordering::Relaxed);
    MENU.with(|m| {
        *m.borrow_mut() = Some(Menubar {
            mtm,
            _item: status_item,
            _handler: handler,
            _delegate: delegate,
            menu,
            status,
            permission,
            login,
            checks,
            config: config.clone(),
            shown: None,
        })
    });
}

/// Services AppKit events (menu clicks) for `duration`.
pub fn pump(duration: Duration) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let app = NSApplication::sharedApplication(mtm);
    let until = NSDate::dateWithTimeIntervalSinceNow(duration.as_secs_f64());
    let end = std::time::Instant::now() + duration;
    loop {
        let event = unsafe {
            app.nextEventMatchingMask_untilDate_inMode_dequeue(NSEventMask::Any, Some(&until), NSDefaultRunLoopMode, true)
        };
        match event {
            Some(e) => app.sendEvent(&e),
            None => break,
        }
        if std::time::Instant::now() >= end {
            break;
        }
    }
}

/// Applies pending menu choices and refreshes what the menu shows.
pub fn tick(status: Status) {
    let actions: Vec<isize> = std::mem::take(&mut *ACTIONS.lock().unwrap());
    MENU.with(|m| {
        let mut guard = m.borrow_mut();
        let Some(mb) = guard.as_mut() else { return };
        for tag in actions {
            mb.handle(tag);
        }
        let access = has_screen_access();
        let text = match (status, access) {
            (_, false) => Some("Needs Screen Recording permission"),
            (Status::Starting, _) => Some("Starting…"),
            (Status::Waiting, _) => Some("Waiting for the tablet"),
            (Status::Streaming, _) => Some("Streaming to the tablet"),
        };
        if mb.shown != Some((status, access)) {
            mb.shown = Some((status, access));
            if let Some(t) = text {
                mb.status.setTitle(&title(t));
            }
            mb.permission.setHidden(access);
        }
        mb.refresh_checks();
        if REOPEN.swap(false, Ordering::Relaxed) {
            let app = NSApplication::sharedApplication(mb.mtm);
            #[allow(deprecated)]
            app.activateIgnoringOtherApps(true);
            let at = NSEvent::mouseLocation();
            mb.menu.popUpMenuPositioningItem_atLocation_inView(None, at, None);
        }
    });
}

impl Menubar {
    fn refresh_checks(&self) {
        let c = &self.config;
        for (tag, it) in &self.checks {
            let on = match *tag {
                t if t == ORIENTATION => !c.portrait,
                t if t == ORIENTATION + 1 => c.portrait,
                t if t == CODEC => !c.hevc,
                t if t == CODEC + 1 => c.hevc,
                t if (FPS..FPS + 1000).contains(&t) => (t - FPS) as u32 == c.fps,
                t if t >= BITRATE => (t - BITRATE) as u32 == c.bitrate,
                _ => false,
            };
            it.setState(if on { NSControlStateValueOn } else { NSControlStateValueOff });
        }
        self.login.setState(if login_enabled() { NSControlStateValueOn } else { NSControlStateValueOff });
        let _ = self.mtm;
    }

    fn handle(&mut self, tag: isize) {
        let before = self.config.clone();
        match tag {
            QUIT => std::process::exit(0),
            LOGIN => set_login(!login_enabled()),
            PERMISSION => {
                unsafe { CGRequestScreenCaptureAccess() };
                let _ = std::process::Command::new("open")
                    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
                    .spawn();
            }
            t if t == ORIENTATION => self.config.portrait = false,
            t if t == ORIENTATION + 1 => self.config.portrait = true,
            t if t == CODEC => self.config.hevc = false,
            t if t == CODEC + 1 => self.config.hevc = true,
            t if (FPS..FPS + 1000).contains(&t) => self.config.fps = (t - FPS) as u32,
            t if t >= BITRATE => self.config.bitrate = (t - BITRATE) as u32,
            _ => {}
        }
        if self.config != before {
            self.config.save();
            relaunch();
        }
    }
}

/// The `.app` bundle this process runs from, if any.
fn bundle_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app")).map(|p| p.to_path_buf())
}

/// Starts a fresh copy after this one has gone (the old virtual display has to disappear first),
/// then exits.
fn relaunch() -> ! {
    let target = match bundle_path() {
        Some(b) => format!("open -n '{}'", b.display()),
        None => format!("'{}' --menubar", std::env::current_exe().unwrap_or_default().display()),
    };
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("sleep 1.5; exec {target}"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    std::process::exit(0);
}

const AGENT: &str = "Library/LaunchAgents/app.ligarview.host.plist";

fn agent_path() -> Option<std::path::PathBuf> {
    Some(std::path::PathBuf::from(std::env::var_os("HOME")?).join(AGENT))
}

fn login_enabled() -> bool {
    agent_path().is_some_and(|p| p.exists())
}

fn set_login(on: bool) {
    let Some(p) = agent_path() else { return };
    if !on {
        let _ = std::fs::remove_file(p);
        return;
    }
    let args = match bundle_path() {
        Some(b) => format!("<string>/usr/bin/open</string><string>{}</string>", b.display()),
        None => format!(
            "<string>{}</string><string>--menubar</string>",
            std::env::current_exe().unwrap_or_default().display()
        ),
    };
    let plist = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict>\n<key>Label</key><string>app.ligarview.host</string>\n<key>ProgramArguments</key><array>{args}</array>\n<key>RunAtLoad</key><true/>\n</dict></plist>\n"
    );
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(p, plist);
}

/// True when launched from inside a `.app` bundle.
pub fn launched_from_bundle() -> bool {
    bundle_path().is_some()
}
