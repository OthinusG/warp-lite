#!/usr/bin/env python3
"""Publish only source-owned step names from an isolated capture failure."""
import argparse
import json
import os
from pathlib import Path
import re


def failed_steps(source: str, log: str) -> list[str]:
    allowed = set(re.findall(r'TestStep::new\(\s*"([^"\n]+)"\)', source))
    observed = re.findall(r"Test step '([^'\n]{1,200})' failed", log)
    return sorted(set(observed).intersection(allowed))


def failed_assertions(source: str, log: str) -> list[str]:
    allowed = set(re.findall(r'add_named_assertion\(\s*"([^"\n]+)"', source))
    observed = re.findall(r"Native checkpoint failed: ([^\n]{1,200})", log)
    return sorted(set(observed).intersection(allowed))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log", type=Path, action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--exit-code", type=int, required=True)
    args = parser.parse_args()
    source = Path(__file__).resolve().parents[1] / "app/src/agent_communication/panel.rs"
    log = "\n".join(log.read_text(errors="replace") for log in args.log)
    for checkpoint in args.output.parent.rglob("checkpoint-assertion.txt"):
        log += "\nNative checkpoint failed: " + checkpoint.read_text(errors="replace")
    locations = sorted({location for location in re.findall(r"((?:app|crates)/[A-Za-z0-9_./-]+\.rs:\d+:\d+)", log.replace("\\", "/")) if (source.parents[3] / location.rsplit(":", 2)[0]).is_file()})
    diagnostic = {
        "source": os.environ.get("GITHUB_SHA", "local"),
        "exit_code": args.exit_code,
        "failed_steps": failed_steps(source.read_text(), log),
        "failed_assertions": failed_assertions(source.read_text(), log),
        "connection_errors": sorted(set(re.findall(r"Native SSH file failure: (InvalidProfile|SshUnavailable|SshAuthenticationUnavailable|ConnectionLost|IncompatibleVersion|StaleAttachment|CompanionUnavailable|FeatureUnavailable|InvalidInput|Conflict|CapacityExceeded|PermissionDenied|NotFound)\b", log))),
        "panic_locations": locations,
        "remote_editors": [dict(zip(["source", "cache", "loaded"], [value == "true" for value in values])) for values in re.findall(r"Remote editor diagnostic: source=(true|false), cache=(true|false), loaded=(true|false)", log)],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(diagnostic, indent=2) + "\n")
    print(json.dumps(diagnostic))


if __name__ == "__main__":
    main()
