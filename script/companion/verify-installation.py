#!/usr/bin/env python3
"""Install only on disposable GitHub runners; assert stable native account delivery."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

if os.environ.get("GITHUB_ACTIONS") != "true":
    raise SystemExit("Native installation verification requires a disposable GitHub runner")

payload_manifest = json.loads(Path("companion-release/manifest.json").read_text())
component_version = payload_manifest["version"].split()[1]

if os.name == "nt":
    binary_name = "warpai-companion.exe"
    command = [str(Path(f"WarpaiCompanion-{component_version}-windows-x64-setup.exe").resolve()), "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"]
else:
    binary_name = "warpai-companion"
    if sys.platform == "darwin":
        command = ["sh", "companion-image/payload/install-unix.sh"]
    else:
        command = ["sh", f"WarpaiCompanion-{component_version}-linux-x64.run"]

installed = Path.home() / ".config/.warpai/bin" / binary_name
if sys.platform == "darwin":
    assert Path("companion-image/.VolumeIcon.icns").read_bytes() == Path("app/assets/branding/warpai-companion.icns").read_bytes()
    launcher = Path("companion-image/Install Warpai Companion.command")
    assert subprocess.check_output(["xattr", "-p", "com.apple.ResourceFork", str(launcher)]), "Finder installer icon is missing"
before = {path: path.read_bytes() if path.is_file() else None for path in [Path.home() / ".bashrc", Path.home() / ".zshrc", Path.home() / ".profile", Path.home() / ".ssh/config"]}
for _ in range(2):
    subprocess.run(command, check=True)
    assert installed.read_bytes() == (Path("companion-release") / binary_name).read_bytes()
    metadata = json.loads(installed.with_name("companion-manifest.json").read_text())
    assert metadata["source"] == os.environ["GITHUB_SHA"]
    assert metadata["sha256"] == hashlib.sha256(installed.read_bytes()).hexdigest()
    version = subprocess.check_output([str(installed), "--version"], text=True).strip()
    assert version == payload_manifest["version"], version
    if os.name != "nt":
        assert installed.stat().st_mode & 0o777 == 0o700
    else:
        assert installed.with_name("companion.ico").read_bytes() == Path("app/assets/branding/warpai-companion.ico").read_bytes()
for path, content in before.items():
    assert (path.read_bytes() if path.is_file() else None) == content, "Installation changed unrelated shell or SSH configuration"

if os.name != "nt":
    payload = Path("companion-image/payload") if sys.platform == "darwin" else Path("companion-release")
    payload_binary = payload / binary_name
    payload_binary.write_bytes(b"invalid component")
    result = subprocess.run(["sh", str(payload / "install-unix.sh")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    assert result.returncode != 0, "Corrupted payload must not install"
    assert hashlib.sha256(installed.read_bytes()).hexdigest() == metadata["sha256"]
print("Native installer, default location, repeat installation, version, checksum and configuration ownership: passed")
