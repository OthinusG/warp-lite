#!/usr/bin/env bash
# Build only the Linux Companion in an owned WSL guest on a Windows CI runner.
set -euo pipefail
[[ "${GITHUB_ACTIONS:-}" == true ]] || { echo 'Disposable GitHub runner required' >&2; exit 1; }
[[ "$(id -u)" == 0 ]] || { echo 'Owned WSL setup requires root' >&2; exit 1; }
git lfs version >/dev/null
source_repository="$(pwd)"
source_revision="$(git -c safe.directory="$source_repository" rev-parse HEAD)"
fixture_user=warpai-test
id "$fixture_user" >/dev/null 2>&1 || useradd --create-home --shell /bin/bash "$fixture_user"
id warpai-other >/dev/null 2>&1 || useradd --create-home --shell /bin/bash warpai-other
source_directory=/home/warpai-test/warpai-source
mkdir -p "$source_directory"
git -c safe.directory="$source_repository" archive "$source_revision" | tar -x -C "$source_directory"
chown -R "$fixture_user:$fixture_user" "$source_directory"
su - "$fixture_user" -s /bin/bash -- -s "$source_repository" "$source_revision" <<'GUEST'
set -euo pipefail
windows_repository=$1
source_revision=$2
cd /home/warpai-test/warpai-source
curl --fail --location --proto '=https' --tlsv1.2 --max-time 120 https://sh.rustup.rs -o /home/warpai-test/rustup-install.sh
bash /home/warpai-test/rustup-install.sh -y --profile minimal --default-toolchain 1.92.0
source /home/warpai-test/.cargo/env
cargo build -p warp-agent-bus --bin warpai-companion --locked
test "$(target/debug/warpai-companion --version)" = 'warpai-companion 4.0.0 protocol 1'
cargo test -p warp-agent-bus --test managed_agent --locked
agent_fixture=$(find target/debug/deps -maxdepth 1 -type f -executable -name 'managed_agent-*' -print -quit)
[[ -n "$agent_fixture" ]]
install -D -m 755 "$agent_fixture" /home/warpai-test/managed-agent-fixture
cargo build -p warp-agent-bus --bin warpai-wsl-companion --features wsl_companion --locked
cargo test -p warp-agent-bus --lib wsl_chunks_ --features wsl_companion --locked
cargo test -p warp-agent-bus --lib companion::guests::tests --features wsl_companion --locked
cargo test -p warp-agent-bus --lib setup::guest_tests --features wsl_companion --locked
cargo test -p warp-agent-bus --lib wsl_setup::tests --features wsl_companion --locked
cargo build --release -p warp-agent-bus --bin warpai-wsl-companion --features wsl_companion --locked
python3 script/companion/package-wsl.py target/release/warpai-wsl-companion "$windows_repository/target/wsl-bundle/wsl-companion.tar.gz" "$source_revision"
install -D -m 755 target/debug/warpai-wsl-companion /home/warpai-test/.config/.warpai/wsl/bin/warpai-wsl-companion
project='/home/warpai-test/project spaces 多语言'
mkdir -p "$project"
cd "$project"
printf 'original\0bytes\377' > binary.dat
printf 'fn main() { println!("WSL fixture"); }\n' > example.rs
printf '# WSL project\n\nOwned Markdown preview.\n\n![Owned image](image.png)\n' > preview.md
python3 - <<'IMAGE'
import struct, zlib
from pathlib import Path
def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 240, 80, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress((b'\0' + b'\x44\x77\xaa' * 240) * 80)) + chunk(b'IEND', b'')
Path('image.png').write_bytes(png)
IMAGE
git init -q
git -c core.hooksPath= -c user.name='Owned WSL Fixture' -c user.email=fixture@example.invalid add binary.dat example.rs preview.md image.png
git -c core.hooksPath= -c user.name='Owned WSL Fixture' -c user.email=fixture@example.invalid commit -qm 'Owned WSL fixture'
printf 'outside' > /home/warpai-test/outside
test "$(/home/warpai-test/.config/.warpai/wsl/bin/warpai-wsl-companion --version)" = 'warpai-companion 4.0.0 protocol 1'
install -m 755 /home/warpai-test/warpai-source/script/test-wsl-native-agent.py /home/warpai-test/native-agent-fixture.py
GUEST
install -D -m 755 /home/warpai-test/warpai-source/target/debug/warpai-wsl-companion /home/warpai-other/.config/.warpai/wsl/bin/warpai-wsl-companion
install -d -o warpai-other -g warpai-other -m 700 /home/warpai-other/project
chown -R warpai-other:warpai-other /home/warpai-other/.config
chmod 700 /home/warpai-other/.config /home/warpai-other/.config/.warpai
