# Changelog

All notable changes to Ligarview. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/). Update the Mac app and the Android app together:
the Mac app tells the tablet what it allows, and an older tablet app ignores that.

## [0.2.0] - 2026-11-27

The tablet can now control the Mac. Everything that does so is **off until you switch it on** in the Mac window.

### Added

- **S Pen control.** Hover moves the pointer, touching clicks and drags, the pen's side button is a right-click,
  quick taps are double and triple clicks, and pressure is sent with the click.
- **Finger scrolling and zoom.** One or two fingers scroll in both directions and keep gliding after a flick.
  Two fingers apart or together zoom (sent as Ctrl and scroll, which Chromium-based apps, Figma and many map and
  drawing apps read as zoom). Fingers never click.
- **A sidebar in full screen**, opened from the tab on the left edge: Cmd, Shift, Ctrl and Option you tap to hold
  or double-tap to lock, Esc, Undo and Redo, Dock and menu bar buttons, a keyboard button and Disconnect.
- **On-screen keyboard.** Type on the tablet and it types on the Mac: text, Backspace, Return, Tab, Delete and
  the arrow keys. Held sidebar modifiers apply, so Cmd and a letter is a shortcut.
- **Mirror mode.** A new **Display** setting shows your main display on the tablet instead of adding an extra one.
- **A Connection setting** in the Mac window, USB cable or Wi-Fi. Only that transport runs: USB does not listen on
  the network and shows no pairing controls.
- Three switches in the Mac window, **Allow pen control**, **Allow finger scrolling** and **Allow typing from the
  tablet**, all off by default. The Mac asks macOS for the **Accessibility** permission when you turn one on.
- The Mac tells the tablet what it allows, so a sidebar button that is not allowed is dimmed and says which
  switch to turn on.
- A documentation page for all of this, and real screenshots in place of the drawn mockups on the website.

### Changed

- The tablet's **Address** field is gone. The Mac is found over the network and pairing uses the code. For
  development, `adb shell am start -n app.ligarview.client/.MainActivity --es host 127.0.0.1:7878` still connects
  to an address (for `adb reverse`).
- The Wi-Fi tab says the cable is in use instead of waiting for a Mac, and plugging the cable in selects the USB tab.
- The long-press statistics overlay is removed. For development it is shown with `--ez stats true`.
- A tablet paired before the Connection setting existed keeps using Wi-Fi; new installs default to USB.
- The connect screen no longer promises an extra display, since mirror mode does not create one.

### Notes

- Typing is limited to text and a few editing keys, and coordinates and text are checked on the Mac.
- Over Wi-Fi the connection is not encrypted and the pairing code is a convenience, not strong authentication:
  turn the control switches on only on a network you trust, or use USB.
- Not yet: pen tilt, finger tap to click, three-finger gestures, and zoom in apps that only understand the
  trackpad's own pinch (Safari, Preview).

## [0.1.0] - 2026-11-27

First release.

### Added

- The tablet as a real extended display for the Mac, over a USB cable (Android Open Accessory, no USB debugging)
  or Wi-Fi, with hardware H.264 or HEVC encoding and decoding.
- Wi-Fi pairing from the Mac with a six-digit code, and Start and Stop sharing.
- The cursor is drawn on the tablet itself, so it never waits for the video.
- A Mac app with a small window and an Android app with an information screen, a live preview, full screen and
  recommendations read from the tablet.
- Release builds: a disk image for the Mac and an APK for Android.

[0.2.0]: https://github.com/kykurniawan/ligarview/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kykurniawan/ligarview/releases/tag/v0.1.0
