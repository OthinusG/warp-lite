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
    allowed = set(re.findall(r'add_named_assertion(?:_with_data_from_prior_step)?\(\s*"([^"\n]+)"', source))
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
    for name in ["checkpoint-panic.txt", "checkpoint-tree.txt", "checkpoint-worktree.txt", "checkpoint-worktree-error.txt", "checkpoint-private-git.txt", "checkpoint-private-git-runner.txt"]:
        for checkpoint in args.output.parent.rglob(name):
            log += "\n" + checkpoint.read_text(errors="replace")
    locations = sorted({location for location in re.findall(r"((?:app|crates)/[A-Za-z0-9_./-]+\.rs:\d+:\d+)", log.replace("\\", "/")) if (source.parents[3] / location.rsplit(":", 2)[0]).is_file()})
    diagnostic = {
        "source": os.environ.get("GITHUB_SHA", "local"),
        "exit_code": args.exit_code,
        "failed_steps": failed_steps(source.read_text(), log),
        "failed_assertions": failed_assertions(source.read_text(), log),
        "connection_errors": sorted(set(re.findall(r"Native SSH file failure: (InvalidProfile|SshUnavailable|SshAuthenticationUnavailable|ConnectionLost|IncompatibleVersion|StaleAttachment|CompanionUnavailable|FeatureUnavailable|InvalidInput|Conflict|CapacityExceeded|PermissionDenied|NotFound)\b", log))),
        "remote_trees": [dict(zip(["active", "current", "selected", "attached", "error", "roots", "entries"], [value == "true" for value in values[:5]] + [int(value) for value in values[5:]])) for values in re.findall(r"Native SSH tree: active=(true|false), current=(true|false), selected=(true|false), attached=(true|false), error=(true|false), roots=(\d{1,6}), entries=(\d{1,6})\b", log)],
        "worktree_states": [dict(zip(["connected", "form", "joined", "team", "main", "submitting", "query_error", "git_rev_parse", "git_registry"], [value == "true" for value in values[:6]] + [int(value) for value in values[6:]])) for values in re.findall(r"Native Worktree: connected=(true|false), form=(true|false), joined=(true|false), team=(true|false), main=(true|false), submitting=(true|false), query_error=(\d{1,2}), git_rev_parse=(-?\d{1,3}), git_registry=(-?\d{1,3})\b", log)],
        "worktree_read_errors": sorted(set(int(value) for value in re.findall(r"Native Worktree read error: category=(\d{1,2})\b", log))),
        "private_git_errors": sorted(set(int(value) for value in re.findall(r"Native private Git error: code=(\d{1,3})\b", log))),
        "private_git_runner": [dict(stage=int(stage), exit=int(exit_code)) for stage, exit_code in re.findall(r"Native private Git runner: stage=([123]), exit=(-?\d{1,3})\b", log)],
        "panic_locations": locations,
        "remote_editors": [dict(zip(["source", "cache", "loaded"], [value == "true" for value in values])) for values in re.findall(r"Remote editor diagnostic: source=(true|false), cache=(true|false), loaded=(true|false)", log)],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(diagnostic, indent=2) + "\n")
    print(json.dumps(diagnostic))


if __name__ == "__main__":
    main()
