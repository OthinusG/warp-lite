#!/usr/bin/env python3
"""Repeat deterministic protocol/restart/history checks; never invoke vendor models."""
import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import time

CHECKS = (
    ("protocol_restart_replay", ["cargo", "test", "-p", "warp-agent-bus", "--locked"]),
    ("representative_history", ["cargo", "test", "-p", "warp-agent-bus", "--lib",
        "representative_history_pages_within_budget_and_preserves_live_work", "--locked", "--", "--ignored"]),
)


def run(args):
    if sys.platform not in ("darwin", "win32"):
        raise ValueError("Soak targets are macOS and Windows")
    source = os.environ.get("GITHUB_SHA") or subprocess.check_output(
        ["git", "rev-parse", "HEAD"], text=True).strip()
    if len(source) != 40 or any(char not in "0123456789abcdef" for char in source):
        raise ValueError("Invalid source commit")
    phases = []
    if args.phase == 2:
        if not args.previous:
            raise ValueError("Phase two requires the first phase report")
        previous = json.loads(args.previous.read_text())
        phases = previous.get("phases", [])
        if (previous.get("schema") != 1 or previous.get("source") != source
                or previous.get("platform") != sys.platform or previous.get("status") != "passed"
                or len(phases) != 1 or phases[0].get("phase") != 1
                or phases[0].get("target_seconds") != args.seconds
                or not isinstance(phases[0].get("elapsed_seconds"), (int, float))
                or not math.isfinite(phases[0]["elapsed_seconds"])
                or not args.seconds <= phases[0]["elapsed_seconds"] <= 6 * 3600):
            raise ValueError("Prior soak report does not match this source, OS and duration")
    elif args.previous:
        raise ValueError("Phase one cannot import a previous report")
    if args.dry_run:
        print(json.dumps({"source": source, "platform": sys.platform,
            "phase": args.phase, "seconds": args.seconds, "checks": [name for name, _ in CHECKS]}))
        return 0
    # Compile before the timer: build time cannot count as runtime soak evidence.
    warm = subprocess.run(["cargo", "test", "-p", "warp-agent-bus", "--locked", "--no-run"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=1800)
    if warm.returncode:
        raise ValueError("Soak executable compilation failed")
    started = time.monotonic()
    phase = {"phase": args.phase, "target_seconds": args.seconds, "iterations": 0, "elapsed_seconds": 0}
    report = {"schema": 1, "source": source, "platform": sys.platform,
        "status": "running", "eight_hour_pass": False, "phases": phases + [phase]}
    args.report.parent.mkdir(parents=True, exist_ok=True)

    def save():
        phase["elapsed_seconds"] = time.monotonic() - started
        temporary = args.report.with_suffix(".tmp")
        temporary.write_text(json.dumps(report, indent=2) + "\n")
        temporary.replace(args.report)

    save()
    try:
        while time.monotonic() - started < args.seconds:
            for name, command in CHECKS:
                outcome = subprocess.run(command, stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL, timeout=300)
                if outcome.returncode:
                    raise RuntimeError(name)
            phase["iterations"] += 1
            save()
            print(f"Soak phase {args.phase}: {phase['iterations']} cycles, {phase['elapsed_seconds']:.0f}s", flush=True)
            time.sleep(min(30, max(0, args.seconds - (time.monotonic() - started))))
        report["status"] = "passed"
        save()
        report["eight_hour_pass"] = (args.phase == 2 and args.seconds == 4 * 3600
            and sum(item["elapsed_seconds"] for item in report["phases"]) >= 8 * 3600)
        save()
        return 0
    except (RuntimeError, subprocess.TimeoutExpired):
        report["status"] = "failed"
        report["failed_check"] = name
        save()
        print("Deterministic soak failed; rerun the protocol/history checks for diagnostics", file=sys.stderr)
        return 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--phase", type=int, choices=(1, 2), required=True)
    parser.add_argument("--seconds", type=int, default=4 * 3600)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--previous", type=Path)
    parser.add_argument("--dry-run", action="store_true")
    arguments = parser.parse_args()
    if not 1 <= arguments.seconds <= 4 * 3600:
        parser.error("Duration must be between 1 and 14400 seconds")
    sys.exit(run(arguments))
