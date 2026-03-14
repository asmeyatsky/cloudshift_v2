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

## Key Commands

- `cloudshift transform ./path --source aws --dry-run`
- `cloudshift analyse ./path --output json`
- `cloudshift catalogue list --language python`
