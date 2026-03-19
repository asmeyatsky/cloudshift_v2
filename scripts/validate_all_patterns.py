#!/usr/bin/env python3
"""
Run cloudshift transform on one minimal file per Python pattern; assert the
pattern's id appears in the JSON report. Parallel by default.

Usage (repo root):
  python3 scripts/validate_all_patterns.py
  python3 scripts/validate_all_patterns.py --jobs 1   # serial, easier logs
  python3 scripts/validate_all_patterns.py --regenerate   # rebuild JSON from patterns/

Requires: cargo build -p cloudshift-cli
Data: scripts/data/pattern_smoke_cases.json (run scripts/_generate_smoke_cases.py)

Sets CLOUDSHIFT_MATCH_WITHOUT_CONSTRUCTS=1 so matching runs even when semantic
analysis finds no constructs (minimal snippets). Normal `cloudshift transform`
behaviour is unchanged.
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tomllib
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
CASES_JSON = REPO / "scripts" / "data" / "pattern_smoke_cases.json"
OUT_DIR = REPO / "target" / "pattern_validation"
PATTERNS_PY = REPO / "patterns" / "python"


def regenerate_cases() -> None:
    import importlib.util

    p = REPO / "scripts" / "_generate_smoke_cases.py"
    spec = importlib.util.spec_from_file_location("gen", p)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    m.main()


def load_cases() -> list[dict]:
    if not CASES_JSON.exists():
        print(f"Missing {CASES_JSON}; run with --regenerate after adding _generate_smoke_cases.py", file=sys.stderr)
        sys.exit(1)
    return json.loads(CASES_JSON.read_text())


def run_one(case: dict, verbose: bool) -> tuple[str, bool, str]:
    stem = case["stem"]
    source = case["source"]
    code = case["code"]
    expected = case["pattern_id"]
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    py_path = OUT_DIR / f"{stem}.py"
    report_path = OUT_DIR / f"{stem}.report.json"
    py_path.write_text(code, encoding="utf-8")
    cmd = [
        "cargo",
        "run",
        "-p",
        "cloudshift-cli",
        "--quiet",
        "--",
        "transform",
        str(py_path),
        "--source",
        source,
        "--dry-run",
        "--report",
        str(report_path),
    ]
    env = {**os.environ, "CLOUDSHIFT_MATCH_WITHOUT_CONSTRUCTS": "1"}
    r = subprocess.run(cmd, cwd=REPO, capture_output=True, text=True, env=env)
    if r.returncode != 0:
        return stem, False, f"exit {r.returncode}: {r.stderr[-500:]}"
    try:
        rep = json.loads(report_path.read_text())
    except Exception as e:
        return stem, False, f"bad report json: {e}"
    ids = [p["pattern_id"] for p in rep.get("patterns", [])]
    if expected not in ids:
        return stem, False, f"expected id not in matches: {expected!r} got {ids[:5]}..."
    if verbose:
        return stem, True, f"ok ({len(ids)} matches)"
    return stem, True, "ok"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("-v", "--verbose", action="store_true")
    ap.add_argument("--regenerate", action="store_true", help="Rebuild case JSON (needs generator)")
    args = ap.parse_args()
    if args.regenerate:
        regenerate_cases()
        return 0

    cases = load_cases()
    fails: list[tuple[str, str]] = []
    oks = 0
    with ThreadPoolExecutor(max_workers=args.jobs) as ex:
        futs = {ex.submit(run_one, c, args.verbose): c for c in cases}
        for fut in as_completed(futs):
            stem, ok, msg = fut.result()
            if ok:
                oks += 1
                if args.verbose:
                    print(f"[OK] {stem}: {msg}")
            else:
                fails.append((stem, msg))
                print(f"[FAIL] {stem}: {msg}", file=sys.stderr)

    print(f"Patterns validated: {oks}/{len(cases)}")
    if fails:
        print(f"Failed: {len(fails)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
