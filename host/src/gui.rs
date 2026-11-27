//! The small settings window (and Dock icon) of the Ligarview app.
//!
//! The host's supervisor loop stays on the main thread and pumps AppKit events through
//! [`pump`] instead of a bare run loop, so the window works without a separate UI thread.
//! Changing a setting saves it and relaunches the host, which rebuilds the display from scratch.

use crate::config::Config;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, ProtocolObject};
use objc2::{define_class, msg_send, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSBackingStoreType, NSButton,
    NSColor, NSControlStateValueOff, NSControlStateValueOn, NSEventMask, NSFont, NSImage,
    NSImageView, NSLayoutConstraintOrientation, NSMenu, NSMenuItem, NSPopUpButton, NSStackView,
    NSTextField, NSUserInterfaceLayoutOrientation, NSView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{
    NSArray, NSData, NSDate, NSDefaultRunLoopMode, NSEdgeInsets, NSObjectProtocol, NSPoint, NSRect, NSSize,
    NSString,
};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static REOPEN: AtomicBool = AtomicBool::new(false);
/// Pending UI changes: (control, value).
static ACTIONS: Mutex<Vec<(Control, isize)>> = Mutex::new(Vec::new());

pub fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// What the window shows on its status line.
#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Starting,
    Waiting,
    Streaming,
    Off,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Control {
    Orientation,
    FrameRate,
    Quality,
    Codec,
    Login,
    Permission,
    Sharing,
    NewCode,
}

impl Control {
    fn from_tag(t: isize) -> Self {
        [Self::Orientation, Self::FrameRate, Self::Quality, Self::Codec, Self::Login, Self::Permission, Self::Sharing, Self::NewCode][t as usize]
    }
}

const FPS_CHOICES: [u32; 3] = [30, 60, 90];
const BITRATE_CHOICES: [(u32, &str); 4] = [(20, "Standard"), (30, "High"), (40, "Higher"), (60, "Maximum")];

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "LigarviewHandler"]
    struct Handler;

    impl Handler {
        #[unsafe(method(popup:))]
        fn popup(&self, sender: &NSPopUpButton) {
            let value = sender.indexOfSelectedItem();
            ACTIONS.lock().unwrap().push((Control::from_tag(sender.tag()), value));
        }

        #[unsafe(method(press:))]
        fn press(&self, sender: &NSButton) {
            ACTIONS.lock().unwrap().push((Control::from_tag(sender.tag()), 0));
        }
    }
);

impl Handler {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

define_class!(
    /// Clicking the Dock icon (or opening the app again) brings the window back.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "LigarviewAppDelegate"]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl NSApplicationDelegate for Delegate {
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn reopen(&self, _app: &NSApplication, _visible: bool) -> bool {
            REOPEN.store(true, Ordering::Relaxed);
            false
        }
    }
);

struct Gui {
    mtm: MainThreadMarker,
    window: Retained<NSWindow>,
    _handler: Retained<Handler>,
    _delegate: Retained<Delegate>,
    status: Retained<NSTextField>,
    permission_row: Retained<NSStackView>,
    sharing: Retained<NSButton>,
    code: Retained<NSTextField>,
    login: Retained<NSButton>,
    orientation: Retained<NSPopUpButton>,
    fps: Retained<NSPopUpButton>,
    quality: Retained<NSPopUpButton>,
    codec: Retained<NSPopUpButton>,
    config: Config,
    shown: Option<(Status, bool, u32)>,
}

thread_local! {
    static GUI: RefCell<Option<Gui>> = const { RefCell::new(None) };
}

fn s(text: &str) -> Retained<NSString> {
    NSString::from_str(text)
}

fn label(mtm: MainThreadMarker, text: &str) -> Retained<NSTextField> {
    NSTextField::labelWithString(&s(text), mtm)
}

fn popup(
    mtm: MainThreadMarker,
    handler: &Handler,
    control: Control,
    entries: &[String],
    selected: usize,
) -> Retained<NSPopUpButton> {
    let p = NSPopUpButton::initWithFrame_pullsDown(mtm.alloc(), NSRect::ZERO, false);
    for e in entries {
        p.addItemWithTitle(&s(e));
    }
    p.selectItemAtIndex(selected as isize);
    p.setTag(control as isize);
    unsafe {
        p.setTarget(Some(handler));
        p.setAction(Some(sel!(popup:)));
    }
    p
}

fn row(mtm: MainThreadMarker, name: &str, control: &NSView) -> Retained<NSStackView> {
    let l = label(mtm, name);
    l.setTextColor(Some(&NSColor::secondaryLabelColor()));
    let views = NSArray::from_slice(&[&*l as &NSView, control]);
    let r = NSStackView::stackViewWithViews(&views, mtm);
    r.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
    r.setSpacing(12.0);
    // Same label column width in every row so the controls line up.
    l.setContentHuggingPriority_forOrientation(750.0, NSLayoutConstraintOrientation::Horizontal);
    let w = l.widthAnchor();
    w.constraintEqualToConstant(96.0).setActive(true);
    r
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

pub fn has_screen_access() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

/// Creates the window. Call once, on the main thread, before the supervisor loop.
pub fn init(config: &Config, resolution: (u32, u32), codec: &str, show: bool) {
    let mtm = MainThreadMarker::new().expect("the window must be set up on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    let delegate: Retained<Delegate> = {
        let this = mtm.alloc::<Delegate>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    };
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    add_main_menu(mtm, &app);
    app.finishLaunching();
    let handler = Handler::new(mtm);

    // Header: icon, name, status.
    let title = label(mtm, "Ligarview");
    title.setFont(Some(&NSFont::boldSystemFontOfSize(20.0)));
    let status = label(mtm, "Starting…");
    status.setFont(Some(&NSFont::systemFontOfSize(13.0)));
    let detail = label(mtm, &format!("{}x{} · {} fps · {codec}", resolution.0, resolution.1, config.fps));
    detail.setTextColor(Some(&NSColor::secondaryLabelColor()));
    detail.setFont(Some(&NSFont::systemFontOfSize(12.0)));
    let text = NSStackView::stackViewWithViews(&NSArray::from_slice(&[&*title as &NSView, &status, &detail]), mtm);
    text.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
    text.setAlignment(objc2_app_kit::NSLayoutAttribute::Leading);
    text.setSpacing(2.0);
    let header = NSStackView::new(mtm);
    header.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
    header.setSpacing(14.0);
    let data = NSData::with_bytes(include_bytes!("../assets/app-icon.png"));
    if let Some(img) = NSImage::initWithData(mtm.alloc(), &data) {
        let iv = NSImageView::imageViewWithImage(&img, mtm);
        iv.widthAnchor().constraintEqualToConstant(52.0).setActive(true);
        iv.heightAnchor().constraintEqualToConstant(52.0).setActive(true);
        header.addArrangedSubview(&iv);
    }
    header.addArrangedSubview(&text);

    // Permission notice, only while Screen Recording is not allowed.
    let note = label(mtm, "Screen Recording is not allowed yet.");
    let allow = unsafe { NSButton::buttonWithTitle_target_action(&s("Allow…"), Some(&*handler), Some(sel!(press:)), mtm) };
    allow.setTag(Control::Permission as isize);
    let permission_row = NSStackView::stackViewWithViews(&NSArray::from_slice(&[&*note as &NSView, &allow]), mtm);
    permission_row.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
    permission_row.setSpacing(10.0);
    permission_row.setHidden(true);

    // Connection: sharing switch and pairing code, managed here rather than on the tablet.
    let sharing = unsafe { NSButton::buttonWithTitle_target_action(&s("Stop sharing"), Some(&*handler), Some(sel!(press:)), mtm) };
    sharing.setTag(Control::Sharing as isize);
    let code = label(mtm, &format_code(config.code));
    code.setFont(Some(&NSFont::monospacedSystemFontOfSize_weight(22.0, 0.23)));
    code.setSelectable(true);
    let new_code = unsafe { NSButton::buttonWithTitle_target_action(&s("New code"), Some(&*handler), Some(sel!(press:)), mtm) };
    new_code.setTag(Control::NewCode as isize);
    let code_row = NSStackView::stackViewWithViews(&NSArray::from_slice(&[&*code as &NSView, &new_code]), mtm);
    code_row.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
    code_row.setSpacing(12.0);
    let code_hint = label(mtm, "Enter this code on the tablet once to pair over Wi-Fi. USB needs no code.");
    code_hint.setTextColor(Some(&NSColor::tertiaryLabelColor()));
    code_hint.setFont(Some(&NSFont::systemFontOfSize(11.0)));

    // Settings.
    let orientation = popup(mtm, &handler, Control::Orientation, &["Landscape".into(), "Portrait".into()], config.portrait as usize);
    let fps_names: Vec<String> = FPS_CHOICES.iter().map(|f| format!("{f} fps")).collect();
    let fps = popup(mtm, &handler, Control::FrameRate, &fps_names, FPS_CHOICES.iter().position(|f| *f == config.fps).unwrap_or(1));
    let q_names: Vec<String> = BITRATE_CHOICES.iter().map(|(m, n)| format!("{n} ({m} Mbps)")).collect();
    let quality = popup(mtm, &handler, Control::Quality, &q_names, BITRATE_CHOICES.iter().position(|(m, _)| *m == config.bitrate).unwrap_or(1));
    let codec_popup = popup(mtm, &handler, Control::Codec, &["H.264".into(), "HEVC".into()], config.hevc as usize);
    let login = unsafe { NSButton::checkboxWithTitle_target_action(&s("Start at login"), Some(&*handler), Some(sel!(press:)), mtm) };
    login.setTag(Control::Login as isize);
    let hint = label(mtm, "Changing a setting restarts the stream for a moment.");
    hint.setTextColor(Some(&NSColor::tertiaryLabelColor()));
    hint.setFont(Some(&NSFont::systemFontOfSize(11.0)));

    let rows: Vec<Retained<NSStackView>> = vec![
        row(mtm, "Orientation", &orientation),
        row(mtm, "Frame rate", &fps),
        row(mtm, "Quality", &quality),
        row(mtm, "Codec", &codec_popup),
    ];
    let mut views: Vec<&NSView> = vec![&header, &permission_row, &sharing, &code_row, &code_hint];
    views.extend(rows.iter().map(|r| &***r));
    views.push(&login);
    views.push(&hint);
    let root = NSStackView::stackViewWithViews(&NSArray::from_slice(&views), mtm);
    root.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
    root.setAlignment(objc2_app_kit::NSLayoutAttribute::Leading);
    root.setSpacing(14.0);
    root.setCustomSpacing_afterView(22.0, &header);
    root.setCustomSpacing_afterView(4.0, &code_row);
    root.setCustomSpacing_afterView(22.0, &code_hint);
    root.setEdgeInsets(NSEdgeInsets { top: 24.0, left: 24.0, bottom: 20.0, right: 24.0 });

    let size = root.fittingSize();
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            NSRect { origin: NSPoint { x: 0.0, y: 0.0 }, size: NSSize { width: size.width.max(340.0), height: size.height } },
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe { window.setReleasedWhenClosed(false) };
    window.setTitle(&s("Ligarview"));
    window.setContentView(Some(&root));
    window.center();
    if show {
        window.makeKeyAndOrderFront(None);
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
    }

    if !has_screen_access() {
        unsafe { CGRequestScreenCaptureAccess() };
    }

    ACTIVE.store(true, Ordering::Relaxed);
    GUI.with(|g| {
        *g.borrow_mut() = Some(Gui {
            mtm,
            window,
            _handler: handler,
            _delegate: delegate,
            status,
            permission_row,
            sharing,
            code,
            login,
            orientation,
            fps,
            quality,
            codec: codec_popup,
            config: config.clone(),
            shown: None,
        })
    });
}

/// A minimal main menu so ⌘Q and ⌘W work.
fn add_main_menu(mtm: MainThreadMarker, app: &NSApplication) {
    let bar = NSMenu::new(mtm);
    let app_item = NSMenuItem::new(mtm);
    bar.addItem(&app_item);
    let app_menu = NSMenu::new(mtm);
    let quit = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), &s("Quit Ligarview"), Some(sel!(terminate:)), &s("q"))
    };
    app_menu.addItem(&quit);
    app_item.setSubmenu(Some(&app_menu));
    let win_item = NSMenuItem::new(mtm);
    bar.addItem(&win_item);
    let win_menu = NSMenu::new(mtm);
    win_menu.setTitle(&s("Window"));
    let close = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), &s("Close"), Some(sel!(performClose:)), &s("w"))
    };
    win_menu.addItem(&close);
    win_item.setSubmenu(Some(&win_menu));
    app.setMainMenu(Some(&bar));
}

/// Services AppKit events (clicks) for `duration`.
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

/// Applies pending changes and refreshes what the window shows.
pub fn tick(status: Status) {
    let actions: Vec<(Control, isize)> = std::mem::take(&mut *ACTIONS.lock().unwrap());
    GUI.with(|g| {
        let mut guard = g.borrow_mut();
        let Some(gui) = guard.as_mut() else { return };
        for (c, v) in actions {
            gui.handle(c, v);
        }
        let access = has_screen_access();
        gui.code.setStringValue(&s(&format_code(gui.config.code)));
        gui.sharing.setTitle(&s(if status == Status::Off { "Start sharing" } else { "Stop sharing" }));
        if gui.shown != Some((status, access, gui.config.code)) {
            gui.shown = Some((status, access, gui.config.code));
            let (text, color) = match (status, access) {
                (Status::Off, _) => ("Sharing is off", NSColor::systemOrangeColor()),
                (_, false) => ("Needs Screen Recording permission", NSColor::systemOrangeColor()),
                (Status::Starting, _) => ("Starting…", NSColor::secondaryLabelColor()),
                (Status::Waiting, _) => ("Waiting for the tablet", NSColor::secondaryLabelColor()),
                (Status::Streaming, _) => ("Streaming to the tablet", NSColor::systemGreenColor()),
            };
            gui.status.setStringValue(&s(text));
            gui.status.setTextColor(Some(&color));
            let was_hidden = gui.permission_row.isHidden();
            gui.permission_row.setHidden(access);
            if was_hidden == access {
                // The window grows or shrinks with the notice.
                if let Some(root) = gui.window.contentView() {
                    let size = root.fittingSize();
                    gui.window.setContentSize(NSSize { width: size.width.max(340.0), height: size.height });
                }
            }
        }
        gui.login.setState(if login_enabled() { NSControlStateValueOn } else { NSControlStateValueOff });
        if REOPEN.swap(false, Ordering::Relaxed) {
            gui.window.makeKeyAndOrderFront(None);
            let app = NSApplication::sharedApplication(gui.mtm);
            #[allow(deprecated)]
            app.activateIgnoringOtherApps(true);
        }
    });
}

impl Gui {
    fn handle(&mut self, control: Control, value: isize) {
        let before = self.config.clone();
        match control {
            Control::Sharing => {
                self.config.sharing = !crate::access::enabled();
                crate::access::set_enabled(self.config.sharing);
                self.config.save();
                return;
            }
            Control::NewCode => {
                self.config.code = crate::access::generate_code();
                crate::access::set_code(self.config.code);
                self.config.save();
                return;
            }
            Control::Login => set_login(!login_enabled()),
            Control::Permission => {
                unsafe { CGRequestScreenCaptureAccess() };
                let _ = std::process::Command::new("open")
                    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
                    .spawn();
            }
            Control::Orientation => self.config.portrait = value == 1,
            Control::Codec => self.config.hevc = value == 1,
            Control::FrameRate => self.config.fps = FPS_CHOICES[value as usize],
            Control::Quality => self.config.bitrate = BITRATE_CHOICES[value as usize].0,
        }
        let _ = (&self.orientation, &self.fps, &self.quality, &self.codec);
        if self.config != before {
            self.config.save();
            relaunch();
        }
    }
}

fn format_code(code: u32) -> String {
    let d = format!("{code:06}");
    format!("{} {}", &d[..3], &d[3..])
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
        None => format!("'{}' --gui", std::env::current_exe().unwrap_or_default().display()),
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
        Some(b) => format!(
            "<string>/usr/bin/open</string><string>-a</string><string>{}</string><string>--args</string><string>--background</string>",
            b.display()
        ),
        None => format!(
            "<string>{}</string><string>--gui</string><string>--background</string>",
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
