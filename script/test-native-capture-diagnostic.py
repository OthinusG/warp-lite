#!/usr/bin/env python3
"""Assert that diagnostic output never copies runtime payload or unknown names."""
import importlib.util
from pathlib import Path

module_spec = importlib.util.spec_from_file_location(
    "capture_diagnostic", Path(__file__).with_name("native-capture-diagnostic.py")
)
module = importlib.util.module_from_spec(module_spec)
module_spec.loader.exec_module(module)
source = 'TestStep::new("owned checkpoint")'
log = "\n".join([
    "Test step 'owned checkpoint' failed: unrelated runtime payload",
    "Test step 'runtime payload' failed",
    "Test step 'owned checkpoint' failed",
])
assert module.failed_steps(source, log) == ["owned checkpoint"]
assert module.failed_steps(source, "unrelated runtime payload") == []
assert module.failed_assertions('add_named_assertion("owned assertion", callback)',
    "Native checkpoint failed: owned assertion\nNative checkpoint failed: runtime payload") == ["owned assertion"]
assert module.failed_assertions(source, "Native checkpoint failed: runtime payload") == []
print("Native capture diagnostic redaction passed")
