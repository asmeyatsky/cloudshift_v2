# Which tool should I use?

CloudShift offers a **web UI**, a **CLI**, and **API** (what the UI calls). They share the **same Rust transform engine**—the difference is *workflow*, not magic.

## Quick decision

| Your situation | Best tool | Why |
|----------------|-----------|-----|
| Try an example, paste a snippet, quick diff | **Web UI** | Fast feedback, no install. |
| Many files from GitHub / ZIP / folder (tens of files) | **Web UI batch** *or* **CLI** | UI: good for demos and moderate batches. CLI: better for large trees, scripting, CI. |
| **Entire repository** (hundreds/thousands of files) | **CLI** | Parallel workers, `--report` JSON, glob include/exclude, fits CI/CD. |
| **One huge file** (e.g. 1500 lines, many AWS services in one module) | **Split first**, then CLI or UI | Engine applies **patterns per file**; a kitchen-sink file gets partial rewrites and mixed SDKs. Split by class/service, transform each file, merge manually. |

## CLI (ready to use)

Build: `cargo build -p cloudshift-cli` — binary: `cloudshift`.

```bash
# Whole repo (directory)
cloudshift transform ./path/to/repo --source aws --output json --report migration.json

# Single file (same engine as UI — not a different “deep” refactor)
cloudshift transform ./src/aws_helpers.py --source aws

# Dry-run diff (default)
cloudshift transform ./repo --source aws

# Apply high-confidence changes to disk (use with care)
cloudshift transform ./repo --source aws --auto
```

Options that matter for repos: `--parallel`, `--include`, `--exclude`, `--report`, `--no-iac`, `--language`.

Optional: `--llm_fallback` (requires `ANTHROPIC_API_KEY`) for some remaining references—it does **not** replace splitting large monoliths.

## Will the CLI “fix” my 1500-line AWS file?

**No more than the UI.** One file = one pass through the same pipeline. For that scenario:

1. **Split** the file (e.g. one manager class per file), or  
2. Accept **draft** output and **edit** heavily, or  
3. Treat CloudShift as **assisted** migration, not full auto-rewrite of monoliths.

## Web UI limits (typical)

- Batch size and per-file size caps (rate limits, browser practicality).  
- Great for **exploration** and **moderate** multi-file runs.

For **client-scale repo migration**, document the **CLI + report** path as the supported workflow.
