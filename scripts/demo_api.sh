#!/usr/bin/env bash
# Demo: POST sample code to /api/transform and show that the app returns diffs and patterns.
#
# Usage:
#   ./scripts/demo_api.sh                                    # localhost:8080, no API key
#   BASE_URL=https://cloudshift-xxx.run.app ./scripts/demo_api.sh
#   API_KEY=your-secret ./scripts/demo_api.sh
#
# Requires: curl. For JSON encoding either jq or python3.

set -e

BASE_URL="${BASE_URL:-http://localhost:8080}"
API_KEY="${API_KEY:-}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FIXTURES="$REPO_ROOT/tests/patterns/python"

if [ ! -d "$FIXTURES" ]; then
  echo "Fixtures not found at $FIXTURES (run from repo root)."
  exit 1
fi

# Build JSON body: {"source": "<content>", "language": "python", "source_cloud": "aws", "path_hint": "main.py"}
build_payload() {
  local src="$1"
  if command -v jq &>/dev/null; then
    jq -n --rawfile src <(printf '%s' "$src") '{source: $src, language: "python", source_cloud: "aws", path_hint: "main.py"}'
  else
    printf '%s' "$src" | python3 -c "import json,sys; print(json.dumps({'source': sys.stdin.read(), 'language': 'python', 'source_cloud': 'aws', 'path_hint': 'main.py'}))"
  fi
}

CURL_OPTS=(-s -w "\n%{http_code}" -X POST "$BASE_URL/api/transform" -H "Content-Type: application/json")
[ -n "$API_KEY" ] && CURL_OPTS+=(-H "X-API-Key: $API_KEY")

run_one() {
  local name="$1"
  local source_file="$2"
  local src
  src=$(cat "$source_file")
  local payload
  payload=$(build_payload "$src")
  local tmp
  tmp=$(mktemp)
  echo "$payload" > "$tmp"
  local out
  out=$(curl "${CURL_OPTS[@]}" --data-binary "@$tmp" 2>/dev/null)
  rm -f "$tmp"
  local code
  code=$(echo "$out" | tail -n1)
  local body
  body=$(echo "$out" | sed '$d')
  if [ "$code" != "200" ]; then
    echo "[FAIL] $name — HTTP $code"
    echo "$body" | head -c 400
    echo ""
    return 1
  fi
  local pattern_count
  pattern_count=$(echo "$body" | grep -o '"pattern_id"' 2>/dev/null | wc -l | tr -d ' ')
  if [ "${pattern_count:-0}" -gt 0 ]; then
    echo "[PASS] $name — HTTP 200, $pattern_count pattern(s) matched"
  elif echo "$body" | grep -q '"diff"' && echo "$body" | grep -qE '"[^"]*diff[^"]*":"[^"]{20,}' 2>/dev/null; then
    echo "[PASS] $name — HTTP 200, diff present"
  else
    echo "[WARN] $name — HTTP 200 but no patterns/diff (catalogue empty or no match)"
  fi
  return 0
}

echo "Demo API: $BASE_URL"
echo "Samples: $FIXTURES"
echo ""

run_one "Python S3 (put_object, get_object, list_objects)" "$FIXTURES/aws_s3_to_gcs/before.py"
run_one "Python SQS (send_message, receive_message)" "$FIXTURES/aws_sqs_to_pubsub/before.py"
run_one "Python Secrets Manager (get_secret_value, create_secret)" "$FIXTURES/aws_secrets_to_secret_manager/before.py"
run_one "Python DynamoDB (put_item, get_item, query, ...)" "$FIXTURES/aws_dynamodb_to_firestore/before.py"

echo ""
echo "Done. [PASS] = patterns matched. [WARN] = no match (check CLOUDSHIFT_PATTERNS_DIR in deployment)."
