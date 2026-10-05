#!/bin/sh
set -eu
installer_path=$0
payload_line=$(awk '/^__WARPAI_PAYLOAD__$/ { print NR + 1; exit }' "$installer_path")
[ -n "$payload_line" ] || { echo 'Incomplete Warpai installer.' >&2; exit 1; }
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT HUP INT TERM
tail -n "+$payload_line" "$installer_path" | tar -xz -C "$work_dir"
sh "$work_dir/install-unix.sh"
exit
__WARPAI_PAYLOAD__
