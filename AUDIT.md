# CloudShift v2 — Application audit (Cloud Run + SPA)

## 1. Components

| Component | Location | Deployed |
|-----------|----------|----------|
| **HTTP server** | `crates/cloudshift-server` | Single binary in image |
| **React UI** | `ui/` → built to `static/` | Served by server (`ServeDir`) |
| **Transformation engine** | `crates/cloudshift-core` | Linked into server |
| **Pattern catalogue** | `patterns/*.toml` | `/app/patterns` in image |
| **CLI / Python / LSP** | other crates | Dev / CI only (not in minimal image) |

---

## 2. Server routes (`cloudshift-server`)

| Route | Method | Auth | Notes |
|-------|--------|------|--------|
| `/`, assets | GET | No | SPA + static (when `static/` present) |
| `/favicon.ico` | GET | No | 204 |
| `/health`, `/ready` | GET | No | Probes |
| `/api/auth-check` | GET | Yes* | JSON `{ok:true/false}` |
| `/api/transform` | POST | Yes* | JSON body → `TransformResult` (includes `transformed_source`) |

\* **Auth:** Valid **`X-API-Key`** matching `CLOUDSHIFT_API_KEY`, **or** verified **IAP JWT** (`X-Goog-IAP-JWT-Assertion`) when `CLOUDSHIFT_IAP_AUDIENCE` lists the OAuth client ID(s). At least one of API key or IAP audience must be configured.

**Removed (previously weak):** presence-only IAP header, arbitrary `X-Searce-ID`, any `Bearer` token.

**Limits:** Source field max **1 MiB**; **~90 transforms/min/client** (IP or `X-Forwarded-For`), configurable via `CLOUDSHIFT_TRANSFORM_RPM`.

**Headers:** CSP (Monaco-compatible), `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`.

---

## 3. Dockerfile

Multi-stage: Node builds UI → Rust builds server → slim runtime, `nobody`, `patterns` + `static`.

---

## 4. Deploy / secrets

| Variable | Purpose |
|----------|---------|
| `CLOUDSHIFT_API_KEY` | API key auth (direct Run URL, scripts) |
| `CLOUDSHIFT_IAP_AUDIENCE` | Comma-separated IAP OAuth client ID(s) — **required for custom-domain IAP users** |
| `CLOUDSHIFT_TRANSFORM_RPM` | Optional rate limit override |
| `GEMINI_API_KEY` | LLM fallback in engine |

---

## 5. Tests

| Area | Coverage |
|------|----------|
| `cloudshift-core` | Extensive Rust integration + unit tests |
| `cloudshift-server` | Integration tests in `tests/api.rs` (auth, payload limit, health) |
| `ui` | `npm run test` (Vitest) — e.g. `applyDiff` |

---

## 6. PRD

Product requirements: see **`docs/PRD.md`** (summary) and `CloudShift_PRD_v2.0.pdf` if present in repo.

---

## 7. Local dev

```bash
export CLOUDSHIFT_API_KEY=dev
export CLOUDSHIFT_PATTERNS_DIR=$(pwd)/patterns
cargo run -p cloudshift-server
# UI: cd ui && npm run dev  (proxies /api to :8080)
```

For IAP verification locally, set `CLOUDSHIFT_IAP_AUDIENCE` to your OAuth client ID and send a real IAP JWT.
