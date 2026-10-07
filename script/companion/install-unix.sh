#!/bin/sh
# Install only the account-owned Warpai component; leave shell and SSH settings intact.
set -eu
payload_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
case $(uname -s) in
  Linux) platform=Linux ;;
  Darwin) platform=Darwin ;;
  *) echo 'This installer requires Linux or macOS.' >&2; exit 1 ;;
esac
expected_platform=$(cat "$payload_dir/platform")
[ "$platform:$(uname -m)" = "$expected_platform" ] || {
  echo 'Download the companion installer matching this operating system and architecture.' >&2
  exit 1
}
[ -n "${HOME:-}" ] && [ "${HOME#/}" != "$HOME" ] || {
  echo 'An absolute account home directory is required.' >&2; exit 1
}
if [ "$platform" = Darwin ]; then
  (cd "$payload_dir" && shasum -a 256 -c SHA256SUMS)
else
  (cd "$payload_dir" && sha256sum -c SHA256SUMS)
fi
install_dir="$HOME/.config/.warpai/bin"
# Refuse symlinked installation directories rather than writing outside the account tree.
for directory in "$HOME/.config" "$HOME/.config/.warpai" "$install_dir"; do
  [ ! -L "$directory" ] || { echo 'Warpai installation directory must not be a symlink.' >&2; exit 1; }
  mkdir -p "$directory"
done
staged_binary=$(mktemp "$install_dir/.warpai-companion.XXXXXX")
staged_manifest=$(mktemp "$install_dir/.companion-manifest.XXXXXX")
staged_runtime=$(mktemp -d "$install_dir/.companion-runtime.XXXXXX")
trap 'rm -f "$staged_binary" "$staged_manifest"; rm -rf "$staged_runtime"' EXIT HUP INT TERM
runtime_dir=
for directory in "$payload_dir"/companion-runtime-*; do
  [ -d "$directory" ] || continue
  [ -z "$runtime_dir" ] || { echo 'Multiple Companion runtimes in payload.' >&2; exit 1; }
  runtime_dir=$(basename "$directory")
  cp -R "$directory"/. "$staged_runtime"/
done
[ -n "$runtime_dir" ] && [ -x "$staged_runtime/git/bin/git" ] || {
  echo 'Companion Git runtime missing.' >&2; exit 1
}
"$staged_runtime/git/bin/git" --version
if [ -e "$install_dir/$runtime_dir" ]; then
  [ -d "$install_dir/$runtime_dir" ] && [ ! -L "$install_dir/$runtime_dir" ] || exit 1
  # Reinstallation verifies the installed bytes against the incoming trusted checksums.
  if [ "$platform" = Darwin ]; then
    (cd "$install_dir/$runtime_dir" && shasum -a 256 -c "$staged_runtime/SHA256SUMS")
  else
    (cd "$install_dir/$runtime_dir" && sha256sum -c "$staged_runtime/SHA256SUMS")
  fi
else
  mv "$staged_runtime" "$install_dir/$runtime_dir"
fi
cp "$payload_dir/warpai-companion" "$staged_binary"
chmod 700 "$staged_binary"
"$staged_binary" --version
cp "$payload_dir/manifest.json" "$staged_manifest"
chmod 600 "$staged_manifest"
mv -f "$staged_manifest" "$install_dir/companion-manifest.json"
mv -f "$staged_binary" "$install_dir/warpai-companion"
"$install_dir/warpai-companion" --check-runtime
printf 'Installed Warpai Companion at %s\nReconnect your SSH terminal in Warpai to detect it.\n' "$install_dir/warpai-companion"
