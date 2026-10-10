#!/usr/bin/env python3
"""Build the dedicated Linux guest payload bundled only by Windows Warpai."""
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile


def installer_source():
    source = Path(__file__).with_name("install-unix.sh").read_text()
    source = source.replace('install_dir="$HOME/.config/.warpai/bin"',
                            'install_dir="$HOME/.config/.warpai/wsl/bin"')
    source = source.replace('"$HOME/.config/.warpai" "$install_dir"',
                            '"$HOME/.config/.warpai" "$HOME/.config/.warpai/wsl" "$install_dir"')
    source = source.replace("warpai-companion", "warpai-wsl-companion")
    source = source.replace("Reconnect your SSH terminal in Warpai to detect it.",
                            "Return to WSL communication settings in Warpai.")
    return source


def package(binary, output, source):
    # Archived guest sources have no .git; use the exact revision exported by the runner.
    if re.fullmatch(r"[0-9a-f]{40}", source) is None:
        raise SystemExit("An exact source commit SHA is required")
    if subprocess.check_output(["uname", "-s"], text=True).strip() != "Linux":
        raise SystemExit("The WSL payload must be built in the Linux guest of the Windows build runner")
    version = subprocess.check_output([str(binary.resolve()), "--version"], text=True).strip()
    if version != "warpai-companion 4.0.0 protocol 1":
        raise SystemExit("A dedicated WSL Companion 4.0.0 is required")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="warpai-wsl-package-") as temporary:
        payload = Path(temporary)
        shutil.copy2(binary, payload / "warpai-wsl-companion")
        subprocess.run([sys.executable, str(Path(__file__).with_name("build-runtime.py")),
                        str(payload / "git-runtime/git")], check=True)
        runtime = payload / "git-runtime"
        sums = "".join(hashlib.sha256(path.read_bytes()).hexdigest() + "  " +
                       path.relative_to(runtime).as_posix() + "\n"
                       for path in sorted(runtime.rglob("*")) if path.is_file())
        (runtime / "SHA256SUMS").write_text(sums)
        runtime_name = "companion-runtime-" + hashlib.sha256(sums.encode()).hexdigest()
        runtime.rename(payload / runtime_name)
        (payload / "manifest.json").write_text(json.dumps({
            "source": source, "version": version, "runtime_directory": runtime_name,
            "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        }) + "\n")
        (payload / "platform").write_text("Linux:x86_64\n")
        (payload / "install-unix.sh").write_text(installer_source())
        for notice in ["FORK_NOTICE.md", "LICENSE-AGPL"]:
            shutil.copy2(notice, payload / notice)
        (payload / "SHA256SUMS").write_text("".join(
            hashlib.sha256(path.read_bytes()).hexdigest() + "  " +
            path.relative_to(payload).as_posix() + "\n"
            for path in sorted(payload.rglob("*")) if path.is_file()))
        with tarfile.open(output, "w:gz") as archive:
            for entry in sorted(payload.iterdir()):
                archive.add(entry, arcname=entry.name)
    output.with_suffix(output.suffix + ".sha256").write_text(
        hashlib.sha256(output.read_bytes()).hexdigest() + "\n")


if __name__ == "__main__":
    package(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3])
