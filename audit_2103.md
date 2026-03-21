# CloudShift v2 — Comprehensive Audit

**Date:** 2026-03-21
**Commit:** `25ec0ba` (main)
**Scope:** Full end-to-end audit — Rust core, server, CLI, LSP, Python bindings, React UI, patterns, tests, deployment, CI/CD, documentation, PRD alignment

---

## Build & Test Status

| Check | Result |
|-------|--------|
| `cargo build --workspace --exclude cloudshift-py` | **PASS** |
| `cargo build --workspace` (incl. PyO3) | **FAIL** — linker error (expected; needs `maturin`) |
| `cargo test --workspace --exclude cloudshift-py` | **177 passed, 0 failed, 1 ignored** |
| `cargo clippy --workspace` | **PASS** — zero warnings |
| `cargo fmt --check` | **PASS** |

**Note:** `Cargo.toml` has `exclude = ["crates/cloudshift-py"]` but `cloudshift-py` is still listed in `members`, so `--workspace` tries to build it. Must use `--exclude cloudshift-py` explicitly or remove from `members`.

---

## Codebase Overview

| Component | Lines | Tests | Status |
|-----------|-------|-------|--------|
| cloudshift-core | ~10,500 | 131 unit + 24 integration | Production-quality |
| cloudshift-server | ~815 | 8 API tests | Functional, needs hardening |
| cloudshift-cli | ~1,100 | 10 integration | 5/8 commands complete |
| cloudshift-lsp | ~300 | 0 | Skeleton only |
| cloudshift-py | ~490 | 0 | Functional but incomplete |
| React UI | ~3,270 | 2 unit tests | Functional, good UX |
| Patterns (TOML) | 160 files | 10 test suites | 47.8% of PRD target |

---

## 1. RUST CORE (`cloudshift-core`)

### Strengths
- Excellent hexagonal architecture with clean layer separation (domain, analyser, pattern, pipeline, catalogue, diff, ibte, learning, llm_fallback, fixup)
- Tree-sitter-based AST matching — not regex. Semantic correctness
- Zero `unsafe` code. No `.unwrap()` or `.expect()` in production paths
- Path traversal protection via canonicalization checks
- File size limits enforced (10 MB max)
- DAG-orchestrated pipeline with rayon parallelism
- 4-factor weighted confidence model (specificity 35%, version 25%, argument completeness 25%, test coverage 15%)
- Overlap deduplication: sorts by confidence, greedy non-overlapping selection
- Import pre-filtering skips files missing required imports (30-50% faster)
- 131+ unit tests with fixture-based before/after pairs

### Issues

| # | Severity | Finding |
|---|----------|---------|
| C1 | Medium | **Import detection is substring-based** — `source_str.contains(imp)` could match imports in comments/strings. Low risk (pre-filter only; full tree-sitter matching follows) |
| C2 | Medium | **IBTE `extract_put_item_bindings()` is string-based** — uses `call_slice.find("Item=")` instead of tree-sitter. Fragile for complex Item values |
| C3 | Medium | **IBTE only supports Python** — TypeScript/Java/Go chain detection not implemented |
| C4 | Low | **Confidence thresholds hardcoded** (0.90 high, 0.70 medium) — not parameterised in TransformConfig |
| C5 | Low | **Import removal is text-based** — fragile for TypeScript (comments, complex syntax). Should be AST-based |
| C6 | Low | **LLM responses not logged for audit** — can't verify what LLM was asked or returned |
| C7 | Low | **Tree-sitter query validation happens at match time**, not compile time — invalid detect_query in TOML only discovered when transform runs |

---

## 2. SERVER (`cloudshift-server`)

### Strengths
- Clean Axum routes: `/api/transform`, `/api/auth-check`, `/api/github/repo`, `/api/openapi.json`, health probes
- Dual auth: API key (`X-API-Key`) + IAP JWT (`X-Goog-IAP-JWT-Assertion`)
- Per-IP rate limiting (90 transform/min, 15 github/min, configurable)
- Security headers: CSP (Monaco-compatible), X-Frame-Options: DENY, X-Content-Type-Options: nosniff
- Body size limits per route (4 KB github, 4 MB transform)
- Static file serving via tower-http ServeDir (path-traversal safe)
- GitHub import: URL validation, 25 MB zip limit, 900 KB per file, 80 file max, binary detection

### Issues

| # | Severity | Finding |
|---|----------|---------|
| S1 | **High** | **API key comparison is not constant-time** — `s.trim() == k.as_str()` (lib.rs:116) vulnerable to timing attacks. Use `constant_time_eq` or similar |
| S2 | **High** | **Rate limiter mutex: `.lock().unwrap()`** — if any request panics holding the lock, all subsequent requests panic. Use error recovery |
| S3 | **High** | **Rate limiter memory leak** — HashMap of client IPs grows unbounded. No cleanup of old entries. Long-running server accumulates memory indefinitely |
| S4 | Medium | **GitHub error messages leak internal details** — repo existence, privacy status, API rate limits visible to callers. Should return generic errors |
| S5 | Medium | **No total decompressed size limit for ZIP** — individual files capped (900 KB) and count capped (80), but no aggregate decompression limit |
| S6 | Medium | **IAP JWT validation not tested** — no integration tests for token structure, expiry, audience matching |
| S7 | Medium | **Rate limiting untested** — no tests for the rate limit logic, mutex poisoning, or memory growth |
| S8 | Low | **No API versioning** — routes are `/api/transform` not `/api/v1/transform` |
| S9 | Low | **No CORS headers** — fine for same-origin UI, but undocumented for external clients |
| S10 | Low | **reqwest version conflict** — workspace declares `0.12`, server Cargo.toml uses `0.13`. Should align |

---

## 3. CLI (`cloudshift-cli`)

### Command Status

| Command | Status | Notes |
|---------|--------|-------|
| `transform` | **Complete** | Presets, dry-run, auto-apply, confidence threshold, output formats (diff/json/sarif), parallel, LLM fallback |
| `analyse` | **Complete** | Detects cloud patterns without transforming |
| `diff` | **Complete** | Shows changes without applying |
| `report` | **Complete** | Generates human-readable migration report |
| `learn` | **Complete** | Triggers pattern learning from before/after pairs |
| `apply` | **Stub** | Validates diff format only. Prints "not yet implemented" |
| `catalogue` | **Partial** | `pending/promote/reject/learn-stats` work. **`list/search/info/stats` bail with "not yet implemented"** |
| `validate` | **Stub** | Prints "validation checks are not yet wired up" |

### Issues

| # | Severity | Finding |
|---|----------|---------|
| L1 | **High** | **`apply` command doesn't apply patches** — users run `cloudshift apply` and get "not implemented" |
| L2 | **High** | **4 catalogue subcommands stubbed** — `list/search/info/stats` not wired to core APIs that already exist |
| L3 | Medium | **`validate` command is a no-op** — no actual validation logic |
| L4 | Low | **Error output uses `{:?}` debug format** — noisy for end users |
| L5 | Low | **Minimal test coverage** — only happy-path integration tests via `assert_cmd` |

---

## 4. LSP SERVER (`cloudshift-lsp`)

### Status: Skeleton only (Q4 2026 GA scope)

- JSON-RPC protocol infrastructure is correct (Content-Length framing, initialize/shutdown/exit)
- **`didOpen` publishes empty diagnostics** — no analysis happens
- **`codeAction` returns empty list** — no quick-fixes
- **`didChange` / `didClose` are no-ops** — no document state tracking
- `cloudshift-core` dependency is present but unused
- **Zero tests**

| # | Severity | Finding |
|---|----------|---------|
| P1 | **High** (for GA) | LSP handlers are empty stubs — server is non-functional for IDE integration |
| P2 | Medium | No document state management (HashMap of open files) |
| P3 | Low | Tokio dependency included but unused (blocking stdin read) |

---

## 5. PYTHON BINDINGS (`cloudshift-py`)

### Exposed API
- `transform_file()`, `transform_repo()`, `transform_repo_stream()`, `catalogue_search()`
- Classes: `SourceCloud`, `OutputFormat`, `TransformConfig`, `TransformResult`, `FileChange`, `RepoReport`, `PatternInfo`

### Issues

| # | Severity | Finding |
|---|----------|---------|
| Y1 | Medium | **`catalogue_search()` is stubbed** — returns empty list |
| Y2 | Medium | **`transform_repo_stream()` is not streaming** — wraps `transform_repo()` and returns a list |
| Y3 | Medium | **`threshold` and `auto_apply_threshold` params parsed but unused** (silently ignored via `let _ =`) |
| Y4 | Low | No test coverage (excluded from workspace `cargo test`) |
| Y5 | Low | No Python docstrings — IDE autocomplete won't show descriptions |
| Y6 | Low | Error chain not preserved — only top-level message reaches Python |

---

## 6. REACT UI

### Strengths
- Clean component separation: App, Header, HomeView, TransformView, SettingsModal, InsightsBar
- Zustand store — well-designed global state, no prop drilling, localStorage sync
- Monaco editor with custom dark theme, diff view, 9 language mappings
- driver.js guided tours for home and workspace
- Strict TypeScript (`strict: true`), no `any` types found
- No XSS vectors — no `innerHTML` or `dangerouslySetInnerHTML` anywhere
- Good loading/error/empty states throughout

### Issues

| # | Severity | Finding |
|---|----------|---------|
| U1 | Medium | **No React Error Boundary** — unhandled render errors crash the entire app |
| U2 | Medium | **No request timeout on fetch calls** — slow networks can hang the UI indefinitely |
| U3 | Medium | **API responses not runtime-validated** — `res.json()` trusted without schema check |
| U4 | Medium | **TransformView is 833 lines** — mega-component combining source editor, batch sidebar, result panel, insights |
| U5 | Low | **Network error vs auth error not distinguished** — `checkAuth()` returns `false` for both |
| U6 | Low | **localStorage writes have no try/catch** — fail silently in private browsing |
| U7 | Low | **HCL mapped to `plaintext`** — no syntax highlighting for Terraform files |
| U8 | Low | **Missing ARIA labels** on form inputs (API key, GitHub URL) |
| U9 | Low | **No component or integration tests** — only 2 utility function tests |
| U10 | Low | **API key stored plaintext in localStorage** — documented risk, acceptable trade-off |

---

## 7. PATTERNS & TEST COVERAGE

### Pattern Catalogue

| Language | Count | PRD Target | % Complete |
|----------|-------|------------|------------|
| Python | 76 | 102 | 74.5% |
| TypeScript | 25 | 62 | 40.3% |
| Java | 15 | 44 | 34.1% |
| Go | 12 | 36 | 33.3% |
| HCL | 32 | 91 | 35.2% |
| **Total** | **160** | **335** | **47.8%** |

### Pattern Quality
- Consistent TOML structure across all files
- Tree-sitter queries syntactically correct (spot-checked 20)
- Confidence scores well-calibrated (0.77-0.97 range)
- Templates include TODO comments where mapping is incomplete
- Import add/remove handling correct throughout

### Pattern Issues

| # | Severity | Finding |
|---|----------|---------|
| T1 | **High** | **175-pattern shortfall** — 47.8% of MVP target. Cannot claim "universal" or "comprehensive" |
| T2 | Medium | **Test coverage is ~6%** — only 10 test suites for 160 patterns. Go and Java patterns completely untested |
| T3 | Medium | **Pseudo-bindings** (`__key__`, `__mapped_role__`, `__container_id__`) unclear resolution strategy |
| T4 | Low | **No error-case patterns** — retry logic, exception mapping, credential transforms not addressed |
| T5 | Low | **GCP reference samples are hand-written**, not generated by CloudShift — can't validate transform output |

---

## 8. DEPLOYMENT & CI/CD

### Dockerfile
- Multi-stage build (node:20-slim → rust:1-bookworm → debian:bookworm-slim)
- Non-root execution (`USER nobody`)
- Only `ca-certificates` installed in runtime
- **Issue:** No `.dockerignore` file. Base images use floating tags (not pinned to digests)

### CI/CD Pipelines

| Workflow | Status | Issues |
|----------|--------|--------|
| `ci.yml` | **Working** | Missing: cargo-audit, CodeQL SAST, coverage reporting |
| `deploy-cloudrun.yml` | **Working** | Uses SA key (not WIF), secrets in env vars not Secret Manager |
| `release.yml` | **Working** | No binary code signing, no SLSA provenance |
| `cloudshift.yml` (PR gate) | **Working** | Unpinned cloudshift version |

### Deployment Issues

| # | Severity | Finding |
|---|----------|---------|
| D1 | **High** | **GCP SA key in GitHub Secrets** — long-lived credential, no rotation. Should use Workload Identity Federation |
| D2 | **High** | **No dependency vulnerability scanning** — no `cargo-audit` or `cargo-deny` in CI |
| D3 | Medium | **No container image scanning** (Trivy or similar) |
| D4 | Medium | **No monitoring/alerting setup documented** — Cloud Run metrics, error rate, latency |
| D5 | Medium | **Secrets passed as env vars** — should use Cloud Run Secret Manager references |
| D6 | Medium | **CSP uses `unsafe-inline` for scripts** — needed for Monaco, but could use nonce-based approach |
| D7 | Low | **No disaster recovery runbook** — no documented rollback procedure |
| D8 | Low | **Deployment script has no auth validation** — doesn't verify gcloud permissions before starting |

---

## 9. PRD ALIGNMENT

### MVP Goals (Q3 2026)

| # | Goal | Status | Notes |
|---|------|--------|-------|
| 1 | AWS SDK → GCP (Python, TS, Java, Go) | **Partial** | Patterns exist but 47.8% of target |
| 2 | Azure SDK → GCP | **Partial** | Fewer patterns than AWS |
| 3 | Terraform HCL → Google provider | **Partial** | 32/91 patterns |
| 4 | Dockerfile transforms | **Partial** | Language detected; limited patterns |
| 5 | CI/CD pipeline transforms | **Minimal** | No visible CI/CD-specific patterns |
| 6 | Secrets (AWS → Secret Manager) | **Done** | Patterns exist |
| 7 | Monorepo + multi-file + dependency graph | **Done** | CLI `transform ./path --parallel` works |
| 8 | Ship Rust core, Python module, CLI | **Done** | All three build and run |
| 9 | Diff-first output | **Done** | `--dry-run` default, reviewable diffs |
| 10 | 200+ catalogue rules at MVP | **Not met** | 160/335 (47.8%) |

### GA Goals (Q4 2026)

| # | Goal | Status |
|---|------|--------|
| 11 | New languages (C#, Ruby, Kotlin, Scala, Rust) | Not started |
| 12 | AI-assisted fallback (local LLM) | Infrastructure exists (uses Anthropic API, not local) |
| 13 | Self-learning pattern store | Infrastructure exists, dedup not verified |
| 14 | VS Code & JetBrains extensions (LSP) | Skeleton only — handlers empty |
| 15 | Validation Agent | Stub command, no logic |
| 16 | Pulumi IaC | No patterns |

### Documentation vs Reality

| Document | Accuracy |
|----------|----------|
| CLAUDE.md | Current and correct |
| AUDIT.md | Correct but incomplete — missing feature progress tracking, LSP/catalogue status |
| FIXES.md | Accurate — documents 5 fixes, only Fix 1 partially applied |
| docs/WHICH_TOOL.md | Current |
| docs/AWS_GCP_TRANSFORM.md | Current |
| docs/ADDING_PYTHON_PATTERNS.md | Current |
| docs/PATTERN_COVERAGE_GAPS.md | Stale — no timestamp, claims not verified against current state |

---

## 10. PREVIOUS AUDIT FIXES STATUS

| Fix | Description | Status |
|-----|-------------|--------|
| Fix 1 | PyO3 workspace exclude | **Partial** — `exclude` set but `members` still includes it. `cargo build --workspace` fails without `--exclude` |
| Fix 2 | Catalogue CLI commands | **Not started** — 4 subcommands still bail with "not implemented" |
| Fix 3 | LSP handler implementation | **Not started** — handlers publish empty results |
| Fix 4 | Learning pipeline dedup | **Uncertain** — infrastructure exists, dedup check unverified |
| Fix 5 | CI pipeline update | **Uncertain** — needs verification that CI respects exclude |

---

## 11. PRIORITISED RECOMMENDATIONS

### Critical (Block release)

1. **Fix the workspace members/exclude inconsistency** — either remove `cloudshift-py` from `members` or ensure CI always uses `--exclude`. Current state means `cargo build --workspace` fails
2. **Add constant-time API key comparison** (S1) — trivial fix, significant security improvement
3. **Fix rate limiter panic risk** (S2) — replace `.lock().unwrap()` with proper error recovery
4. **Decide on pattern count target** (T1) — either add 40+ patterns to reach 200, or revise PRD to reflect 160

### High Priority (Before production)

5. **Implement catalogue CLI commands** (L2) — `list/search/info/stats`. Core APIs exist, just need wiring
6. **Implement `apply` command** (L1) — users need to apply diffs
7. **Add cargo-audit to CI** (D2) — catches known CVEs in dependencies
8. **Migrate to Workload Identity Federation** (D1) — eliminate long-lived SA key
9. **Rate limiter memory cleanup** (S3) — add periodic eviction of old client entries
10. **Add React Error Boundary** (U1) — prevent full-app crashes on render errors

### Medium Priority (Soon after launch)

11. **Add request timeouts to UI fetch calls** (U2)
12. **Runtime-validate API responses** (U3) — Zod or similar
13. **IBTE extract_put_item_bindings → tree-sitter** (C2) — fragile string parsing
14. **Expand pattern test coverage** (T2) — Go and Java patterns have zero tests
15. **Generic error messages for GitHub endpoint** (S4) — don't leak repo existence
16. **Add monitoring/alerting** (D4) — Cloud Run dashboards, error rate alerts
17. **Distinguish network vs auth errors in UI** (U5)
18. **Pin Docker base image digests** (D8)
19. **Align reqwest versions** (S10) — workspace 0.12 vs server 0.13

### Low Priority (Nice-to-have)

20. LSP handler implementation (P1) — GA scope, can defer
21. Validation command implementation (L3) — GA scope
22. Python bindings: fix unused params, add docstrings (Y3, Y5)
23. Extract TransformView into subcomponents (U4)
24. ARIA labels on form inputs (U8)
25. Component + integration tests for UI (U9)
26. HCL syntax highlighting (U7)
27. CSP nonce-based inline scripts (D6)
28. SLSA provenance attestation on releases

---

## 12. SECURITY SUMMARY

| Area | Rating | Key Issue |
|------|--------|-----------|
| Rust core | **A** | No unsafe code, path traversal protection, file size limits |
| Server auth | **B** | Timing attack on API key comparison |
| Server rate limiting | **C+** | Mutex poisoning risk, memory leak |
| GitHub integration | **B+** | Info disclosure in error messages |
| Static file serving | **A** | ServeDir handles path traversal |
| UI | **A-** | No XSS; plaintext localStorage key (acceptable) |
| CI/CD | **C+** | SA key credentials, no vuln scanning, no image scanning |
| Docker | **B+** | Good multi-stage, non-root; missing .dockerignore, floating tags |

---

## 13. OVERALL ASSESSMENT

CloudShift v2 has a **solid engineering foundation** — the Rust core is well-architected, the server is functional, the UI is polished, and the pattern system is well-designed. The 177-test suite passes cleanly, clippy is green, and the build is reproducible.

**What's working well:**
- Hexagonal architecture with clean separation
- Tree-sitter-based semantic analysis (not regex)
- DAG pipeline with rayon parallelism
- Dual auth (API key + IAP JWT)
- React + Monaco + Zustand frontend
- Multi-stage Docker build

**What needs stabilisation:**
- Workspace Cargo.toml members/exclude conflict
- Rate limiter robustness (panic risk, memory leak)
- API key timing attack
- Several stubbed CLI commands users will hit

**What's the biggest gap:**
- Pattern catalogue at 47.8% of PRD target (160/335)
- This is the core product differentiator and needs a decision: ship fewer with honest documentation, or sprint to add more

**Release readiness:** The system is **functionally complete for a demo/beta** but has **4 critical items** (workspace fix, timing attack, rate limiter, pattern count decision) that should be addressed before production release.
