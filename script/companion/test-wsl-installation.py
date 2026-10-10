#!/usr/bin/env python3
"""Verify the generated WSL installer in disposable accounts without native builds."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("package_wsl", Path(__file__).with_name("package-wsl.py"))
package_wsl = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package_wsl)


def main():
    with tempfile.TemporaryDirectory(prefix="warpai-wsl-install-") as temporary:
        root = Path(temporary)
        binary_fixture = root / "guest-binary"
        binary_fixture.write_bytes(b"Owned guest fixture")
        output = root / "guest-payload.tar.gz"
        source = "a" * 40
        def build_runtime(arguments, **kwargs):
            git = Path(arguments[-1]) / "bin/git"
            git.parent.mkdir(parents=True)
            git.write_bytes(b"Owned runtime fixture")
        with patch.object(package_wsl.subprocess, "check_output", side_effect=[
            "Linux\n", "warpai-companion 4.0.0 protocol 1\n",
        ]) as commands, patch.object(package_wsl.subprocess, "run", side_effect=build_runtime):
            package_wsl.package(binary_fixture, output, source)
            assert commands.call_count == 2
        with tarfile.open(output) as archive:
            manifest = json.load(archive.extractfile("manifest.json"))
            assert manifest["source"] == source
            assert manifest["sha256"] == hashlib.sha256(binary_fixture.read_bytes()).hexdigest()
        assert output.with_suffix(".gz.sha256").read_text().strip() == hashlib.sha256(output.read_bytes()).hexdigest()
        for invalid in ["", "HEAD", "a" * 39, "a" * 41, "g" * 40, source + "\n"]:
            try:
                package_wsl.package(binary_fixture, output, invalid)
            except SystemExit:
                pass
            else:
                raise AssertionError("Invalid source revision accepted")
        payload = root / "payload"
        payload.mkdir()
        account = root / "account spaces 多语言"
        account.mkdir()
        ssh = account / ".config/.warpai/bin/warpai-companion"
        ssh.parent.mkdir(parents=True)
        ssh.write_text("Preserve original SSH Companion 4.0.0\n")
        profile = account / ".profile"
        profile.write_text("# Preserve shell settings\n")
        binary = payload / "warpai-wsl-companion"
        binary.write_text('#!/bin/sh\ncase "$1" in --version) echo "warpai-companion 4.0.0 protocol 1";; --check-runtime) exit 0;; *) exit 1;; esac\n')
        binary.chmod(0o700)
        runtime_name = "companion-runtime-" + "a" * 64
        runtime = payload / runtime_name
        git = runtime / "git/bin/git"
        git.parent.mkdir(parents=True)
        git.write_text('#!/bin/sh\n[ "$1" = --version ] || exit 1\necho "git version fixture"\n')
        git.chmod(0o700)
        (runtime / "SHA256SUMS").write_text(hashlib.sha256(git.read_bytes()).hexdigest() + "  git/bin/git\n")
        (payload / "manifest.json").write_text(json.dumps({"runtime_directory": runtime_name}))
        platform = subprocess.check_output(["uname", "-s"], text=True).strip()
        architecture = subprocess.check_output(["uname", "-m"], text=True).strip()
        (payload / "platform").write_text(f"{platform}:{architecture}\n")
        (payload / "install-unix.sh").write_text(package_wsl.installer_source())
        (payload / "SHA256SUMS").write_text("".join(
            hashlib.sha256(path.read_bytes()).hexdigest() + "  " + path.relative_to(payload).as_posix() + "\n"
            for path in sorted(payload.rglob("*")) if path.is_file()))
        environment = {**os.environ, "HOME": str(account)}
        def install():
            return subprocess.run(["sh", str(payload / "install-unix.sh")], env=environment,
                                  stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode
        installed = account / ".config/.warpai/wsl/bin/warpai-wsl-companion"
        assert install() == 0
        assert installed.read_bytes() == binary.read_bytes()
        assert install() == 0
        assert ssh.read_text() == "Preserve original SSH Companion 4.0.0\n"
        assert profile.read_text() == "# Preserve shell settings\n"
        original = installed.read_bytes()
        original_git = git.read_bytes()
        git.write_text("Corrupt payload\n")
        assert install() != 0
        assert installed.read_bytes() == original
        git.write_bytes(original_git)
        # An account cannot redirect installation into another account via a symlink.
        other = root / "other-account"
        other.mkdir()
        isolated = root / "isolated-account"
        (isolated / ".config/.warpai").mkdir(parents=True)
        (isolated / ".config/.warpai/wsl").symlink_to(other, target_is_directory=True)
        environment["HOME"] = str(isolated)
        assert install() != 0
        assert list(other.iterdir()) == []
    print("WSL package provenance, staged installation, repeat install, SSH isolation and corruption rejection: passed")


if __name__ == "__main__":
    main()
