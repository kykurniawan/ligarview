#!/bin/sh
# Builds dist/Ligarview-<version>.apk, signed with the release key when LIGARVIEW_KEYSTORE is set
# (see android/app/build.gradle.kts), otherwise with the debug key.
set -e
cd "$(dirname "$0")/.."
(cd android && ./gradlew :app:assembleRelease -q)
VERSION=$(sed -n 's/.*versionName = "\(.*\)".*/\1/p' android/app/build.gradle.kts | head -1)
mkdir -p dist
cp android/app/build/outputs/apk/release/app-release.apk "dist/Ligarview-$VERSION.apk"
echo "Built dist/Ligarview-$VERSION.apk"
