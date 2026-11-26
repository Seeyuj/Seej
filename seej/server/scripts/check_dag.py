#!/usr/bin/env python3
"""Verify Seej's workspace dependency DAG across ALL dependency kinds.

Checks normal, dev, and build dependencies (no loophole via [dev-dependencies]
or [build-dependencies]). Uses only the Python standard library.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

SERVER_ROOT = Path(__file__).resolve().parents[1]

EXPECTED_PURE_DEPS: dict[str, set[str]] = {
    "sy_types": set(),
    "sy_api": {"sy_types"},
    "sy_core": {"sy_types", "sy_api"},
}


def main() -> int:
    raw = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
        cwd=SERVER_ROOT,
        text=True,
    )
    metadata = json.loads(raw)
    packages = {pkg["name"]: pkg for pkg in metadata["packages"]}

    errors: list[str] = []
    for crate, expected in EXPECTED_PURE_DEPS.items():
        if crate not in packages:
            errors.append(f"required crate missing from workspace: {crate}")
            continue
        actual_all = {dep["name"] for dep in packages[crate]["dependencies"]}
        if actual_all != expected:
            errors.append(
                f"{crate} must have exact deps (normal+dev+build) {sorted(expected)}; "
                f"got {sorted(actual_all)}"
            )

    if errors:
        print("dependency DAG check failed:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    print(
        "dependency DAG check passed (normal+dev+build): "
        f"{', '.join(sorted(packages.keys()))}."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
