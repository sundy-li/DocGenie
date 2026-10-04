#!/usr/bin/env python3
"""Prepare a pinned private Makepad checkout without moving existing checkouts."""
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
PIN = json.loads((ROOT / "runtime.lock.json").read_text())["makepad"]
TARGET = ROOT / PIN["directory"]


def git(*args):
    return subprocess.check_output(["git", "-C", str(TARGET), *args], text=True).strip()


def main():
    if not TARGET.exists():
        if "--check" in sys.argv:
            raise SystemExit("runtime missing: run python3 tools/setup-runtime.py")
        TARGET.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(["git", "clone", "--no-checkout", PIN["repository"], str(TARGET)], check=True)
        subprocess.run(["git", "-C", str(TARGET), "checkout", "--detach", PIN["revision"]], check=True)
    actual = git("rev-parse", "HEAD")
    if actual != PIN["revision"] or git("status", "--porcelain"):
        raise SystemExit("runtime revision mismatch or local edits; refusing to alter checkout")
    print(f"runtime verified: {actual}")


if __name__ == "__main__":
    main()
