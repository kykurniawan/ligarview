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

   Defaults to a 1280x800 HiDPI display (2560x1600 real pixels, like a Retina Mac) at 60 fps, 30 Mbps.
   For a 120 Hz panel: `cargo run --release -- --fps 120 --bitrate 60`.

   Options: `--width --height --scale --fps --bitrate <Mbps> --port --stats`.

2. Android: build and install the app (`cd android && ./gradlew installDebug`).
3. Connect:
   - **USB**: enable USB debugging, run `adb reverse tcp:7878 tcp:7878`, open the app and connect to `127.0.0.1:7878`.
   - **Wi-Fi**: open the app and connect to `<mac-ip>:7878` (same network, 5 GHz recommended).

The app reconnects automatically and remembers the last address. Back returns to the connect screen.
Long-press the picture to show a statistics overlay (fps, bitrate, decoder latency).

Debugging without a tablet: `python3 tools/dump_stream.py 127.0.0.1 7878 5 out.h264` records the stream to a file.

## Protocol

See [`host/src/protocol.rs`](host/src/protocol.rs): a 12-byte hello, then length-prefixed Annex-B H.264 frames.

## Notes

Latency: VideoToolbox defaults to 11 reference frames, which makes hardware decoders
buffer 20+ frames before output. The host limits it to 1 (`ReferenceBufferCount`),
which took decoder latency on a Tab S9 from ~500 ms to ~40 ms.

The virtual display uses the private `CGVirtualDisplay` API. It works without
SIP changes or kernel extensions, but Apple may change it in a macOS update.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
