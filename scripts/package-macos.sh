#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --release --locked
bundle="dist/ClickUp Tracker.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp target/release/clickup-tracker "$bundle/Contents/MacOS/clickup-tracker.next"
mv -f "$bundle/Contents/MacOS/clickup-tracker.next" "$bundle/Contents/MacOS/clickup-tracker"
cp packaging/Info.plist "$bundle/Contents/Info.plist"
cp LICENSE "$bundle/Contents/Resources/LICENSE"
# A local ad-hoc signature is sufficient for running your own build.
codesign --force --sign - "$bundle"
target_triple=$(rustc -vV | sed -n 's/^host: //p')
ditto -c -k --sequesterRsrc --keepParent "$bundle" "dist/clickup-tracker-$target_triple.zip"
printf '\nReady: %s\n' "$bundle"
