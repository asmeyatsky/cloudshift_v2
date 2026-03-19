# Adding Python AWS/Azure → GCP patterns

Use this when **client-init patterns** are not enough and you need to rewrite **specific API calls**.

## 1. Pick one call shape

Example: `self.ec2_client.terminate_instances(InstanceIds=[x])` — not `boto3.client('ec2')` (that’s already covered).

## 2. Write a tree-sitter query

Match a **call** whose function is `something.method_name`. Use **`object: (_)`** when the receiver is `self.ec2_client` (nested attribute), not a bare identifier:

```scheme
(call
  function: (attribute
    object: (_) @recv
    attribute: (identifier) @method (#eq? @method "terminate_instances"))
  arguments: (argument_list
    (keyword_argument
      name: (identifier) @param_name
      value: (_) @param_value)*) @args)
```

- Capture the whole `argument_list` as `@args` so bindings like `args.InstanceIds` work.
- Pre-filter with `imports = ["boto3"]` or **`["botocore"]`** if an earlier pattern already removed `import boto3` (e.g. after `aws_boto3_client_*` runs). Same for Route53 + other AWS managers that keep `ClientError` from botocore.

## 3. Template + bindings

- Placeholders `{var}` are filled from **bindings**.
- `args.FieldName` pulls the value of keyword argument `FieldName` from the captured `@args` node.

```toml
[pattern.bindings]
instance_ids = "args.InstanceIds"
```

- If a binding can’t be resolved, you get `/* unresolved: args.Foo */` in output — tighten the query or fix the arg name.

## 4. Create `patterns/python/your_pattern.toml`

Copy an existing file (e.g. `aws_cloudwatch_put_metric.toml`), change `id`, `query`, `template`, `bindings`, `import_add` / `import_remove`.

**Avoid** `import_remove = ["import boto3"]` on method-only patterns if the file still uses other boto3 calls — or the first match will strip `boto3`. Prefer removing only unused imports, or leave `import_remove` empty for follow-up patterns.

## 5. Verify

```bash
cargo test -p cloudshift-core
cargo run -p cloudshift-cli -- transform path/to/sample.py --source aws --dry-run
```

## 6. Optional fixture

Add `tests/patterns/python/<name>/before.py` + `after.py` + `meta.toml` if you want a golden diff (see existing folders).

## GCP mapping references

- `samples/gcp_reference/` — target-shaped Python.
- Google’s [AWS ↔ GCP product mapping](https://docs.cloud.google.com/docs/get-started/aws-azure-gcp-service-comparison).

## Regenerate gap report

```bash
python3 scripts/report_pattern_gaps.py --write docs/PATTERN_COVERAGE_GAPS.md
```
