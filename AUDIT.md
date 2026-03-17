# CloudShift v2 — Full app audit for Cloud Run

## 1. What is the “full app”

| Component | Location | Deployed |
|-----------|----------|----------|
| **HTTP server** | `crates/cloudshift-server` | ✅ Single binary in image |
| **Transformation engine** | `crates/cloudshift-core` | ✅ Linked into server |
| **Pattern catalogue** | `patterns/*.toml` | ✅ Copied to `/app/patterns` in image |
| **CLI / Python / LSP** | other crates | ❌ Not in Cloud Run (dev/CI only) |

There is **no separate web UI** in this repo. The “full app” for Cloud Run is the **API server** (health, auth, `POST /api/transform`).

---

## 2. Server surface (cloudshift-server)

| Route | Method | Auth | Purpose |
|-------|--------|------|---------|
| `/` | GET | Yes | Root / liveness (returns "ok") |
| `/index.html` | GET | Yes | Same as `/` |
| `/favicon.ico` | GET | No | 204 (avoid 404) |
| `/health` | GET | No | Health check (returns "ok") |
| `/ready` | GET | No | Readiness (returns "ready") |
| `/api/transform` | POST | Yes | Transform in-memory source → JSON result |
| *other* | * | No | 404 "Not found" |

**Auth:** IAP (`X-Goog-IAP-JWT-Assertion`), `X-Searce-ID`, `Authorization: Bearer`, or `X-API-Key` (must match `CLOUDSHIFT_API_KEY`).

**`POST /api/transform` body (JSON):**
- `source` (string, required)
- `language` (required: `python`, `typescript`, `javascript`, `java`, `go`, `hcl`, `yaml`, `dockerfile`, `json`)
- `source_cloud` (optional: `aws`, `azure`, `any`)
- `path_hint` (optional, e.g. `main.py`)

**Response:** `TransformResult` (path, language, diff, patterns, confidence, warnings, applied).

---

## 3. Dockerfile

- **Build:** `rust:1-bookworm` → `cargo build --release -p cloudshift-server`.
- **Runtime:** `debian:bookworm-slim`, binary + `patterns/` → `/app/patterns`.
- **Env:** `PORT=8080`, `CLOUDSHIFT_PATTERNS_DIR=/app/patterns`.
- **User:** `nobody`.
- **CMD:** `cloudshift-server`.

**Check:** `COPY patterns /app/patterns` requires `patterns/` in the build context (repo root). It is present and not in `.gitignore`.

---

## 4. GitHub Actions (deploy-cloudrun.yml)

| Step | Status |
|------|--------|
| Trigger | `push` → `main` |
| Job 1: test | checkout, Rust (clippy, rustfmt), `cargo check`, `cargo test`, `cargo clippy`, `cargo fmt --check`, maturin build (cloudshift-py) |
| Job 2: deploy | needs test, checkout, auth (`GCP_SA_KEY`), setup-gcloud, `gcloud run deploy ... --source .` |
| Deploy flags | `--allow-unauthenticated`, `--ingress=all`, `--set-env-vars` for GEMINI_API_KEY, CLOUDSHIFT_API_KEY, CLOUDSHIFT_PATTERNS_DIR |

**Secrets:** `GCP_SA_KEY` (required), `GEMINI_API_KEY`, `CLOUDSHIFT_API_KEY` (optional).

---

## 5. Cloud Run runtime

| Item | Value |
|------|--------|
| Project | emea-mas |
| Region | europe-west1 |
| Service | cloudshift |
| Direct URL | https://cloudshift-cux4sclfpq-ew.a.run.app |
| IAP URL | https://cloudshift.poc-searce.com |
| Ingress | all (direct + LB) |
| Invoker | allUsers (so LB can call) |

**Env at runtime:** `PORT` (set by Cloud Run), `CLOUDSHIFT_PATTERNS_DIR=/app/patterns`, `GEMINI_API_KEY`, `CLOUDSHIFT_API_KEY` (from workflow).

---

## 6. Gaps and fixes applied

- **Patterns in image:** Dockerfile copies `patterns/` and sets `CLOUDSHIFT_PATTERNS_DIR`; workflow also sets it in `--set-env-vars`. ✅
- **Health:** Use `/health` for probes (returns 200). Root `/` returns 401 without auth; Cloud Run still sees a response. ✅
- **No .gcloudignore:** Build context uses `.gitignore`; `target/` excluded, `patterns/` included. ✅

---

## 7. How to get it running

1. **Secrets (GitHub):** `GCP_SA_KEY`, optionally `GEMINI_API_KEY`, `CLOUDSHIFT_API_KEY`.
2. **Push to `main`** → workflow runs tests then deploys.
3. **Access:**  
   - **Browser (IAP):** https://cloudshift.poc-searce.com  
   - **Direct:** https://cloudshift-cux4sclfpq-ew.a.run.app (e.g. `curl` with auth header or `/health`).
4. **API:** `POST /api/transform` with JSON body and one of the auth headers above.

---

## 8. Quick verification before deploy

```bash
cargo build --release -p cloudshift-server
cargo test --workspace
```

Then push to `main` and watch the Actions tab for the deploy.
