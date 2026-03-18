# CloudShift v2

Universal GCP code refactoring engine. Rust core + Python bindings via Maturin/PyO3.

## Build

- `cargo build` — build all crates
- `cargo test` — run all tests
- `cargo clippy` — lint
- `maturin develop -m crates/cloudshift-py/Cargo.toml` — build Python bindings for dev

## Architecture

- `crates/cloudshift-core/` — Pure transformation engine (domain + infrastructure)
- `crates/cloudshift-cli/` — CLI binary (clap)
- `crates/cloudshift-py/` — Python bindings (PyO3)
- `crates/cloudshift-lsp/` — LSP server for IDE extensions
- `patterns/` — GCP Pattern Catalogue (TOML)
- `tests/patterns/` — Pattern test fixtures (before/after pairs)

## Server (Cloud Run)

Set **`CLOUDSHIFT_API_KEY`** and/or **`CLOUDSHIFT_IAP_AUDIENCE`** (IAP OAuth client ID). Optional: `CLOUDSHIFT_TRANSFORM_RPM`, `CLOUDSHIFT_PATTERNS_DIR`, `CLOUDSHIFT_STATIC_DIR`, `GITHUB_TOKEN`, `CLOUDSHIFT_GITHUB_RPM`.

## UI examples (AWS / Azure)

Home-screen samples are **intentional source-cloud code** (Blob, S3, etc.) — inputs to **Transform**, not GCP snippets. LLMs sometimes flag “Azure + GCS” confusion; the Azure SDK here is correct as migration **from** Azure **to** generated GCP code.

**Guided tours** (driver.js): `ui/src/tour/cloudshiftTour.ts` — Home menu + Editor workspace; Help (?) in header and Settings.

## Key Commands

- `cloudshift transform ./path --source aws --dry-run`
- `cloudshift analyse ./path --output json`
- `cloudshift catalogue list --language python`
