#!/usr/bin/env python3
"""Build debug test artifacts, then run them with a debug-only cargo adapter.

Pinned makepad-test hardcodes --release for its child app. Set CARGO only in
already-built test executables, not in cargo (which rewrites that variable).
No dependency checkout patching, and no user app process is touched.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
cargo = shutil.which("cargo")
if not cargo:
    raise SystemExit("cargo not found")
args = sys.argv[1:]
filters = []
if "--" in args:
    split = args.index("--")
    filters = args[split + 1:]
    args = args[:split]
command = [cargo, "test", "--locked", "-p", "docgenie-desktop", "--no-run", "--message-format=json", *(args or ["--tests"])]
result = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, text=True)
executables = []
for line in result.stdout.splitlines():
    try:
        message = json.loads(line)
    except json.JSONDecodeError:
        continue
    if message.get("reason") == "compiler-message":
        print(message.get("message", {}).get("rendered", ""), file=sys.stderr, end="")
    if message.get("reason") == "compiler-artifact" and message.get("profile", {}).get("test") and message.get("executable"):
        path = message["executable"]
        if path not in executables:
            executables.append(path)
if result.returncode:
    raise SystemExit(result.returncode)
if not executables:
    raise SystemExit("no test executables found")
env = dict(os.environ)
# Bypass optional cargo wrappers that recursively consult $CARGO; this
# variable deliberately points at our adapter only inside test executables.
real_cargo = subprocess.check_output(["rustup", "which", "cargo"], cwd=ROOT, text=True).strip()
env["DOCGENIE_REAL_CARGO"] = real_cargo
env["CARGO"] = str(ROOT / "tools/test-cargo-debug.sh")
failed = False
for executable in executables:
    print(f"DEBUG test: {executable}", flush=True)
    status = subprocess.run([executable, "--test-threads=1", *filters], cwd=ROOT, env=env).returncode
    failed = failed or status != 0
raise SystemExit(1 if failed else 0)
