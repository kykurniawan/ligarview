#!/bin/sh
# Builds host/target/Ligarview.app: the host as a menu bar app with the Ligarview icon.
set -e
cd "$(dirname "$0")/.."
(cd host && cargo build --release)

APP=host/target/Ligarview.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp host/target/release/ligarview-host "$APP/Contents/MacOS/ligarview-host"

ICONSET=$(mktemp -d)/Ligarview.iconset
mkdir -p "$ICONSET"
SRC=$(mktemp -d)/icon.png
ffmpeg -v error -y -i assets/ligarview-logo.png -vf "crop=1730:1730:160:140" "$SRC"
for s in 16 32 64 128 256 512; do
  sips -z $s $s "$SRC" --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
  sips -z $((s * 2)) $((s * 2)) "$SRC" --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Ligarview.icns"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Ligarview</string>
  <key>CFBundleDisplayName</key><string>Ligarview</string>
  <key>CFBundleIdentifier</key><string>app.ligarview.host</string>
  <key>CFBundleExecutable</key><string>ligarview-host</string>
  <key>CFBundleIconFile</key><string>Ligarview</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST

# A requirement on the identifier alone keeps the Screen Recording permission across rebuilds
# (an ad-hoc signature is otherwise tied to the exact binary).
codesign --force --deep --sign - -r='designated => identifier "app.ligarview.host"' "$APP"
echo "Built $APP"
