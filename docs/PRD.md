# CloudShift v2 — PRD alignment

Formal requirements: use **`CloudShift_PRD_v2.0.pdf`** if present in the repo root; otherwise treat this file + `AUDIT.md` as the living spec.

## Implemented vs typical PRD themes

| Theme | Status |
|-------|--------|
| AWS/Azure → GCP code transformation | Core engine + patterns + UI |
| Pattern catalogue | `patterns/*.toml`, CLI `catalogue` |
| In-browser try/transform | React UI + `POST /api/transform` |
| Secure enterprise access | IAP JWT verification + API key; rate limits |
| Observability / audit | Structured logs; extend per PRD for audit trails |

## User guidance: UI vs CLI

See **`docs/WHICH_TOOL.md`** — web UI for try/batch; **CLI** for full-repo transforms (`cloudshift transform ./path`, reports, parallel). Same engine per file; monolithic multi-service files need splitting first.

## Gaps to track in PRD revisions

- Optional: catalogue browser in UI, org-wide audit logging, SLA targets, async server-side repo jobs.

When the PDF is updated, refresh this file with any new acceptance criteria.
