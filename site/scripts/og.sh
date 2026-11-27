#!/bin/sh
# Renders static/og.png (1200x630, the Open Graph / Twitter card image) from scripts/og.svg.
# Needs rsvg-convert (brew install librsvg). rsvg only reads images beside the SVG, so copy them first.
set -e
cd "$(dirname "$0")/.."
T=$(mktemp -d)
cp scripts/og.svg static/icon-192.png static/desktop.jpg static/info-screen.png static/mac-window.png "$T/"
rsvg-convert -w 1200 -h 630 "$T/og.svg" -o static/og.png
echo "Wrote static/og.png"
