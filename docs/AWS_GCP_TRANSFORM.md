# AWS → GCP transformation: scope and guarantees

## What the pipeline does

1. **Pattern catalogue** (`patterns/`) — deterministic tree-sitter matches + templates.
2. **Import management** — adds/removes imports per pattern.
3. **Python fixups** — small text passes (S3 response shapes, exceptions, `s3://`→`gs://` when safe).
4. **LLM fallback** (optional, `--llm-fallback` + API key) — runs **only if** the detector still finds AWS/Azure references in the file after steps 1–3.

## What we do **not** guarantee

- **Complete migration of every AWS service** — the catalogue has **finite** patterns (~50+ Python AWS entries). Services without patterns rely on the LLM or manual work.
- **Correctness after LLM** — models can hallucinate, omit edge cases, or hit context limits. **Human review** is required for production.
- **Single pass on huge files** — very large multi-service modules often need **splitting** (see `samples/aws_comprehensive_split/` and `samples/gcp_reference/`).

## Safety rails (recent / ongoing)

| Risk | Mitigation |
|------|------------|
| Lambda pattern matching **instance methods** | Python `aws.lambda.handler` only matches `(event, context)` with second param **named `context`**, first not `self`/`cls`. |
| Azure Functions pattern on **managers / `__init__`** | `azure.functions.handler` matches only **single-arg** `def` (no top-level comma in params), not `self`/`cls`, not dunder names. |
| **ClientError** rewritten while still on boto3 | Generic `except ClientError` → GCP only when file has **no** boto3/botocore import. |
| **`client.exceptions.`** false positives | Removed blanket `client.exceptions.` replacement (was matching `dynamodb_client.exceptions`). |
| **`s3://` → `gs://`** while S3 still used | Rewritten only when the transformed file has **no** `boto3.client('s3')` / `resource('s3')` (DynamoDB-only files still get URI rewrite). |

## LLM fallback: when it runs

`needs_llm_fallback()` is true if **any** line matches heuristics: boto3 imports, `boto3.client`/`resource`, ARNs, `AWS_*` env vars, `.amazonaws.com`, etc. (see `llm_fallback/detector.rs`).

It does **not** mean the LLM will succeed on every service; it means “there is still AWS-shaped surface area to address.”

## Making new AWS services less likely to break

1. **Add a TOML pattern** under `patterns/python/` (or TS/Java) with a **narrow** detect query — avoid matching arbitrary `def foo(a, b)`.
2. **Run** `cargo test` and `cloudshift transform` on a fixture.
3. **Optional:** LLM learning can emit **candidate** patterns from a good LLM run (`cloudshift catalogue pending`).

## Reference implementations

For services without solid patterns, see **`samples/gcp_reference/`** (Firestore, Pub/Sub, Cloud SQL, Workflows, etc.) as **human-authored targets**, not auto-generated output.

## TypeScript / Java Lambda patterns

TS and Java Lambda handlers use **typed** queries (`Context`, `RequestHandler`) — they do **not** suffer the Python “two-parameter method” false positive.
