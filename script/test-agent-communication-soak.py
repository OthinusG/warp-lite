#!/usr/bin/env python3
"""Verify soak orchestration without compiling Rust or calling any models."""
import argparse
import importlib.util
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("soak", Path(__file__).with_name("soak-agent-communication.py"))
soak = importlib.util.module_from_spec(spec)
spec.loader.exec_module(soak)

with tempfile.TemporaryDirectory() as directory:
    first = Path(directory) / "phase-one.json"
    second = Path(directory) / "phase-two.json"
    ticks = iter(value / 4 for value in range(100))
    args = argparse.Namespace(phase=1, seconds=1, report=first, previous=None, dry_run=False)
    with patch.dict(soak.os.environ, {"GITHUB_SHA": "a" * 40}), patch.object(soak.sys, "platform", "darwin"), \
            patch.object(soak.time, "monotonic", side_effect=lambda: next(ticks)), \
            patch.object(soak.time, "sleep"), patch.object(soak.subprocess, "run", return_value=SimpleNamespace(returncode=0)):
        assert soak.run(args) == 0
        assert json.loads(first.read_text())["status"] == "passed"
        args.phase, args.report, args.previous = 2, second, first
        assert soak.run(args) == 0
        report = json.loads(second.read_text())
        assert len(report["phases"]) == 2 and not report["eight_hour_pass"]
        with patch.dict(soak.os.environ, {"GITHUB_SHA": "b" * 40}):
            try:
                soak.run(args)
                raise AssertionError("A previous source was accepted")
            except ValueError:
                pass
        args.phase, args.report, args.previous = 1, first, None
        with patch.object(soak.subprocess, "run", side_effect=[SimpleNamespace(returncode=0), SimpleNamespace(returncode=1)]):
            assert soak.run(args) == 1
            failure = json.loads(first.read_text())
            assert failure["status"] == "failed" and failure["failed_check"] == "protocol_restart_replay"
        with patch.object(soak.subprocess, "run", side_effect=[SimpleNamespace(returncode=0),
                soak.subprocess.TimeoutExpired("fixed protocol check", 300)]):
            assert soak.run(args) == 1
            assert json.loads(first.read_text())["status"] == "failed"
print("Soak source continuity, short-run labeling and failure reporting: pass")
