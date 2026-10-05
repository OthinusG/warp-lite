#!/usr/bin/env bash
# Builds a minimal Warpai.app bundle on macOS for Warpai.
#
# Prerequisites:
#   - target/release/warpai already built (cargo build --release --bin warpai)
#
# Usage:
#   script/build-warpai-app.sh
#   script/build-warpai-app.sh --debug  (isolated UI review build)
#
# Output:
#   ./Warpai.app   (drag into /Applications)

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="target/release/warpai"
AGENT_BIN="target/release/warpai-agent"
APP="Warpai.app"
APP_VERSION="${GIT_RELEASE_TAG:-0.1.0}"
APP_VERSION="${APP_VERSION#v}"
[[ "$APP_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
    echo "Error: GIT_RELEASE_TAG must be a semantic release version." >&2
    exit 1
}
APP_SHORT_VERSION="$APP_VERSION"
APP_IDENTIFIER="dev.warpai.Warpai"
SRC_ICON="app/assets/branding/warpai.icns"

case "${1:-}" in
    "") ;;
    --debug)
        BIN="target/debug/warpai"
        AGENT_BIN="target/debug/warpai-agent"
        APP="WarpaiReview.app"
        APP_IDENTIFIER="dev.warpai.WarpaiReview"
        ;;
    *) echo "Usage: $0 [--debug]" >&2; exit 1 ;;
esac

if [[ ! -f "$BIN" ]]; then
    echo "Error: $BIN not found. Run: cargo build --release --bin warpai" >&2
    exit 1
fi

if [[ ! -f "$AGENT_BIN" ]]; then
    echo "Error: $AGENT_BIN not found. Build warp-agent-bus on GitHub before packaging." >&2
    exit 1
fi

if [[ ! -f "$SRC_ICON" ]]; then
    echo "Error: source icon $SRC_ICON not found." >&2
    exit 1
fi

# Wipe any prior bundle
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

# 1) Copy the binary
cp "$BIN" "$APP/Contents/MacOS/Warpai"
cp "$AGENT_BIN" "$APP/Contents/MacOS/warpai-agent"
chmod +x "$APP/Contents/MacOS/Warpai" "$APP/Contents/MacOS/warpai-agent"
if [[ "${1:-}" == --debug ]]; then
    # Keep debug assertions/profile isolation without shipping bulky debug symbols.
    strip -S "$APP/Contents/MacOS/Warpai" "$APP/Contents/MacOS/warpai-agent"
fi

# 2) Reuse the reviewed multiresolution icon without changing its artwork.
cp "$SRC_ICON" "$APP/Contents/Resources/AppIcon.icns"

# 3) Info.plist
cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Warpai</string>
    <key>CFBundleDisplayName</key>
    <string>Warpai</string>
    <key>CFBundleIdentifier</key>
    <string>$APP_IDENTIFIER</string>
    <key>CFBundleVersion</key>
    <string>$APP_VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$APP_SHORT_VERSION</string>
    <key>CFBundleExecutable</key>
    <string>Warpai</string>
    <key>CFBundleURLTypes</key>
    <array><dict>
        <key>CFBundleURLName</key><string>Warpai</string>
        <key>CFBundleURLSchemes</key><array><string>warpai</string></array>
    </dict></array>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>WLIT</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.14</string>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>NSHumanReadableCopyright</key>
    <string>Forked from Warp Terminal © Denver Technologies, Inc. — AGPL-3.0-only</string>
</dict>
</plist>
EOF

# 4) Ad-hoc codesign so Gatekeeper does not refuse the binary outright.
codesign --force --deep --sign - "$APP" >/dev/null

echo "Built $APP ($(du -sh "$APP" | awk '{print $1}'))"
echo "  Identifier: $APP_IDENTIFIER"
echo "  Version:    $APP_SHORT_VERSION"
echo "Drag $APP into /Applications, or run: open $APP"
