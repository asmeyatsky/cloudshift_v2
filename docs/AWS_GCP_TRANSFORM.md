# AWS → GCP transformation: scope and guarantees

## What the pipeline does

1. **Pattern catalogue** (`patterns/`) — deterministic tree-sitter matches + templates.
2. **Import management** — adds/removes imports per pattern.
3. **Python fixups** — small text passes (S3 response shapes, exceptions, `s3://`→`gs://` when safe).
4. **LLM fallback** (optional, `--llm-fallback` + API key) — runs **only if** the detector still finds AWS/Azure references in the file after steps 1–3.

## What we do **not** guarantee

- **Complete migration of every AWS service** — the catalogue has **finite** patterns (~50+ Python AWS entries). Services without patterns rely on the LLM or manual work. See **`docs/PATTERN_COVERAGE_GAPS.md`** (regenerate with `python3 scripts/report_pattern_gaps.py --write docs/PATTERN_COVERAGE_GAPS.md`).
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

## Client-initialization patterns (AWS / Azure gaps)

For boto3 services that had **no** call-level patterns (EC2, ECS, EKS, API Gateway, Route 53, ElastiCache, IAM, CloudWatch **Logs**, etc.), the catalogue includes **`aws_boto3_client_<service>.toml`** rules that replace `boto3.client('<service>')` with the closest **GCP client constructor**. Same idea for **`aws_boto3_resource_ec2`**.

Azure **mgmt** and Table/EventGrid/App Insights clients have analogous **`azure_*_client.toml`** entries.

These rewrites produce **valid GCP client imports** but **do not** rewrite individual RPC calls (`describe_instances`, `create_hosted_zone`, …). You still need follow-up patterns or manual/LLM migration for method bodies. **`azure_sql_management_client`** maps to `bigquery.Client()` only as a stub — prefer Cloud SQL (see pattern notes).

**How to add method-level patterns:** see **`docs/ADDING_PYTHON_PATTERNS.md`**.

## Making new AWS services less likely to break

1. **Add a TOML pattern** under `patterns/python/` (or TS/Java) with a **narrow** detect query — avoid matching arbitrary `def foo(a, b)`.
2. **Run** `cargo test` and `cloudshift transform` on a fixture.
3. **Optional:** LLM learning can emit **candidate** patterns from a good LLM run (`cloudshift catalogue pending`).

## Reference implementations

For services without solid patterns, see **`samples/gcp_reference/`** (Firestore, Pub/Sub, Cloud SQL, Workflows, etc.) as **human-authored targets**, not auto-generated output.

## TypeScript / Java Lambda patterns

TS and Java Lambda handlers use **typed** queries (`Context`, `RequestHandler`) — they do **not** suffer the Python “two-parameter method” false positive.
