#!/usr/bin/env python3
"""Verify Markdown links, Unicode GitHub anchors, single AGENTS.md, and EN/FR parity.

Uses only the Python standard library so it runs identically locally and in CI.
"""

from __future__ import annotations

import re
import sys
import unicodedata
from collections import Counter
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
IGNORED_TOP_DIRS = {".git", "work", "target", "graft"}

LINK_RE = re.compile(r"(?<!\!)\[(?:[^\[\]]|`[^`]*`)*\]\(([^)\s]+)\)")
GAP_ID_RE = re.compile(r"\b(?:K-\d{2}|P[012]-\d{2})\b")
DECISION_ID_RE = re.compile(r"\bD-\d{3}\b")

FORBIDDEN_STALE_LINKS = (
    "EXIT_CHECKLIST.md",
    "DEPENDENCY_RULES.md",
    "NODE_NETWORK.md",
    "DATA_FLOW.md",
    "STRUCT.md",
    "PROTOTYPE.md",
    "seej/docs/simulation",
    "seej/docs/phase1",
    "seej/clients",
    "seej/schemas",
    "seej/server/README.md",
    "doc/readme.md",
)


def strip_md_inline(text: str) -> str:
    text = re.sub(r"!\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", text)
    return re.sub(r"[*~`]", "", text)


def github_slug(heading: str) -> str:
    lowered = strip_md_inline(heading).strip().lower()
    out: list[str] = []
    for ch in lowered:
        if ch == " ":
            out.append("-")
        elif ch in ("-", "_"):
            out.append(ch)
        elif unicodedata.category(ch)[0] in ("L", "N"):
            out.append(ch)
    return "".join(out)


def extract_anchors(path: Path) -> set[str]:
    anchors: set[str] = set()
    counts: Counter[str] = Counter()
    in_fence = False
    for line in path.read_text(encoding="utf-8").splitlines():
        if re.match(r"^\s*```", line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        match = re.match(r"^\s{0,3}#{1,6}\s+(.*?)\s*#*\s*$", line)
        if match:
            base = github_slug(match.group(1))
            idx = counts[base]
            counts[base] += 1
            anchors.add(base if idx == 0 else f"{base}-{idx}")
    return anchors


def iter_tracked_md_files() -> list[Path]:
    result: list[Path] = []
    for path in sorted(REPO_ROOT.rglob("*.md"), key=lambda p: p.as_posix()):
        rel = path.relative_to(REPO_ROOT)
        if rel.parts[0] in IGNORED_TOP_DIRS:
            continue
        result.append(path)
    return result


def main() -> int:
    md_files = iter_tracked_md_files()
    anchor_map = {p.resolve(): extract_anchors(p) for p in md_files}
    errors: list[str] = []

    # 1. Single AGENTS.md rule (root only).
    agents_files = [
        p.relative_to(REPO_ROOT).as_posix()
        for p in md_files
        if p.name == "AGENTS.md"
    ]
    if agents_files != ["AGENTS.md"]:
        errors.append(
            f"expected exactly one root AGENTS.md, found: {agents_files}"
        )

    # 2. Local links and GitHub Unicode anchors.
    link_count = 0
    for md in md_files:
        in_fence = False
        for lineno, line in enumerate(md.read_text(encoding="utf-8").splitlines(), 1):
            if re.match(r"^\s*```", line):
                in_fence = not in_fence
                continue
            if in_fence:
                continue
            for match in LINK_RE.finditer(line):
                raw = match.group(1).strip()
                if raw.startswith(("http://", "https://", "mailto:")):
                    continue
                link_count += 1
                for stale in FORBIDDEN_STALE_LINKS:
                    if stale in raw:
                        errors.append(
                            f"{md.relative_to(REPO_ROOT)}:{lineno}: stale link target '{raw}'"
                        )
                path_part, frag = raw.split("#", 1) if "#" in raw else (raw, None)
                target = md.resolve() if not path_part else (md.parent / path_part).resolve()
                if not target.exists():
                    errors.append(
                        f"{md.relative_to(REPO_ROOT)}:{lineno}: missing link target '{raw}'"
                    )
                    continue
                if frag is not None:
                    if target not in anchor_map:
                        errors.append(
                            f"{md.relative_to(REPO_ROOT)}:{lineno}: anchor on non-markdown target '{raw}'"
                        )
                    elif frag not in anchor_map[target]:
                        errors.append(
                            f"{md.relative_to(REPO_ROOT)}:{lineno}: missing anchor '#{frag}' "
                            f"in {target.relative_to(REPO_ROOT).as_posix()}"
                        )

    # 3. Duplicate decision headings (D-NNN) in EN/FR DECISIONS.md.
    heading_dec_re = re.compile(r"^##\s+(D-\d{3})\b", re.MULTILINE)
    for rel_dec in ("doc/DECISIONS.md", "doc/fr/DECISIONS.md"):
        ids = heading_dec_re.findall((REPO_ROOT / rel_dec).read_text(encoding="utf-8"))
        dupes = sorted(k for k, v in Counter(ids).items() if v > 1)
        if dupes:
            errors.append(f"duplicate decision headings in {rel_dec}: {dupes}")

    # 4. EN / FR ID parity and dynamic contract_version parity.
    en_rm = set(GAP_ID_RE.findall((REPO_ROOT / "doc/ROADMAP.md").read_text(encoding="utf-8")))
    fr_rm = set(GAP_ID_RE.findall((REPO_ROOT / "doc/fr/ROADMAP.md").read_text(encoding="utf-8")))
    if en_rm != fr_rm:
        errors.append(
            f"ROADMAP gap ID mismatch: EN-FR={sorted(en_rm - fr_rm)}, FR-EN={sorted(fr_rm - en_rm)}"
        )

    en_dec = set(DECISION_ID_RE.findall((REPO_ROOT / "doc/DECISIONS.md").read_text(encoding="utf-8")))
    fr_dec = set(DECISION_ID_RE.findall((REPO_ROOT / "doc/fr/DECISIONS.md").read_text(encoding="utf-8")))
    if en_dec != fr_dec:
        errors.append(
            f"DECISIONS ID mismatch: EN-FR={sorted(en_dec - fr_dec)}, FR-EN={sorted(fr_dec - en_dec)}"
        )

    ver_re = re.compile(r"^contract_version:\s*(\S+)\s*$", re.MULTILINE)
    for rel_en, rel_fr in (
        ("seej/docs/CONTRACT.md", "doc/fr/CONTRACT.md"),
        ("seej/docs/PLUGIN_CONTRACT.md", "doc/fr/PLUGIN_CONTRACT.md"),
    ):
        en_m = ver_re.search((REPO_ROOT / rel_en).read_text(encoding="utf-8"))
        fr_m = ver_re.search((REPO_ROOT / rel_fr).read_text(encoding="utf-8"))
        if not en_m or not fr_m or en_m.group(1) != fr_m.group(1):
            errors.append(
                f"contract_version mismatch between {rel_en} ({en_m.group(1) if en_m else 'missing'}) "
                f"and {rel_fr} ({fr_m.group(1) if fr_m else 'missing'})"
            )

    # 5. Root and workspace rust-toolchain.toml must stay identical.
    root_tc = (REPO_ROOT / "rust-toolchain.toml").read_text(encoding="utf-8").strip()
    srv_tc = (REPO_ROOT / "seej/server/rust-toolchain.toml").read_text(encoding="utf-8").strip()
    if root_tc != srv_tc:
        errors.append("rust-toolchain.toml at repo root differs from seej/server/rust-toolchain.toml")

    if errors:
        print("documentation check failed:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    print(
        f"documentation check passed: {link_count} links across {len(md_files)} files; "
        f"{len(en_rm)} ROADMAP IDs; {len(en_dec)} DECISIONS IDs; single root AGENTS.md."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
