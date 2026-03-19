# Demo scripts

## validate_all_patterns.py — every Python pattern, automatically

Regenerates minimal snippets (optional), runs `cloudshift transform --dry-run` per
pattern, and asserts each pattern id appears in the JSON report.

```bash
python3 scripts/_generate_smoke_cases.py   # after adding/changing patterns/python/*.toml
python3 scripts/validate_all_patterns.py   # parallel; use --jobs 1 for quieter logs
```

The validator sets `CLOUDSHIFT_MATCH_WITHOUT_CONSTRUCTS=1` so snippets that do not
trigger the semantic “cloud constructs” heuristics still get pattern matching.
Normal CLI transforms are unchanged.

## report_pattern_gaps.py — AWS/Azure vs catalogue (no network)

Lists boto3 services used in `samples/aws_comprehensive_split/` and Azure managers
with **no / partial** pattern coverage. Regenerate the doc anytime:

```bash
python3 scripts/report_pattern_gaps.py --write docs/PATTERN_COVERAGE_GAPS.md
```

## demo_local.sh — prove patterns work (CLI)

Runs `cloudshift transform` on every Python fixture under `tests/patterns/python/`.  
Use this to confirm the pattern catalogue and engine work **before** debugging the deployed app.

```bash
./scripts/demo_local.sh
```

Requires: from repo root, `patterns/` and `tests/patterns/` present. Uses `CLOUDSHIFT_CATALOGUE_PATH=./patterns` (or the CLI’s default catalogue discovery).

## demo_api.sh — prove the deployed app works (API)

POSTs fixture code to `POST /api/transform` and reports whether the response contains patterns/diff.

```bash
# Local server (no auth)
./scripts/demo_api.sh

# Cloud Run direct URL (API key required)
BASE_URL=https://cloudshift-xxx.run.app API_KEY=your-secret ./scripts/demo_api.sh

# LB URL (IAP; no API key in script)
BASE_URL=https://cloudshift.poc-searce.com ./scripts/demo_api.sh
```

Requires: `curl`; for JSON encoding either `jq` or `python3`.

- **[PASS]** — HTTP 200 and at least one pattern matched or non-empty diff.
- **[WARN]** — HTTP 200 but no patterns/diff: catalogue may be empty in the container, or fixture didn’t match (run `demo_local.sh` to confirm fixtures match locally).
- **[FAIL]** — non-200 (e.g. 401: set `API_KEY`; 404: check LB routes `/api/*` to Cloud Run).

If the API always returns no patterns while `demo_local.sh` shows diffs, the deployment likely has an empty or wrong `CLOUDSHIFT_PATTERNS_DIR` or the patterns directory was not included in the image build.
