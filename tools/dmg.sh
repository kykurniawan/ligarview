#!/bin/sh
# Builds dist/Ligarview-<version>-macos-arm64.dmg: the app plus an Applications shortcut.
# The app is ad-hoc signed, not notarized (see docs/install).
set -e
cd "$(dirname "$0")/.."
./tools/bundle.sh
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' host/Cargo.toml | head -1)
OUT=dist/Ligarview-$VERSION-macos-arm64.dmg
STAGE=$(mktemp -d)/Ligarview
mkdir -p "$STAGE" dist
cp -R host/target/Ligarview.app "$STAGE/"
ln -s /Applications "$STAGE/Applications"
rm -f "$OUT"
hdiutil create -volname "Ligarview" -srcfolder "$STAGE" -ov -format UDZO "$OUT" >/dev/null
echo "Built $OUT"
