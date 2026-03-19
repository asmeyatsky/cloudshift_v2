#!/usr/bin/env python3
"""
Report AWS/Azure API surfaces used in split samples vs patterns/python/*.toml.

Run from repo root:
  python3 scripts/report_pattern_gaps.py
  python3 scripts/report_pattern_gaps.py --write docs/PATTERN_COVERAGE_GAPS.md

Does not call the network. Heuristic: a boto3 service has coverage if some
pattern id/filename/body references that service (see SERVICE_ALIASES).
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
PATTERNS = REPO / "patterns" / "python"
AWS_SAMPLES = REPO / "samples" / "aws_comprehensive_split"
AZURE_SAMPLES = REPO / "samples" / "azure_comprehensive_split"

# boto3 service name -> True if any pattern targets that API surface
def aws_service_has_pattern(svc: str) -> bool:
    """Derived from patterns/python/aws_*.toml filenames."""
    s = svc.lower()
    stems = {f.stem for f in PATTERNS.glob("aws_*.toml")}
    if s == "s3":
        return any(x.startswith("aws_s3_") for x in stems)
    if s == "dynamodb":
        return any("dynamodb" in x for x in stems) or "aws_boto3_client_dynamodb" in stems
    if s == "sqs":
        return "aws_sqs_send_message" in stems or "aws_sqs_receive_message" in stems
    if s == "sns":
        return "aws_sns_publish" in stems or "aws_boto3_client_sns" in stems
    if s == "ses":
        return "aws_ses_send_email" in stems
    if s == "secretsmanager":
        return "aws_secrets_manager" in stems or "aws_boto3_client_secretsmanager" in stems
    if s == "cloudwatch":
        return "aws_cloudwatch_put_metric" in stems or "aws_cloudwatch_get_metric" in stems
    if s == "logs":
        return "aws_boto3_client_logs" in stems
    if s == "stepfunctions":
        return "aws_step_functions_start" in stems
    if s == "kinesis":
        return "aws_kinesis_put_record" in stems
    if s == "rds":
        return "aws_rds_connection" in stems
    if s == "lambda":
        return "aws_lambda_handler" in stems
    if s == "ec2":
        return "aws_boto3_client_ec2" in stems or "aws_boto3_resource_ec2" in stems
    if s == "apigateway":
        return "aws_boto3_client_apigateway" in stems
    if s == "route53":
        return "aws_boto3_client_route53" in stems
    if s == "eks":
        return "aws_boto3_client_eks" in stems
    if s == "ecs":
        return "aws_boto3_client_ecs" in stems
    if s == "elasticache":
        return "aws_boto3_client_elasticache" in stems
    if s == "iam":
        return "aws_boto3_client_iam" in stems
    # Monolith-only services with patterns (not in split sample but listed for completeness)
    extra = {
        "athena": "aws_athena_query",
        "bedrock": "aws_bedrock_invoke_model",
        "comprehend": "aws_comprehend_detect_sentiment",
        "ecr": "aws_ecr_get_auth",
        "eventbridge": "aws_eventbridge_put_events",
        "glue": "aws_glue_start_job",
        "polly": "aws_polly_synthesize",
        "rekognition": "aws_rekognition_detect_labels",
        "redshift": "aws_redshift_query",
        "sagemaker": "aws_sagemaker_create_endpoint",
        "textract": "aws_textract_detect",
        "translate": "aws_translate_text",
        "sts": "aws_sts_assume_role",
    }
    if s in extra:
        return extra[s] in stems
    return False

# Azure manager -> (sdk blurb, coverage: full | partial | none)
# partial = at least one call-shape pattern exists; many methods still untransformed.
AZURE_ROWS: list[tuple[str, str, str]] = [
    ("blob_storage_manager.py", "azure.storage.blob", "partial"),
    ("queue_storage_manager.py", "azure.storage.queue", "partial"),
    ("table_storage_manager.py", "azure.data.tables", "partial"),  # + client init pattern
    ("cosmos_db_manager.py", "azure.cosmos", "partial"),
    ("key_vault_manager.py", "azure.keyvault.secrets", "partial"),
    ("service_bus_manager.py", "azure.servicebus", "partial"),
    ("event_grid_manager.py", "azure.eventgrid", "partial"),  # publisher client
    ("sql_database_manager.py", "azure.mgmt.sql", "partial"),  # client init → review
    ("virtual_machine_manager.py", "azure.mgmt.compute", "partial"),
    ("container_instances_manager.py", "azure.mgmt.containerinstance", "partial"),
    ("app_service_manager.py", "azure.mgmt.web", "partial"),
    ("resource_manager.py", "azure.mgmt.resource", "partial"),
    ("azure_ad_manager.py", "azure.graphrbac", "partial"),
    ("azure_monitor_manager.py", "azure.monitor", "partial"),
    ("application_insights_manager.py", "applicationinsights", "partial"),
    (
        "cognitive_services_manager.py",
        "Text Analytics + Vision REST",
        "partial",  # sentiment pattern; vision/read not covered
    ),
    ("azure_functions_manager.py", "azure.functions HTTP", "partial"),  # handler shape
]


def boto_clients_in_samples() -> dict[str, list[str]]:
    """service -> [files]"""
    out: dict[str, list[str]] = {}
    pat = re.compile(r"""boto3\.(?:client|resource)\(\s*['"]([a-z0-9-]+)['"]""", re.I)
    for py in AWS_SAMPLES.glob("*.py"):
        text = py.read_text(encoding="utf-8")
        for m in pat.finditer(text):
            svc = m.group(1).lower()
            out.setdefault(svc, []).append(py.name)
    return out




def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--write",
        type=Path,
        help="Write markdown report to this path",
    )
    args = ap.parse_args()

    boto = boto_clients_in_samples()

    lines: list[str] = [
        "# Pattern coverage gaps (automated draft)",
        "",
        "Generated by `scripts/report_pattern_gaps.py`. Compares **split samples** to **patterns/python/*.toml** using heuristics — verify before prioritizing work.",
        "",
        "## AWS (`samples/aws_comprehensive_split`)",
        "",
        "| boto3 service | Example files | Pattern coverage (heuristic) |",
        "|---------------|---------------|------------------------------|",
    ]

    aws_gaps: list[str] = []
    for svc in sorted(boto.keys()):
        files = ", ".join(sorted(set(boto[svc])))
        ok = aws_service_has_pattern(svc)
        status = "Likely **yes**" if ok else "**GAP**"
        lines.append(f"| `{svc}` | {files} | {status} |")
        if not ok:
            aws_gaps.append(svc)

    lines += [
        "",
        f"**Summary:** {len(aws_gaps)} boto3 service(s) in split samples with no confident pattern match: "
        + ", ".join(f"`{s}`" for s in aws_gaps)
        if aws_gaps
        else "**Summary:** all sampled boto3 services matched at least one pattern heuristic.",
        "",
        "## Azure (`samples/azure_comprehensive_split` managers)",
        "",
        "| Module | SDK area | Heuristic pattern coverage |",
        "|--------|----------|---------------------------|",
    ]

    az_none: list[str] = []
    az_partial: list[str] = []
    for fname, area, cov in AZURE_ROWS:
        if cov == "none":
            status = "**none**"
            az_none.append(fname)
        elif cov == "partial":
            status = "**partial** (subset of methods)"
            az_partial.append(fname)
        else:
            status = cov
        lines.append(f"| `{fname}` | {area} | {status} |")

    lines += [
        "",
        "### Azure summary",
        "",
        f"- **No pattern** ({len(az_none)} modules): "
        + ", ".join(f"`{x}`" for x in az_none),
        "",
        f"- **Partial** ({len(az_partial)} modules): "
        + ", ".join(f"`{x}`" for x in az_partial),
        "",
        "## Next steps",
        "",
        "1. Add TOML patterns for each **GAP** / **none** row you need for automated output.",
        "2. Extend **partial** modules with one pattern per additional call shape.",
        "3. Add `tests/patterns/` fixtures; run `cargo test` and `cloudshift transform`.",
        "",
    ]

    report = "\n".join(lines) + "\n"
    if args.write:
        args.write.parent.mkdir(parents=True, exist_ok=True)
        args.write.write_text(report, encoding="utf-8")
        print(f"Wrote {args.write}", file=sys.stderr)
    print(report, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
