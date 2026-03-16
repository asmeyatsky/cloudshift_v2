#!/usr/bin/env bash
# Repoint cloudshift.poc-searce.com from Cloud Run v1 (us-central1) to v2 (europe-west1).
# Run from anywhere with gcloud auth to project emea-mas.
#
# What this does:
#   - Creates NEG cloudshift-neg-ew1 in europe-west1 for Cloud Run service "cloudshift" (v2).
#   - Removes the old us-central1 NEG from backend cloudshift-backend.
#   - Adds cloudshift-neg-ew1 to cloudshift-backend.
# After this, https://cloudshift.poc-searce.com/ hits v2.

set -e

PROJECT="${GCP_PROJECT_ID:-emea-mas}"
REGION_V2="${GCP_REGION_V2:-europe-west1}"
REGION_V1="us-central1"
CLOUD_RUN_SERVICE="cloudshift"
BACKEND_NAME="cloudshift-backend"
NEG_V1="cloudshift-neg"
NEG_V2="cloudshift-neg-ew1"

echo "Project: $PROJECT  (v1 NEG: $REGION_V1 / $NEG_V1  ->  v2 NEG: $REGION_V2 / $NEG_V2)"
echo ""

# --- 1. Create NEG in europe-west1 for v2 Cloud Run ---
if gcloud compute network-endpoint-groups describe "$NEG_V2" --region="$REGION_V2" --project="$PROJECT" &>/dev/null; then
  echo "[1/3] NEG $NEG_V2 already exists in $REGION_V2."
else
  echo "[1/3] Creating NEG $NEG_V2 in $REGION_V2 for Cloud Run $CLOUD_RUN_SERVICE..."
  gcloud compute network-endpoint-groups create "$NEG_V2" \
    --region="$REGION_V2" \
    --network-endpoint-type=serverless \
    --cloud-run-service="$CLOUD_RUN_SERVICE" \
    --project="$PROJECT"
  echo "      Created $NEG_V2."
fi

# --- 2. Remove old v1 NEG from backend (ignore error if not attached) ---
echo "[2/3] Removing v1 NEG ($NEG_V1 in $REGION_V1) from $BACKEND_NAME..."
if gcloud compute backend-services describe "$BACKEND_NAME" --global --project="$PROJECT" --format='value(backends[].group)' 2>/dev/null | grep -q "$NEG_V1"; then
  gcloud compute backend-services remove-backend "$BACKEND_NAME" \
    --global \
    --network-endpoint-group="$NEG_V1" \
    --network-endpoint-group-region="$REGION_V1" \
    --project="$PROJECT" \
    --quiet
  echo "      Removed $NEG_V1 from backend."
else
  echo "      $NEG_V1 was not attached; skipping."
fi

# --- 3. Add v2 NEG to backend ---
if gcloud compute backend-services describe "$BACKEND_NAME" --global --project="$PROJECT" --format='value(backends[].group)' 2>/dev/null | grep -q "$NEG_V2"; then
  echo "[3/3] NEG $NEG_V2 already attached to $BACKEND_NAME."
else
  echo "[3/3] Attaching $NEG_V2 to $BACKEND_NAME..."
  gcloud compute backend-services add-backend "$BACKEND_NAME" \
    --global \
    --network-endpoint-group="$NEG_V2" \
    --network-endpoint-group-region="$REGION_V2" \
    --project="$PROJECT"
  echo "      Attached $NEG_V2."
fi

echo ""
echo "Done. https://cloudshift.poc-searce.com/ now routes to Cloud Run v2 (europe-west1)."
