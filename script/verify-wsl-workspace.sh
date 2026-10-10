#!/usr/bin/env bash
# Build only the Linux Companion in an owned WSL guest on a Windows CI runner.
set -euo pipefail
[[ "${GITHUB_ACTIONS:-}" == true ]] || { echo 'Disposable GitHub runner required' >&2; exit 1; }
[[ "$(id -u)" == 0 ]] || { echo 'Owned WSL setup requires root' >&2; exit 1; }
source_repository="$(pwd)"
fixture_user=warpai-test
id "$fixture_user" >/dev/null 2>&1 || useradd --create-home --shell /bin/bash "$fixture_user"
id warpai-other >/dev/null 2>&1 || useradd --create-home --shell /bin/bash warpai-other
source_directory=/home/warpai-test/warpai-source
mkdir -p "$source_directory"
git -c safe.directory="$source_repository" archive HEAD | tar -x -C "$source_directory"
chown -R "$fixture_user:$fixture_user" "$source_directory"
su - "$fixture_user" -s /bin/bash <<'GUEST'
set -euo pipefail
cd /home/warpai-test/warpai-source
curl --fail --location --proto '=https' --tlsv1.2 --max-time 120 https://sh.rustup.rs -o /home/warpai-test/rustup-install.sh
bash /home/warpai-test/rustup-install.sh -y --profile minimal --default-toolchain 1.92.0
source /home/warpai-test/.cargo/env
cargo build -p warp-agent-bus --bin warpai-companion --locked
cargo test -p warp-agent-bus --lib wsl_chunks_ --locked
cargo test -p warp-agent-bus --test managed_agent --locked
install -D -m 755 target/debug/warpai-companion /home/warpai-test/.config/.warpai/bin/warpai-companion
project='/home/warpai-test/project spaces 多语言'
mkdir -p "$project"
cd "$project"
printf 'original\0bytes\377' > binary.dat
printf 'fn main() { println!("WSL fixture"); }\n' > example.rs
printf '# WSL project\n\nOwned Markdown preview.\n' > preview.md
git init -q
git -c core.hooksPath= -c user.name='Owned WSL Fixture' -c user.email=fixture@example.invalid add binary.dat example.rs preview.md
git -c core.hooksPath= -c user.name='Owned WSL Fixture' -c user.email=fixture@example.invalid commit -qm 'Owned WSL fixture'
printf 'outside' > /home/warpai-test/outside
/home/warpai-test/.config/.warpai/bin/warpai-companion --version
GUEST
