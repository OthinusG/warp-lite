#!/usr/bin/env python3
"""Package a verified source-matched payload as a native remote installer."""
import os
import re
from pathlib import Path
import shutil
import subprocess
import tarfile
import time

payload = Path("companion-release")
version = os.environ["RELEASE_TAG"].removeprefix("v")
if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
    raise SystemExit("A semantic release version is required")
if os.name == "nt":
    subprocess.run(["ISCC", f"/DProductVersion={version}", "script/companion/windows.iss"], check=True)
    shutil.move("script/companion/WarpaiCompanion-windows-x64-setup.exe", "WarpaiCompanion-windows-x64-setup.exe")
else:
    system = subprocess.check_output(["uname", "-s"], text=True).strip()
    architecture = subprocess.check_output(["uname", "-m"], text=True).strip()
    if (system, architecture) not in {("Linux", "x86_64"), ("Darwin", "arm64")} :
        raise SystemExit("Unsupported installer architecture")
    (payload / "platform").write_text(f"{system}:{architecture}\n")
    shutil.copy2("script/companion/install-unix.sh", payload)
    if system == "Linux":
        archive = Path("companion-payload.tar.gz")
        with tarfile.open(archive, "w:gz") as output:
            for entry in sorted(payload.iterdir()):
                output.add(entry, arcname=entry.name)
        installer = Path("WarpaiCompanion-linux-x64.run")
        with installer.open("wb") as output:
            output.write(Path("script/companion/linux-installer-header.sh").read_bytes())
            output.write(archive.read_bytes())
        installer.chmod(0o755)
        archive.unlink()
    elif system == "Darwin":
        image = Path("companion-image")
        image.mkdir()
        shutil.copytree(payload, image / "payload")
        launcher = image / "Install Warpai Companion.command"
        launcher.write_text('#!/bin/sh\nset -eu\ninstaller_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\nsh "$installer_dir/payload/install-unix.sh"\nprintf "Press Return to close this installer."\nread -r answer\n')
        launcher.chmod(0o755)
        subprocess.run(["hdiutil", "create", "-volname", "Warpai Companion", "-srcfolder", str(image), "-ov", "-format", "UDZO", "WarpaiCompanion-macos-arm64.dmg"], check=True)
        for attempt in range(4):
            result = subprocess.run(["hdiutil", "verify", "WarpaiCompanion-macos-arm64.dmg"], capture_output=True, text=True)
            if result.returncode == 0:
                print(result.stdout)
                break
            if attempt == 3 or "Resource temporarily unavailable" not in result.stderr:
                raise SystemExit(result.stderr)
            # macOS can briefly hold the newly created image open after create exits.
            time.sleep(2)
    else:
        raise SystemExit("Unsupported remote installer platform")
