#!/usr/bin/env python3
"""Single unified validation entry point for Seej (local & CI).

Runs purity checks, dependency-DAG checks, documentation checks, Rust format,
Clippy, unit/integration tests, strict Rustdoc, and cargo-deny from seej/server
so rust-toolchain.toml pins the exact Rust version on every OS and shell.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

SERVER_ROOT = Path(__file__).resolve().parents[1]


def run_step(label: str, cmd: list[str], *, env: dict[str, str] | None = None) -> None:
    print(f"==> {label}: {' '.join(cmd)}", flush=True)
    result = subprocess.run(cmd, cwd=SERVER_ROOT, env=env)
    if result.returncode != 0:
        sys.exit(result.returncode)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Run full Seej validation suite.")
    parser.add_argument(
        "--ci",
        action="store_true",
        help="Fail if optional local tools such as cargo-deny are missing.",
    )
    parser.add_argument(
        "--skip-docs",
        action="store_true",
        help="Skip Markdown documentation link/parity checks.",
    )
    args = parser.parse_args(argv)

    py = sys.executable

    run_step("purity self-test", [py, "-B", "scripts/check_sy_core_purity.py", "--self-test"])
    run_step("pure crates purity", [py, "-B", "scripts/check_sy_core_purity.py"])
    run_step("dependency DAG", [py, "-B", "scripts/check_dag.py"])
    if not args.skip_docs:
        run_step("documentation links & parity", [py, "-B", "scripts/check_docs.py"])

    run_step("rustfmt", ["cargo", "fmt", "--all", "--", "--check"])
    run_step(
        "clippy",
        ["cargo", "clippy", "--locked", "--workspace", "--all-targets", "--", "-D", "warnings"],
    )
    run_step("tests", ["cargo", "test", "--locked", "--workspace", "--all-targets"])

    doc_env = os.environ.copy()
    doc_env["RUSTDOCFLAGS"] = "-D warnings"
    run_step(
        "rustdoc",
        ["cargo", "doc", "--locked", "--workspace", "--no-deps"],
        env=doc_env,
    )

    if shutil.which("cargo-deny"):
        run_step(
            "cargo-deny",
            ["cargo", "deny", "check", "advisories", "licenses", "bans", "sources"],
        )
    elif args.ci:
        print("error: cargo-deny is required in --ci mode", file=sys.stderr)
        return 1
    else:
        print("==> cargo-deny: skipped (not installed locally)", flush=True)

    print("All Seej checks passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
