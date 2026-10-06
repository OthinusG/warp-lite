#!/usr/bin/env python3
"""Run file acceptance against an owned loopback SSH server, optionally capturing UI."""
import argparse
import getpass
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import uuid


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--capture", type=Path)
    args = parser.parse_args()
    windows = os.name == "nt"
    server = shutil.which("sshd") or ("C:/Windows/System32/OpenSSH/sshd.exe" if windows else "/usr/sbin/sshd")
    if not Path(server).is_file():
        raise SystemExit("Owned SSH acceptance requires the native OpenSSH server")
    # Keep POSIX fixtures under the owner home: sshd StrictModes rejects shared temp ancestors.
    with tempfile.TemporaryDirectory(prefix="warpai-ssh-check-", dir=Path.home(), ignore_cleanup_errors=windows) as temporary:
        fixture = Path(temporary)
        root = fixture / "SSH project with spaces 多语言"
        root.mkdir()
        for name in ["host", "client"]:
            subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(fixture / name)], check=True)
        if windows:
            for name in ["host", "client", "client.pub"]:
                subprocess.run(["icacls", str(fixture / name), "/inheritance:r", "/grant:r", f"{getpass.getuser()}:F", "*S-1-5-18:F", "*S-1-5-32-544:F"], check=True, stdout=subprocess.DEVNULL)
        public = (fixture / "host.pub").read_text().split()
        (fixture / "known_hosts").write_text(f"[127.0.0.1]:22222 {public[0]} {public[1]}\n")
        native = fixture.as_posix()
        (fixture / "server.conf").write_text(
            f'Port 22222\nListenAddress 127.0.0.1\nHostKey "{native}/host"\n'
            f'AuthorizedKeysFile "{native}/client.pub"\n'
            'PasswordAuthentication no\nKbdInteractiveAuthentication no\nStrictModes yes\n'
            + ("" if windows else "UsePAM yes\n")
            + 'Subsystem sftp internal-sftp\n'
        )
        config = fixture / "client.conf"
        config.write_text(f'Host warpai-test\n HostName 127.0.0.1\n Port 22222\n User {getpass.getuser()}\n IdentityFile "{native}/client"\n IdentitiesOnly yes\n UserKnownHostsFile "{native}/known_hosts"\n')
        (root / "preview.md").write_text("# Remote project\n\n![Remote image](image.svg)\n\n[Open code](example.rs)\n", encoding="utf-8")
        (root / "image.svg").write_text('<svg xmlns="http://www.w3.org/2000/svg" width="240" height="80"><rect width="240" height="80" fill="#4477aa"/></svg>')
        (root / "example.rs").write_text('fn main() { println!("Remote original"); }\n')
        for command in [["init", "--initial-branch=main"], ["add", "--", "."], ["-c", "user.name=Warpai Fixture", "-c", "user.email=fixture@example.invalid", "-c", "core.hooksPath=", "commit", "-m", "Fixture base"]]:
            subprocess.run(["git", *command], cwd=root, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        (root / "example.rs").write_text('fn main() { println!("Remote changed"); }\n')
        command = [server, "-D", "-e", "-f", str(fixture / "server.conf")]
        if not windows and os.geteuid() != 0 and os.environ.get("GITHUB_ACTIONS") == "true":
            command = ["sudo", "-n", *command]
        # Never publish daemon logs: only controlled return codes are diagnostic output.
        with (fixture / "server.log").open("w") as log:
            service = "warpai-file-check-" + uuid.uuid4().hex if windows else None
            if windows:
                subprocess.run(["sc.exe", "create", service, "binPath=", f'"{server}" -f "{fixture / "server.conf"}"', "start=", "demand"], check=True, stdout=subprocess.DEVNULL)
                subprocess.run(["sc.exe", "start", service], check=True, stdout=subprocess.DEVNULL)
                daemon = None
            else:
                daemon = subprocess.Popen(command, stdout=subprocess.DEVNULL, stderr=log)
            try:
                for _ in range(30):
                    probe = subprocess.run(["ssh", "-F", str(config), "-oBatchMode=yes", "-oStrictHostKeyChecking=yes", "warpai-test", "echo", "ready"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10)
                    if probe.returncode == 0:
                        break
                    time.sleep(0.2)
                else:
                    raise RuntimeError("Owned loopback SSH authentication failed")
                env = os.environ.copy()
                companion = Path("target/debug/warpai-companion" + (".exe" if windows else "")).resolve()
                env.update(WARP_TEST_SSH_CONFIG=str(config), WARP_TEST_REMOTE_ROOT=str(root), WARP_TEST_COMPANION_PATH=str(companion))
                subprocess.run(["cargo", "test", "-p", "warp-agent-bus", "--test", "ssh_companion", "native_ssh_file_tools_", "--locked", "--", "--ignored"], env=env, check=True, timeout=120)
                if args.capture:
                    executable = args.capture.resolve()
                    subprocess.run([str(executable)], cwd=executable.parent, env=env, check=True, timeout=330)
                print("Owned native SSH/SFTP file acceptance passed")
            finally:
                if windows:
                    subprocess.run(["sc.exe", "stop", service], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    subprocess.run(["sc.exe", "delete", service], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                else:
                    daemon.terminate()
                    daemon.wait(timeout=10)


if __name__ == "__main__":
    main()
