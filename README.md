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

   Options: `--width --height --scale --fps --bitrate <Mbps> --port --stats --codec --low-latency-rc --cursor-in-video --no-adb`.
   `--stats` prints capture/encode rates and latencies once per second.

2. Android: build and install the app (`cd android && ./gradlew installDebug`).
3. Connect:
   - **USB**: enable USB debugging, plug the cable in, open the app and connect to `127.0.0.1:7878`. The host sets up (and re-creates) the `adb reverse` tunnels for video and cursor by itself; use `--no-adb` to manage them yourself.
   - **Wi-Fi**: open the app and connect to `<mac-ip>:7878` (same network, 5 GHz recommended). Ports 7878 and 7879 must be reachable.

The app reconnects automatically and remembers the last address. Back returns to the connect screen.
Long-press the picture to show a statistics overlay (fps, bitrate, decoder latency).

Debugging without a tablet: `python3 tools/dump_stream.py 127.0.0.1 7878 5 out.h264` records the video, `python3 tools/dump_cursor.py` prints cursor messages.

## Recovery

You should not have to restart anything:

- Unplug and replug the USB cable: the host re-creates the `adb reverse` tunnels and the app reconnects.
- Silent connection loss (Wi-Fi drop, frozen host): the host sends a heartbeat every second; the app
  treats 4 s of silence as a dead link and reconnects.
- Tablet sleep/lock: the app reconnects when it comes back to the foreground.
- Virtual display or capture lost on the Mac (for example after sleep): the host recreates the
  display, the HiDPI mode, capture and encoder, then asks for a keyframe. `--debug-drop-display-after N`
  simulates this.

## Cursor

The cursor is not part of the video. The host sends its position (and shape, when it changes)
on a second connection (video port + 1) and the tablet draws it on top, once per vsync, so the
cursor does not wait for capture + encode + decode. Use `--cursor-in-video` to fall back to a
cursor rendered into the video.

For a smooth 120 Hz panel on Samsung tablets set Settings > Display > Motion smoothness to
Adaptive and turn off Power saving.

## Protocol

Video: [`host/src/protocol.rs`](host/src/protocol.rs) (a 12-byte hello, then length-prefixed Annex-B H.264 frames).
Cursor: [`host/src/cursor_server.rs`](host/src/cursor_server.rs).

## Notes

Codec: H.264 by default. `--codec hevc` also works (the app picks the decoder from the hello
message) but measured the same latency on an M1 Pro / Tab S9 (~41 ms encode, ~8 ms decode);
its benefit is compression efficiency, not speed.

Encoder: the default VideoToolbox mode sustains ~57 fps on a busy 2560x1600 screen with
~40 ms encode latency. `--low-latency-rc` saves ~10 ms per frame but caps throughput near
30 fps, which makes dragging windows look choppy. Captured frames are never queued: the
encoder always takes the newest one and stale frames are dropped.

Latency: VideoToolbox defaults to 11 reference frames, which makes hardware decoders
buffer 20+ frames before output. The host limits it to 1 (`ReferenceBufferCount`),
which took decoder latency on a Tab S9 from ~500 ms to ~40 ms. On Qualcomm devices the
client also sets `vendor.qti-ext-dec-picture-order.enable`, which removes the last held-back
frame (~40 ms -> ~8 ms at 30 fps).

The virtual display uses the private `CGVirtualDisplay` API. It works without
SIP changes or kernel extensions, but Apple may change it in a macOS update.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
