# Demo scripts

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
