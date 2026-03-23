# CloudShift v2

Universal GCP-oriented code refactoring: migrate and modernize cloud usage with a **Rust** transformation core, **CLI**, optional **web UI** and **Cloud Run** API, **Python** bindings (PyO3/Maturin), and an **LSP** plus **VS Code** extension for in-editor diagnostics and actions.

**Repository:** [github.com/asmeyatsky/cloudshift_v2](https://github.com/asmeyatsky/cloudshift_v2) · **License:** MIT

## Requirements

- **Rust** (stable) — workspace builds with `cargo`
- **Node.js 20** — UI tests (`ui/`) and frontend development
- **Python 3.12+** and **maturin** — optional; for `cloudshift-py` bindings

## Quick start

```bash
cargo build
cargo test
cargo clippy
```

Python bindings (development install):

```bash
maturin develop -m crates/cloudshift-py/Cargo.toml
```

Run the CLI after a debug build:

```bash
cargo run -p cloudshift-cli -- --help
```

## Common commands

```bash
cloudshift transform ./path --source aws --dry-run
cloudshift analyse ./path --output json
cloudshift catalogue list --language python
```

## Layout

| Path | Role |
|------|------|
| `crates/cloudshift-core/` | Transformation engine |
| `crates/cloudshift-cli/` | Command-line interface |
| `crates/cloudshift-server/` | HTTP API (e.g. Cloud Run) |
| `crates/cloudshift-py/` | Python bindings |
| `crates/cloudshift-lsp/` | Language Server Protocol |
| `extensions/vscode/` | VS Code extension |
| `patterns/` | Pattern catalogue (TOML) |
| `tests/patterns/` | Pattern fixtures (before/after) |
| `ui/` | Web UI |

**VS Code + LSP:** build the server (`cargo build -p cloudshift-lsp`), then open `extensions/vscode` and launch with F5, or set `cloudshift.lspPath` to `target/debug/cloudshift-lsp`.

## Server (Cloud Run)

Set **`CLOUDSHIFT_API_KEY`** and/or **`CLOUDSHIFT_IAP_AUDIENCE`** (IAP OAuth client ID). Optional: `CLOUDSHIFT_TRANSFORM_RPM`, `CLOUDSHIFT_PATTERNS_DIR`, `CLOUDSHIFT_STATIC_DIR`, `GITHUB_TOKEN`, `CLOUDSHIFT_GITHUB_RPM`.

## Documentation

- **CLI vs UI:** [`docs/WHICH_TOOL.md`](docs/WHICH_TOOL.md)
- **AWS→GCP expectations:** [`docs/AWS_GCP_TRANSFORM.md`](docs/AWS_GCP_TRANSFORM.md)

Home-screen samples in the UI use **source-cloud** snippets (AWS/Azure) as transform inputs, not hand-written GCP targets. For reference GCP examples, see `samples/gcp_reference/`.

## CI

Pushes and pull requests run `cargo check`, `cargo test` (including `ui` tests), `clippy`, `rustfmt --check`, and a **maturin** build for `cloudshift-py`.
