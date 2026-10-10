#!/usr/bin/env python3
"""Owned CI fixture: ordinary Linux Agent starts MCP from guest configuration."""
import hashlib
import json
import os
from pathlib import Path
import pty
import pwd
import shlex
import subprocess
import sys
import threading
import uuid


def main():
    mode, companion, directory = sys.argv[1:]
    assert pwd.getpwuid(os.getuid()).pw_name == "warpai-test"
    root = Path(directory).resolve()
    assert root == Path("/home/warpai-test/project spaces 多语言")
    os.chdir(root)
    assert all(name not in os.environ for name in (
        "WARP_AGENT_CAPABILITY", "WARP_AGENT_ENDPOINT", "WARP_TERMINAL_SESSION_UUID"))
    if mode == "--terminal":
        namespace = hashlib.sha256(os.environ["WSL_DISTRO_NAME"].encode()).hexdigest()
        state = Path(os.environ.get("XDG_DATA_HOME", str(Path.home() / ".local/share"))) / "warpai/wsl-remote" / namespace
        state.mkdir(parents=True, exist_ok=True)
        (state / "mcp-settings.json").write_text(json.dumps({"enabled": True, "selected": {
            "fixture": {"active": True, "program": "fixture", "executable": sys.executable,
                        "adapter": {"Codex": str(root / "native-mcp.json")}, "bridge": companion}
        }}))
        (root / "native-mcp.json").write_text(json.dumps({"command": companion, "args": ["mcp", "fixture"]}))
        (root / "native-session.json").unlink(missing_ok=True)
        child, terminal = pty.fork()
        if child == 0:
            command = shlex.join([sys.executable, str(Path(__file__).resolve()), "--agent", companion, str(root)])
            os.execl("/bin/bash", "bash", "--noprofile", "--norc", "-ic", command)
        def output():
            try:
                while data := os.read(terminal, 65536):
                    os.write(1, data)
            except OSError:
                pass
        threading.Thread(target=output, daemon=True).start()
        try:
            while data := os.read(0, 65536):
                os.write(terminal, data)
        finally:
            # Closing a master with a blocked reader does not deliver EOF to the Agent.
            os.write(terminal, b"\x04")
            os.waitpid(child, 0)
            os.close(terminal)
        return
    assert mode == "--agent" and os.isatty(0)
    config = json.loads((root / "native-mcp.json").read_text())
    bridge = subprocess.Popen([config["command"], *config["args"]], stdin=subprocess.PIPE,
                              stdout=subprocess.PIPE, stderr=None, text=True)
    sequence = 0
    def rpc(method, params):
        nonlocal sequence
        sequence += 1
        bridge.stdin.write(json.dumps({"jsonrpc": "2.0", "id": sequence, "method": method, "params": params}) + "\n")
        bridge.stdin.flush()
        while True:
            line = bridge.stdout.readline(1024 * 1024)
            assert line, "Native guest MCP disconnected"
            value = json.loads(line)
            if value.get("id") == sequence:
                assert "error" not in value, value.get("error", {}).get("data", {}).get("code", "MCP request failed")
                return value["result"]
    def tool(name, arguments):
        result = rpc("tools/call", {"name": name, "arguments": arguments})
        assert not result.get("isError", False)
        return result.get("structuredContent") or json.loads(result["content"][0]["text"])
    rpc("initialize", {"protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "owned-native-wsl", "version": "1"}})
    bridge.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
    bridge.stdin.flush()
    assert len(rpc("tools/list", {})["tools"]) == 31
    assert tool("warp_agent_ready", {})["ready"]
    def publish(wakes):
        temporary = root / "native-session.tmp"
        temporary.write_text(json.dumps({"shell_pid": os.getppid(), "agent_pid": os.getpid(), "wakes": wakes}))
        temporary.replace(root / "native-session.json")
    publish(0)
    wakes = 0
    try:
        for line in sys.stdin:
            if line.strip() == "OWNED_READY":
                assert tool("warp_agent_ready", {})["ready"]
                continue
            assert line.startswith("Warpai peer work is waiting (message ")
            uuid.UUID(line.split("(message ", 1)[1].split(")", 1)[0])
            inbox = tool("warp_agent_inbox", {})
            for message in inbox["messages"]:
                tool("warp_agent_ack", {"message_id": message["id"]})
            wakes += 1
            assert tool("warp_agent_ready", {})["ready"]
            publish(wakes)
    finally:
        bridge.stdin.close()
        bridge.wait(timeout=10)


if __name__ == "__main__":
    main()
