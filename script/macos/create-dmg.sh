#!/bin/sh
set -eu

test "$#" -eq 3 || { echo 'Usage: create-dmg.sh SOURCE OUTPUT VOLUME' >&2; exit 2; }
test -f "$1/.VolumeIcon.icns"
image_workspace=$(mktemp -d)
image_mounted=0
cleanup() {
    if [ "$image_mounted" -eq 1 ]; then
        hdiutil detach "$image_workspace/mount" >/dev/null 2>&1 || return
    fi
    rm -rf "$image_workspace"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM

hdiutil create -volname "$3" -srcfolder "$1" -ov -format UDRW "$image_workspace/raw.dmg"
mkdir "$image_workspace/mount"
hdiutil attach "$image_workspace/raw.dmg" -nobrowse -mountpoint "$image_workspace/mount"
image_mounted=1
# Source-folder Finder flags do not become disk-volume flags.
xcrun SetFile -a C "$image_workspace/mount"
test "$(xcrun GetFileInfo -aC "$image_workspace/mount")" -eq 1
hdiutil detach "$image_workspace/mount"
image_mounted=0
hdiutil convert "$image_workspace/raw.dmg" -format UDZO -ov -o "$2"
