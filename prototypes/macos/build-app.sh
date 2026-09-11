#!/bin/sh
set -eu
prototype_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_dir=$(CDPATH= cd -- "$prototype_dir/../.." && pwd)
cargo build --manifest-path "$project_dir/Cargo.toml" -p fishing-core --bin fishing-core-cli
swift build --package-path "$prototype_dir" -c release
binary_dir=$(swift build --package-path "$prototype_dir" -c release --show-bin-path)
app_dir="$prototype_dir/build/Arcane Native Comparison.app"
mkdir -p "$app_dir/Contents/MacOS"
cp "$binary_dir/ArcaneNative" "$app_dir/Contents/MacOS/ArcaneNative"
cat > "$app_dir/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>com.aus1273.arcane.native-comparison</string>
<key>CFBundleName</key><string>Arcane Native Comparison</string>
<key>CFBundleExecutable</key><string>ArcaneNative</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>1</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>LSMinimumSystemVersion</key><string>14.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign --force --sign - "$app_dir"
printf 'Built: %s\n' "$app_dir"
if [ "${1:-}" = "--open" ]; then
  open "$app_dir" --args --project "$project_dir"
fi
