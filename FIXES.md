# CloudShift v2 — Remaining Fixes Before Go-Live

## Fix 1: PyO3 Python Bindings Build Failure

**Problem:** `cargo build --workspace` fails with undefined Python symbol linker errors (`_PyBaseObject_Type`, `_PyBool_Type`, etc.) because `cloudshift-py` is a `cdylib` that requires maturin to link against the Python interpreter. Running `cargo build` directly can't resolve Python symbols.

**Fix (partially applied — workspace Cargo.toml already edited):**
- `Cargo.toml` (workspace root): Add `exclude = ["crates/cloudshift-py"]` so `cargo build/test/clippy --workspace` skips it.
- `.github/workflows/ci.yml`: Update `cargo check/test/clippy` jobs to use `--workspace` (which now excludes cloudshift-py). The existing `python` job already uses `maturin build` correctly.
- Verify: `cargo build --workspace` should succeed. `maturin develop -m crates/cloudshift-py/Cargo.toml` builds the Python wheel separately.

**Files to change:**
- `Cargo.toml` (already done)
- `.github/workflows/ci.yml` — no changes needed, `--workspace` will respect the exclude

## Fix 2: Implement Stubbed Catalogue CLI Commands

**Problem:** `catalogue list`, `catalogue search`, `catalogue info`, and `catalogue stats` all bail with "not yet implemented". The core `Catalogue` struct already has full APIs: `all_patterns()`, `get_patterns(language, source)`, `search(query)`, `get_by_id(id)`, `count()`.

**Fix:** Replace the bail stubs in `crates/cloudshift-cli/src/commands/catalogue.rs`:

### `catalogue list`
- Load catalogue via `discover_catalogue_path()` → `Catalogue::from_directory()`
- Apply optional `--language`, `--source`, `--tag` filters using `get_patterns()` and tag filtering
- Print each pattern: ID, language, source cloud, confidence, description
- Show total count at the end

### `catalogue search`
- Load catalogue, call `catalogue.search(&query)` (already implemented in core)
- Print matching patterns same format as list

### `catalogue info`
- Load catalogue, call `catalogue.get_by_id(&PatternId::new(&id))`
- Print all fields: ID, description, source, language, confidence, tags, detect query, transform template, import add/remove

### `catalogue stats`
- Load catalogue, print: total patterns, breakdown by language, breakdown by source cloud, average confidence

**Files to change:**
- `crates/cloudshift-cli/src/commands/catalogue.rs`

**Key types from core:**
- `CompiledPattern { id, description, source, language, confidence, tags, detect_query, detect_imports, transform_template, import_add, import_remove, bindings }`
- `Catalogue` implements `PatternRepositoryPort` with `get_patterns()`, `get_by_id()`, `search()`, `count()`

## Fix 3: Implement LSP Server Handlers

**Problem:** `crates/cloudshift-lsp/src/main.rs` has the full JSON-RPC/LSP protocol infrastructure working but three handlers are stubbed:
- `handle_did_open()` — publishes empty diagnostics
- `handle_code_action()` — returns empty list
- `textDocument/didChange` — does nothing

**Fix:**

### `handle_did_open` (line 148)
- Extract `textDocument.uri` and `textDocument.text` from params
- Convert URI to file path (strip `file://` prefix)
- Detect language from filename via `Language::from_filename()`
- Build a `TransformConfig` with `catalogue_path` from `discover_catalogue_path()` or env var
- Call `cloudshift_core::pipeline::transform_source()` — or simpler: just load catalogue and run pattern matching directly
- Convert each `PatternMatch` to an LSP Diagnostic:
  - `range`: map `span.start_byte/end_byte` to line/character using the document text
  - `severity`: map confidence (high → Information, low → Warning)
  - `source`: "cloudshift"
  - `message`: pattern description + source construct
- Publish via `textDocument/publishDiagnostics` notification

### `handle_code_action` (line 171)
- Extract document URI, range, and diagnostics from params
- For each diagnostic in the request range, look up the corresponding pattern
- Generate a `CodeAction` with:
  - `title`: "Migrate to GCP: {pattern description}"
  - `kind`: "quickfix"
  - `edit`: `WorkspaceEdit` with a `TextEdit` containing the replacement text
- Return array of CodeActions

### `textDocument/didChange` (line 267)
- Re-run analysis on new document content (same as didOpen but with updated text)
- Publish updated diagnostics

### Document state
- Add a `HashMap<String, String>` field to track document URI → content
- Update on didOpen/didChange, remove on didClose

**Files to change:**
- `crates/cloudshift-lsp/src/main.rs`

**Needs added to LSP Cargo.toml dependencies:**
- `cloudshift-core` is already a dependency

## Fix 4: Learning Pipeline Deduplication

**Problem:** If the same LLM fallback runs twice on the same file, duplicate candidate patterns are saved to `learned/`. No dedup check exists.

**Fix:** In `crates/cloudshift-core/src/learning/store.rs`, update `save_candidate()`:

- Before writing a new candidate, scan existing files in the target directory
- Compare `source_construct` + `target_construct` + `language` (extracted from existing TOML files via `extract_field`)
- If a matching candidate already exists, skip saving and log a dedup message
- Return the existing path instead of creating a new file

Alternative simpler approach: hash the `pattern_output + llm_output` content and use it as part of the filename. If file already exists, skip.

**Files to change:**
- `crates/cloudshift-core/src/learning/store.rs` — add dedup check in `save_candidate()`
- `crates/cloudshift-core/src/learning/generator.rs` — add a `content_hash` field to `CandidatePattern`

## Fix 5: CI Pipeline Update

**Problem:** CI runs `cargo check/test/clippy --workspace` which will now correctly skip cloudshift-py due to the workspace exclude. But the CI should also run the new learning tests explicitly.

**Fix:** The existing CI already runs `cargo test --workspace` which will pick up the new `learning_test.rs` integration tests automatically. Verify:

- `cargo test --workspace` runs all 152+ tests including the 12 learning tests
- `cargo clippy --workspace` passes clean
- `maturin build` in the python job still works

**Files to change:**
- `.github/workflows/ci.yml` — likely no changes needed, but verify the `python` job still works since cloudshift-py is now excluded from workspace. May need to add `maturin build -m crates/cloudshift-py/Cargo.toml` explicitly (already does this).

---

## Execution Order

1. Fix 1 (PyO3) — already partially done, just verify
2. Fix 4 (Dedup) — small, self-contained
3. Fix 2 (Catalogue CLI) — moderate, uses existing core APIs
4. Fix 3 (LSP) — largest, but well-scoped
5. Fix 5 (CI) — verify everything passes

## Verification

```bash
cargo build --workspace          # Should succeed (no PyO3 linker errors)
cargo test --workspace           # All 152+ tests pass
cargo clippy --workspace         # Clean
cargo build -p cloudshift-lsp    # LSP builds
maturin build -m crates/cloudshift-py/Cargo.toml  # Python wheel builds
```
