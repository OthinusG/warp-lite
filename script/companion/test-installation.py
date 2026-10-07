#!/usr/bin/env python3
"""Check staged Unix installation and corruption rejection without native builds."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    installer = Path(__file__).with_name("install-unix.sh").resolve()
    with tempfile.TemporaryDirectory(prefix="warpai-install-check-") as temporary:
        root = Path(temporary)
        payload = root / "payload"
        payload.mkdir()
        account = root / "account with spaces"
        account.mkdir()
        marker = account / ".profile"
        marker.write_text("# preserved\n")
        binary = payload / "warpai-companion"
        binary.write_text('#!/bin/sh\ncase "$1" in --version) echo "warpai-companion 3.0.0 protocol 1";; --check-runtime) exit 0;; *) exit 1;; esac\n')
        binary.chmod(0o700)
        runtime_name = "companion-runtime-" + "a" * 64
        runtime = payload / runtime_name
        git = runtime / "git/bin/git"
        git.parent.mkdir(parents=True)
        git.write_text('#!/bin/sh\n[ "$1" = --version ] || exit 1\necho "git version fixture"\n')
        git.chmod(0o700)
        (runtime / "SHA256SUMS").write_text(hashlib.sha256(git.read_bytes()).hexdigest() + "  git/bin/git\n")
        (payload / "manifest.json").write_text(json.dumps({"runtime_directory": runtime_name}))
        (payload / "platform").write_text(subprocess.check_output(["uname", "-s"], text=True).strip() + ":" + subprocess.check_output(["uname", "-m"], text=True).strip() + "\n")
        (payload / "install-unix.sh").write_bytes(installer.read_bytes())
        (payload / "SHA256SUMS").write_text("".join(
            hashlib.sha256(file.read_bytes()).hexdigest() + "  " + file.relative_to(payload).as_posix() + "\n"
            for file in sorted(payload.rglob("*")) if file.is_file() and file != payload / "SHA256SUMS"
        ))
        environment = {**os.environ, "HOME": str(account)}
        def install():
            return subprocess.run(["sh", str(payload / "install-unix.sh")], env=environment,
                                  stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        assert install().returncode == 0
        installed = account / ".config/.warpai/bin"
        assert (installed / "warpai-companion").read_bytes() == binary.read_bytes()
        assert (installed / runtime_name / "git/bin/git").read_bytes() == git.read_bytes()
        assert install().returncode == 0
        assert marker.read_text() == "# preserved\n"
        original = (installed / "warpai-companion").read_bytes()
        git.write_text("corrupt runtime")
        assert install().returncode != 0
        assert (installed / "warpai-companion").read_bytes() == original
        assert marker.read_text() == "# preserved\n"
    print("Unix staged runtime installation, repeat install and corruption rejection: passed")


if __name__ == "__main__":
    main()
