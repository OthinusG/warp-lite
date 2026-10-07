#!/usr/bin/env python3
"""Build a private, pinned Git runtime on native packaging runners only."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
import zipfile

SOURCE = "a018953688f1b10bddf91bff8747068f5f4746a4"  # Git v2.56.0
SOURCE_URL = f"https://codeload.github.com/git/git/tar.gz/{SOURCE}"
SOURCE_SHA256 = "898ecd505bcec4390b3b7b2c1882509af3f11f3b62c56147d19f01e2a4e81355"
WINDOWS_URL = "https://github.com/git-for-windows/git/releases/download/v2.56.0.windows.2/MinGit-2.56.0.2-64-bit.zip"
WINDOWS_SHA256 = "da35e72aa21c005a5a0d298cfbae110bc1609a815730ea0dde84b01a1b3cd3be"


def download(url, digest, destination):
    with urllib.request.urlopen(url, timeout=60) as response:
        data = response.read(128 * 1024 * 1024 + 1)
    if len(data) > 128 * 1024 * 1024 or hashlib.sha256(data).hexdigest() != digest:
        raise SystemExit("Git runtime source checksum mismatch")
    destination.write_bytes(data)


def build(destination):
    if destination.exists():
        raise SystemExit("Refusing to overwrite an existing Git runtime")
    destination.mkdir(parents=True)
    with tempfile.TemporaryDirectory(prefix="warpai-git-build-") as temporary:
        temporary = Path(temporary)
        if os.name == "nt":
            archive = temporary / "git.zip"
            download(WINDOWS_URL, WINDOWS_SHA256, archive)
            with zipfile.ZipFile(archive) as zipped:
                for entry in zipped.infolist():
                    path = Path(entry.filename)
                    if path.is_absolute() or ".." in path.parts or "\\" in entry.filename:
                        raise SystemExit("Invalid Git runtime archive path")
                zipped.extractall(destination)
            executable = destination / "cmd/git.exe"
            provenance = WINDOWS_URL
        else:
            archive = temporary / "git.tar.gz"
            download(SOURCE_URL, SOURCE_SHA256, archive)
            with tarfile.open(archive) as tar:
                # data rejects traversal and special files without relying on archive ownership.
                members = [entry for entry in tar.getmembers() if not entry.issym() and not entry.islnk()]
                tar.extractall(temporary, members=members, filter="data")
            source = temporary / f"git-{SOURCE}"
            options = ["NO_CURL=YesPlease", "NO_EXPAT=YesPlease", "NO_OPENSSL=YesPlease",
                       "NO_GETTEXT=YesPlease", "NO_PERL=YesPlease", "NO_PYTHON=YesPlease",
                       "NO_TCLTK=YesPlease"]
            if sys.platform == "linux":
                options += ["LDFLAGS=-static", "NO_ICONV=YesPlease"]
            elif sys.platform == "darwin":
                options += ["CC=clang", "CFLAGS=-O2 -mmacosx-version-min=11.0"]
            else:
                raise SystemExit("Unsupported Companion runtime platform")
            subprocess.run(["make", "-j2", *options, "git"], cwd=source, check=True)
            (destination / "bin").mkdir()
            executable = destination / "bin/git"
            shutil.copy2(source / "git", executable)
            shutil.copy2(source / "COPYING", destination / "COPYING")
            # Ship the exact source archive for GPL source availability.
            shutil.copy2(archive, destination / "git-source.tar.gz")
            provenance = SOURCE_URL
            if sys.platform == "linux":
                dynamic = subprocess.run(["ldd", str(executable)], capture_output=True, text=True)
                if "not a dynamic executable" not in dynamic.stdout + dynamic.stderr:
                    raise SystemExit("Linux Git runtime must be statically linked")
            else:
                linked = subprocess.check_output(["otool", "-L", str(executable)], text=True)
                for line in linked.splitlines()[1:]:
                    if not line.strip().startswith(("/usr/lib/", "/System/Library/")):
                        raise SystemExit("macOS Git runtime must use only OS libraries")
        subprocess.run([str(executable.resolve()), "--version"], check=True)
        (destination / "SOURCE.txt").write_text(provenance + "\n", encoding="utf-8")


if __name__ == "__main__":
    build(Path(sys.argv[1]))
