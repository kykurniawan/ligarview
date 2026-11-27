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
    Pair,
    Unpair,
    Tablet,
    Input,
    Finger,
    Connection,
}

impl Control {
    fn from_tag(t: isize) -> Self {
        [Self::Orientation, Self::FrameRate, Self::Quality, Self::Codec, Self::Login, Self::Permission, Self::Sharing, Self::Pair, Self::Unpair, Self::Tablet, Self::Input, Self::Finger, Self::Connection][t as usize]
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
    tablet: Retained<NSPopUpButton>,
    code_field: Retained<NSTextField>,
    message: Retained<NSTextField>,
    tablets_shown: Vec<String>,
    tablet_row: Retained<NSStackView>,
    code_row: Retained<NSStackView>,
    unpair: Retained<NSButton>,
    connection: Retained<NSPopUpButton>,
    login: Retained<NSButton>,
    input: Retained<NSButton>,
    finger: Retained<NSButton>,
    input_note: Retained<NSTextField>,
    orientation: Retained<NSPopUpButton>,
    fps: Retained<NSPopUpButton>,
    quality: Retained<NSPopUpButton>,
    codec: Retained<NSPopUpButton>,
    config: Config,
    shown: Option<(Status, bool, bool, String, bool)>,
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
    let tablet = popup(mtm, &handler, Control::Tablet, &["No tablet found".into()], 0);
    let code_field = NSTextField::textFieldWithString(&s(""), mtm);
    code_field.setPlaceholderString(Some(&s("6-digit code")));
    code_field.widthAnchor().constraintEqualToConstant(110.0).setActive(true);
    let pair = unsafe { NSButton::buttonWithTitle_target_action(&s("Connect"), Some(&*handler), Some(sel!(press:)), mtm) };
    pair.setTag(Control::Pair as isize);
    let unpair = unsafe { NSButton::buttonWithTitle_target_action(&s("Unpair"), Some(&*handler), Some(sel!(press:)), mtm) };
    unpair.setTag(Control::Unpair as isize);
    let code_row = NSStackView::stackViewWithViews(&NSArray::from_slice(&[&*code_field as &NSView, &pair]), mtm);
    code_row.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
    code_row.setSpacing(10.0);
    let sharing_row = NSStackView::stackViewWithViews(&NSArray::from_slice(&[&*sharing as &NSView, &unpair]), mtm);
    sharing_row.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
    sharing_row.setSpacing(10.0);
    tablet.widthAnchor().constraintGreaterThanOrEqualToConstant(170.0).setActive(true);
    let message = label(mtm, "Open Ligarview on the tablet, then enter the code it shows.");
    message.setTextColor(Some(&NSColor::tertiaryLabelColor()));
    message.setFont(Some(&NSFont::systemFontOfSize(11.0)));
    let tablet_row = row(mtm, "Tablet", &tablet);
    let code_row = row(mtm, "Code", &code_row);

    // How the tablet connects. Wi-Fi shows the pairing controls; USB needs none of them.
    let connection = popup(mtm, &handler, Control::Connection, &["USB cable".into(), "Wi-Fi".into()], config.wifi as usize);
    let connection_row = row(mtm, "Connection", &connection);

    // Settings.
    let orientation = popup(mtm, &handler, Control::Orientation, &["Landscape".into(), "Portrait".into()], config.portrait as usize);
    let fps_names: Vec<String> = FPS_CHOICES.iter().map(|f| format!("{f} fps")).collect();
    let fps = popup(mtm, &handler, Control::FrameRate, &fps_names, FPS_CHOICES.iter().position(|f| *f == config.fps).unwrap_or(1));
    let q_names: Vec<String> = BITRATE_CHOICES.iter().map(|(m, n)| format!("{n} ({m} Mbps)")).collect();
    let quality = popup(mtm, &handler, Control::Quality, &q_names, BITRATE_CHOICES.iter().position(|(m, _)| *m == config.bitrate).unwrap_or(1));
    let codec_popup = popup(mtm, &handler, Control::Codec, &["H.264".into(), "HEVC".into()], config.hevc as usize);
    let login = unsafe { NSButton::checkboxWithTitle_target_action(&s("Start at login"), Some(&*handler), Some(sel!(press:)), mtm) };
    login.setTag(Control::Login as isize);
    let input = unsafe { NSButton::checkboxWithTitle_target_action(&s("Allow pen control from the tablet"), Some(&*handler), Some(sel!(press:)), mtm) };
    input.setTag(Control::Input as isize);
    let finger = unsafe { NSButton::checkboxWithTitle_target_action(&s("Allow finger scrolling from the tablet"), Some(&*handler), Some(sel!(press:)), mtm) };
    finger.setTag(Control::Finger as isize);
    let input_note = label(mtm, "Needs Accessibility permission in System Settings.");
    input_note.setTextColor(Some(&NSColor::systemOrangeColor()));
    input_note.setFont(Some(&NSFont::systemFontOfSize(11.0)));
    input_note.setHidden(true);
    let hint = label(mtm, "Changing a setting restarts the stream for a moment.");
    hint.setTextColor(Some(&NSColor::tertiaryLabelColor()));
    hint.setFont(Some(&NSFont::systemFontOfSize(11.0)));

    let rows: Vec<Retained<NSStackView>> = vec![
        row(mtm, "Orientation", &orientation),
        row(mtm, "Frame rate", &fps),
        row(mtm, "Quality", &quality),
        row(mtm, "Codec", &codec_popup),
    ];
    let mut views: Vec<&NSView> = vec![&header, &permission_row, &connection_row, &sharing_row, &tablet_row, &code_row, &message];
    views.extend(rows.iter().map(|r| &***r));
    views.push(&login);
    views.push(&input);
    views.push(&finger);
    views.push(&input_note);
    views.push(&hint);
    let root = NSStackView::stackViewWithViews(&NSArray::from_slice(&views), mtm);
    root.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
    root.setAlignment(objc2_app_kit::NSLayoutAttribute::Leading);
    root.setSpacing(14.0);
    root.setCustomSpacing_afterView(22.0, &header);
    root.setCustomSpacing_afterView(4.0, &code_row);
    root.setCustomSpacing_afterView(22.0, &message);
    root.setEdgeInsets(NSEdgeInsets { top: 24.0, left: 24.0, bottom: 20.0, right: 24.0 });

    let size = root.fittingSize();
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            NSRect { origin: NSPoint { x: 0.0, y: 0.0 }, size: NSSize { width: size.width.max(380.0), height: size.height } },
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
            tablet,
            code_field,
            message,
            tablets_shown: Vec::new(),
            tablet_row,
            code_row,
            unpair,
            connection,
            login,
            input,
            finger,
            input_note,
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
        let tablets = crate::pairing::tablets();
        if tablets != gui.tablets_shown {
            let keep = gui.tablet.titleOfSelectedItem().map(|t| t.to_string());
            gui.tablet.removeAllItems();
            if tablets.is_empty() {
                gui.tablet.addItemWithTitle(&s("No tablet found"));
            }
            for t in &tablets {
                gui.tablet.addItemWithTitle(&s(t));
            }
            gui.tablet.selectItemAtIndex(0);
            if let Some(k) = keep {
                let i = gui.tablet.indexOfItemWithTitle(&s(&k));
                if i >= 0 {
                    gui.tablet.selectItemAtIndex(i);
                }
            }
            gui.tablets_shown = tablets;
        }
        if let Some(m) = crate::pairing::take_message() {
            gui.message.setStringValue(&s(&m));
        }
        gui.sharing.setTitle(&s(if status == Status::Off { "Start sharing" } else { "Stop sharing" }));
        let paired = crate::access::code() != 0;
        let name = crate::access::tablet();
        let wifi = gui.config.wifi;
        if gui.shown != Some((status, access, paired, name.clone(), wifi)) {
            gui.shown = Some((status, access, paired, name.clone(), wifi));
            let who = if name.is_empty() || !wifi { "the tablet".to_string() } else { name.clone() };
            let (text, color) = match (status, access) {
                (Status::Off, _) => ("Sharing is off", NSColor::systemOrangeColor()),
                (_, false) => ("Needs Screen Recording permission", NSColor::systemOrangeColor()),
                (Status::Starting, _) => ("Starting…", NSColor::secondaryLabelColor()),
                (Status::Waiting, _) if !wifi => ("Waiting for the tablet on the USB cable", NSColor::secondaryLabelColor()),
                (Status::Waiting, _) if paired => (&*format!("Waiting for {who}"), NSColor::secondaryLabelColor()),
                (Status::Waiting, _) => ("Not paired yet", NSColor::secondaryLabelColor()),
                (Status::Streaming, _) => (&*format!("Streaming to {who}"), NSColor::systemGreenColor()),
            };
            gui.status.setStringValue(&s(text));
            gui.status.setTextColor(Some(&color));
            // Pairing controls only matter while nothing is paired; Unpair only while something is.
            gui.permission_row.setHidden(access);
            gui.tablet_row.setHidden(!wifi || paired);
            gui.code_row.setHidden(!wifi || paired);
            gui.message.setHidden(!wifi || paired);
            gui.unpair.setHidden(!wifi || !paired);
            // The window grows or shrinks with what is shown.
            if let Some(root) = gui.window.contentView() {
                let size = root.fittingSize();
                gui.window.setContentSize(NSSize { width: size.width.max(380.0), height: size.height });
            }
        }
        let control = crate::access::control_enabled();
        gui.input.setState(if control { NSControlStateValueOn } else { NSControlStateValueOff });
        let finger = crate::access::finger_enabled();
        gui.finger.setState(if finger { NSControlStateValueOn } else { NSControlStateValueOff });
        gui.input_note.setHidden(!(control || finger) || crate::input::trusted());
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
                self.config.code = crate::access::code();
                self.config.tablet = crate::access::tablet();
                self.config.save();
                return;
            }
            Control::Unpair => {
                crate::access::set_code(0);
                crate::access::set_tablet(String::new());
                self.config.code = 0;
                self.config.tablet = String::new();
                self.config.save();
                self.message.setStringValue(&s("Unpaired. Pair a tablet again with the code it shows."));
                return;
            }
            Control::Tablet => return,
            Control::Finger => {
                let on = !crate::access::finger_enabled();
                crate::access::set_finger(on);
                self.config.finger = on;
                self.config.code = crate::access::code();
                self.config.tablet = crate::access::tablet();
                self.config.save();
                if on && !crate::input::request_trust() {
                    crate::input::open_accessibility_settings();
                }
                return;
            }
            Control::Input => {
                let on = !crate::access::control_enabled();
                crate::access::set_control(on);
                self.config.control = on;
                self.config.code = crate::access::code();
                self.config.tablet = crate::access::tablet();
                self.config.save();
                if on && !crate::input::request_trust() {
                    crate::input::open_accessibility_settings();
                }
                return;
            }
            Control::Pair => {
                let name = self.tablet.titleOfSelectedItem().map(|n| n.to_string()).unwrap_or_default();
                let digits: String = self.code_field.stringValue().to_string().chars().filter(|c| c.is_ascii_digit()).collect();
                if !self.tablets_shown.contains(&name) {
                    self.message.setStringValue(&s("Open Ligarview on the tablet first."));
                } else if digits.len() != 6 {
                    self.message.setStringValue(&s("Enter the 6-digit code shown on the tablet."));
                } else {
                    self.message.setStringValue(&s("Connecting…"));
                    crate::pairing::pair(name, digits.parse().unwrap_or(0));
                    self.code_field.setStringValue(&s(""));
                }
                return;
            }
            Control::Login => set_login(!login_enabled()),
            Control::Permission => {
                unsafe { CGRequestScreenCaptureAccess() };
                let _ = std::process::Command::new("open")
                    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
                    .spawn();
            }
            Control::Connection => self.config.wifi = value == 1,
            Control::Orientation => self.config.portrait = value == 1,
            Control::Codec => self.config.hevc = value == 1,
            Control::FrameRate => self.config.fps = FPS_CHOICES[value as usize],
            Control::Quality => self.config.bitrate = BITRATE_CHOICES[value as usize].0,
        }
        let _ = (&self.orientation, &self.fps, &self.quality, &self.codec, &self.connection);
        self.config.code = crate::access::code();
        self.config.tablet = crate::access::tablet();
        self.config.sharing = crate::access::enabled();
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
