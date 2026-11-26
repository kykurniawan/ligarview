# Ligarview

Use an Android tablet as a real **extended** second display for your Mac, over Wi-Fi or USB.
Think Sidecar, but for Android (built and tested with a Samsung Galaxy Tab S9).

No touch input, no audio, no extras. The tablet is just a monitor.

## How it works

```
Mac (Rust)                                        Android (Kotlin)
CGVirtualDisplay -> ScreenCaptureKit              TCP
  (extra display)    (capture)  -> VideoToolbox  ====>  MediaCodec -> SurfaceView
                                   H.264 (HW)
Wi-Fi: direct TCP  |  USB: `adb reverse` tunnel
```

## Layout

- `host/` - macOS host app (Rust)
- `android/` - Android client (Kotlin)

## Status

Early work in progress.

- [x] Host: virtual display
- [x] Host: capture + hardware H.264 encode
- [x] Transport (TCP, works over Wi-Fi and USB via `adb reverse`)
- [x] Android client (MediaCodec low-latency decode, fullscreen)
- [ ] Latency and quality tuning

## Requirements

- macOS 14+ (developed on macOS 15), Apple Silicon
- Rust (stable)
- Android 11+ (API 30), Android SDK + JDK 17+ to build the app
- Screen Recording permission for the host app

## Usage

1. Mac: start the host (creates the extra display and listens on port 7878).

   ```bash
   cd host
   cargo run --release
   ```

   Options: `--width --height --fps --bitrate <Mbps> --port --hidpi`.

2. Android: build and install the app (`cd android && ./gradlew installDebug`).
3. Connect:
   - **USB**: enable USB debugging, run `adb reverse tcp:7878 tcp:7878`, open the app and connect to `127.0.0.1:7878`.
   - **Wi-Fi**: open the app and connect to `<mac-ip>:7878` (same network, 5 GHz recommended).

The app reconnects automatically and remembers the last address. Back returns to the connect screen.

Debugging without a tablet: `python3 tools/dump_stream.py 127.0.0.1 7878 5 out.h264` records the stream to a file.

## Protocol

See [`host/src/protocol.rs`](host/src/protocol.rs): a 12-byte hello, then length-prefixed Annex-B H.264 frames.

## Notes

The virtual display uses the private `CGVirtualDisplay` API. It works without
SIP changes or kernel extensions, but Apple may change it in a macOS update.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
