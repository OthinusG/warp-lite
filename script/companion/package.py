#!/usr/bin/env python3
"""Package a verified source-matched payload as a native remote installer."""
import json
import hashlib
import os
import re
from pathlib import Path
import shutil
import subprocess
import tarfile
import time

payload = Path("companion-release")
subprocess.run(["python3", "script/companion/build-runtime.py", str(payload / "git-runtime/git")], check=True)
runtime = payload / "git-runtime"
runtime_files = sorted(path for path in runtime.rglob("*") if path.is_file())
runtime_sums = "".join(hashlib.sha256(path.read_bytes()).hexdigest() + "  " + path.relative_to(runtime).as_posix() + "\n" for path in runtime_files)
(runtime / "SHA256SUMS").write_text(runtime_sums)
runtime_name = "companion-runtime-" + hashlib.sha256(runtime_sums.encode()).hexdigest()
runtime.rename(payload / runtime_name)
branding = Path("app/assets/branding")
shutil.copy2(branding / "warpai-companion.png", payload / "companion.png")
manifest = json.loads((payload / "manifest.json").read_text())
manifest["runtime_directory"] = runtime_name
manifest["git_source"] = (payload / runtime_name / "git/SOURCE.txt").read_text().strip()
(payload / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
checksums = sorted(path for path in payload.rglob("*") if path.is_file() and path.name != "SHA256SUMS")
(payload / "SHA256SUMS").write_text("".join(hashlib.sha256(path.read_bytes()).hexdigest() + "  " + path.relative_to(payload).as_posix() + "\n" for path in checksums))
match = re.fullmatch(r"warpai-companion ([0-9]+\.[0-9]+\.[0-9]+) protocol [0-9]+", manifest["version"])
if match is None:
    raise SystemExit("A semantic Companion component version is required")
version = match.group(1)
if os.name == "nt":
    subprocess.run(["ISCC", f"/DProductVersion={version}", f"/DRuntimeDirectory={runtime_name}", "script/companion/windows.iss"], check=True)
    installer = f"WarpaiCompanion-{version}-windows-x64-setup.exe"
    shutil.move(Path("script/companion") / installer, installer)
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
        installer = Path(f"WarpaiCompanion-{version}-linux-x64.run")
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
        shutil.copy2(branding / "warpai-companion.icns", image / ".VolumeIcon.icns")
        # Finder custom icons use a resource fork; hdiutil retains it in the image.
        subprocess.run(["osascript", "-l", "JavaScript", "-e", '''ObjC.import("AppKit");
function run(args) {
    var icon = $.NSImage.alloc.initWithContentsOfFile(args[0]);
    if (!$.NSWorkspace.sharedWorkspace.setIconForFileOptions(icon, args[1], 0)) {
        throw new Error("Unable to assign installer icon");
    }
}''', str((branding / "warpai-companion.icns").resolve()), str(launcher.resolve())], check=True)
        installer = f"WarpaiCompanion-{version}-macos-arm64.dmg"
        subprocess.run(["sh", "script/macos/create-dmg.sh", str(image), installer, "Warpai Companion"], check=True)
        for attempt in range(4):
            result = subprocess.run(["hdiutil", "verify", installer], capture_output=True, text=True)
            if result.returncode == 0:
                print(result.stdout)
                break
            if attempt == 3 or "Resource temporarily unavailable" not in result.stderr:
                raise SystemExit(result.stderr)
            # macOS can briefly hold the newly created image open after create exits.
            time.sleep(2)
    else:
        raise SystemExit("Unsupported remote installer platform")
