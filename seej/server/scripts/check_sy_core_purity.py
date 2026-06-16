#!/usr/bin/env python3
"""Static purity gate for sy_core.

The check intentionally uses only the Python standard library so it can run
from a clean checkout in CI. It scans Rust source after masking comments and
string literals, which keeps documented examples of forbidden APIs from
triggering the gate.
"""

from __future__ import annotations

import argparse
import bisect
import re
import sys
import tempfile
import textwrap
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


SERVER_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CORE_SRC = SERVER_ROOT / "crates" / "sy_core" / "src"


@dataclass(frozen=True)
class Rule:
    rule_id: str
    reason: str
    patterns: tuple[re.Pattern[str], ...]


@dataclass(frozen=True)
class Finding:
    path: Path
    line: int
    column: int
    rule_id: str
    reason: str
    snippet: str


def rx(pattern: str) -> re.Pattern[str]:
    return re.compile(pattern, re.MULTILINE | re.DOTALL)


RULES = (
    Rule(
        "wall-clock-time",
        "sy_core must use injected ISimClock instead of wall-clock APIs.",
        (
            rx(r"\bstd\s*::\s*time\s*::\s*(?:SystemTime|Instant)\b"),
            rx(r"\buse\s+std\s*::\s*time\s*::[^;]*\b(?:SystemTime|Instant|\*)\b"),
            rx(r"\buse\s+std\s*::\s*\{[^;]*\btime\s*::[^;]*\b(?:SystemTime|Instant|\*)\b"),
            rx(r"\b(?:SystemTime|Instant)\s*::\s*(?:now|elapsed|duration_since|checked_duration_since)\b"),
        ),
    ),
    Rule(
        "environment-access",
        "sy_core must not read process environment; inject configuration outside the core.",
        (
            rx(r"\bstd\s*::\s*env\b"),
            rx(r"\buse\s+std\s*::\s*env\b"),
            rx(r"\buse\s+std\s*::\s*\{[^;]*\benv\b"),
            rx(r"\b(?:env|option_env)\s*!"),
        ),
    ),
    Rule(
        "filesystem-io",
        "sy_core must not access filesystem, path, or generic I/O APIs.",
        (
            rx(r"\bstd\s*::\s*(?:fs|path|io)\b"),
            rx(r"\buse\s+std\s*::\s*(?:fs|path|io)\b"),
            rx(r"\buse\s+std\s*::\s*\{[^;]*\b(?:fs|path|io)\b"),
            rx(r"\binclude_(?:str|bytes)\s*!"),
        ),
    ),
    Rule(
        "networking",
        "sy_core must remain headless and must not use networking APIs.",
        (
            rx(r"\bstd\s*::\s*net\b"),
            rx(r"\buse\s+std\s*::\s*net\b"),
            rx(r"\buse\s+std\s*::\s*\{[^;]*\bnet\b"),
            rx(r"\b(?:TcpStream|TcpListener|UdpSocket|ToSocketAddrs|SocketAddr)\b"),
        ),
    ),
    Rule(
        "os-randomness",
        "sy_core must use injected IRng instead of OS or process randomness.",
        (
            rx(r"\brand\s*::\s*rngs\s*::\s*OsRng\b"),
            rx(r"\buse\s+rand\s*::\s*rngs\s*::[^;]*\bOsRng\b"),
            rx(r"\bOsRng\b"),
            rx(r"\brand\s*::\s*(?:random|thread_rng|rng)\b"),
            rx(r"\bgetrandom\b"),
        ),
    ),
    Rule(
        "noncanonical-collections",
        "sy_core canonical paths must not use randomized hash collections; use ordered structures.",
        (
            rx(r"\bstd\s*::\s*collections\s*::[^;]*\b(?:HashMap|HashSet|hash_map|hash_set)\b"),
            rx(r"\buse\s+std\s*::\s*collections\s*::[^;]*\b(?:HashMap|HashSet|hash_map|hash_set)\b"),
            rx(r"\b(?:HashMap|HashSet|RandomState|DefaultHasher)\b"),
        ),
    ),
)


def mask_preserve_newlines(chars: list[str], start: int, end: int) -> None:
    for index in range(start, end):
        if chars[index] not in "\r\n":
            chars[index] = " "


def raw_string_end(source: str, start: int) -> int | None:
    cursor = start
    if source.startswith("br", start):
        cursor += 2
    elif source.startswith("r", start):
        cursor += 1
    else:
        return None

    hashes = 0
    while cursor + hashes < len(source) and source[cursor + hashes] == "#":
        hashes += 1

    quote_index = cursor + hashes
    if quote_index >= len(source) or source[quote_index] != '"':
        return None

    end_marker = '"' + ("#" * hashes)
    end_index = source.find(end_marker, quote_index + 1)
    if end_index == -1:
        return len(source)
    return end_index + len(end_marker)


def quoted_string_end(source: str, start: int) -> int:
    cursor = start + 1
    while cursor < len(source):
        if source[cursor] == "\\":
            cursor += 2
            continue
        if source[cursor] == '"':
            return cursor + 1
        cursor += 1
    return len(source)


def block_comment_end(source: str, start: int) -> int:
    cursor = start + 2
    depth = 1
    while cursor < len(source) and depth > 0:
        if source.startswith("/*", cursor):
            depth += 1
            cursor += 2
            continue
        if source.startswith("*/", cursor):
            depth -= 1
            cursor += 2
            continue
        cursor += 1
    return cursor


def mask_comments_and_literals(source: str) -> str:
    chars = list(source)
    cursor = 0
    while cursor < len(source):
        raw_end = raw_string_end(source, cursor)
        if raw_end is not None:
            mask_preserve_newlines(chars, cursor, raw_end)
            cursor = raw_end
            continue

        if source.startswith("//", cursor):
            end = source.find("\n", cursor)
            if end == -1:
                end = len(source)
            mask_preserve_newlines(chars, cursor, end)
            cursor = end
            continue

        if source.startswith("/*", cursor):
            end = block_comment_end(source, cursor)
            mask_preserve_newlines(chars, cursor, end)
            cursor = end
            continue

        if source[cursor] == '"':
            end = quoted_string_end(source, cursor)
            mask_preserve_newlines(chars, cursor, end)
            cursor = end
            continue

        if source.startswith('b"', cursor):
            end = quoted_string_end(source, cursor + 1)
            mask_preserve_newlines(chars, cursor, end)
            cursor = end
            continue

        cursor += 1

    return "".join(chars)


def line_starts(source: str) -> list[int]:
    starts = [0]
    for match in re.finditer(r"\n", source):
        starts.append(match.end())
    return starts


def line_column(starts: list[int], offset: int) -> tuple[int, int]:
    line = bisect.bisect_right(starts, offset)
    column = offset - starts[line - 1] + 1
    return line, column


def line_snippet(source: str, line: int) -> str:
    lines = source.splitlines()
    if line < 1 or line > len(lines):
        return ""
    snippet = lines[line - 1].strip()
    if len(snippet) > 160:
        return snippet[:157] + "..."
    return snippet


def scan_file(path: Path) -> list[Finding]:
    original = path.read_text(encoding="utf-8")
    scanned = mask_comments_and_literals(original)
    starts = line_starts(original)
    findings: list[Finding] = []
    seen: set[tuple[str, int, int]] = set()

    for rule in RULES:
        for pattern in rule.patterns:
            for match in pattern.finditer(scanned):
                line, column = line_column(starts, match.start())
                key = (rule.rule_id, line, column)
                if key in seen:
                    continue
                seen.add(key)
                findings.append(
                    Finding(
                        path=path,
                        line=line,
                        column=column,
                        rule_id=rule.rule_id,
                        reason=rule.reason,
                        snippet=line_snippet(original, line),
                    )
                )

    return findings


def rust_files(root: Path) -> Iterable[Path]:
    if root.is_file():
        if root.suffix == ".rs":
            yield root
        return

    for path in sorted(root.rglob("*.rs"), key=lambda item: item.as_posix()):
        if path.is_file():
            yield path


def scan_tree(root: Path) -> tuple[list[Finding], int]:
    findings: list[Finding] = []
    count = 0
    for path in rust_files(root):
        count += 1
        findings.extend(scan_file(path))
    findings.sort(key=lambda item: (item.path.as_posix(), item.line, item.column, item.rule_id))
    return findings, count


def display_path(path: Path) -> str:
    try:
        return path.resolve().relative_to(SERVER_ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def print_findings(findings: list[Finding]) -> None:
    print("sy_core purity gate failed:", file=sys.stderr)
    for finding in findings:
        location = f"{display_path(finding.path)}:{finding.line}:{finding.column}"
        print(f"{location}: {finding.rule_id}: {finding.reason}", file=sys.stderr)
        if finding.snippet:
            print(f"  {finding.snippet}", file=sys.stderr)


def assert_case(label: str, code: str, expected_rule: str | None) -> None:
    with tempfile.TemporaryDirectory(prefix="sy-core-purity-") as temp_dir:
        root = Path(temp_dir) / "src"
        root.mkdir(parents=True)
        (root / "lib.rs").write_text(textwrap.dedent(code), encoding="utf-8")
        findings, _ = scan_tree(root)

    if expected_rule is None:
        if findings:
            rules = ", ".join(sorted({finding.rule_id for finding in findings}))
            raise AssertionError(f"{label}: expected no findings, got {rules}")
        return

    if not any(finding.rule_id == expected_rule for finding in findings):
        rules = ", ".join(sorted({finding.rule_id for finding in findings})) or "none"
        raise AssertionError(f"{label}: expected {expected_rule}, got {rules}")


def run_self_test() -> int:
    cases = [
        (
            "comments and strings are ignored",
            """
            //! std::time::SystemTime, std::env::var, HashMap in docs.
            pub const NOTE: &str = "std::fs::read_to_string and OsRng are not code";
            use std::collections::BTreeMap;
            pub fn ordered() -> BTreeMap<u64, u64> { BTreeMap::new() }
            """,
            None,
        ),
        (
            "SystemTime import is rejected",
            "use std::time::SystemTime; pub fn f() { let _ = SystemTime::now(); }",
            "wall-clock-time",
        ),
        (
            "Instant use tree is rejected",
            "use std::time::{Duration, Instant}; pub fn f() { let _ = Instant::now(); }",
            "wall-clock-time",
        ),
        (
            "environment access is rejected",
            "pub fn f() { let _ = std::env::var(\"SEEJ\"); }",
            "environment-access",
        ),
        (
            "env macro is rejected",
            "pub const HOME: &str = env!(\"HOME\");",
            "environment-access",
        ),
        (
            "filesystem access is rejected",
            "pub fn f() { let _ = std::fs::read_to_string(\"world.json\"); }",
            "filesystem-io",
        ),
        (
            "std use tree filesystem access is rejected",
            "use std::{fs, fmt}; pub fn f() { let _ = fs::read_dir(\".\"); }",
            "filesystem-io",
        ),
        (
            "network access is rejected",
            "pub fn f() { let _ = std::net::TcpStream::connect(\"127.0.0.1:1\"); }",
            "networking",
        ),
        (
            "OsRng is rejected",
            "use rand::rngs::OsRng; pub fn f() { let _ = OsRng; }",
            "os-randomness",
        ),
        (
            "HashMap is rejected",
            "use std::collections::HashMap; pub fn f() -> HashMap<u64, u64> { HashMap::new() }",
            "noncanonical-collections",
        ),
        (
            "HashSet is rejected",
            "pub fn f(values: std::collections::HashSet<u64>) { let _ = values; }",
            "noncanonical-collections",
        ),
    ]

    try:
        for label, code, expected_rule in cases:
            assert_case(label, code, expected_rule)
    except AssertionError as error:
        print(f"sy_core purity gate self-test failed: {error}", file=sys.stderr)
        return 1

    print(f"sy_core purity gate self-test passed ({len(cases)} cases).")
    return 0


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Check sy_core deterministic purity rules.")
    parser.add_argument(
        "--root",
        type=Path,
        default=DEFAULT_CORE_SRC,
        help="Rust source root to scan. Defaults to crates/sy_core/src.",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run built-in regression cases for the purity gate.",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)

    if args.self_test:
        return run_self_test()

    root = args.root.resolve()
    if not root.exists():
        print(f"sy_core purity gate root does not exist: {root}", file=sys.stderr)
        return 2

    findings, scanned_files = scan_tree(root)
    if findings:
        print_findings(findings)
        return 1

    print(
        "sy_core purity gate passed: "
        f"scanned {scanned_files} Rust files under {display_path(root)}."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
