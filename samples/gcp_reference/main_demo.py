"""
GCP-oriented orchestrator — same *workflow shape* as
`aws_comprehensive_split/main_demo.py` (upload → document DB → notify → queue → metric).

Run from repo root:
  PYTHONPATH=samples/gcp_reference python samples/gcp_reference/main_demo.py

Env:
  GOOGLE_CLOUD_PROJECT  (required)
  GCS_BUCKET            (required for step 1)

Optional: GOOGLE_APPLICATION_CREDENTIALS or `gcloud auth application-default login`

Services not wired here (use dedicated products / gcp_reference modules):
  Lambda, EC2, RDS, API Gateway, Step Functions, IAM, Kinesis, ElastiCache,
  ECS, EKS, SES, Route53 — see README.md in this folder.

Requires:
  pip install google-cloud-storage google-cloud-firestore google-cloud-pubsub \\
              google-cloud-secret-manager google-cloud-logging google-api-core
"""
from __future__ import annotations

import json
import os
import uuid
from datetime import datetime

from google.api_core import exceptions as gcp_exceptions
import google.cloud.logging
from google.cloud import pubsub_v1
from google.cloud import secretmanager
from google.cloud import storage

from firestore_manager import FirestoreManager


def _project() -> str:
    p = os.environ.get("GOOGLE_CLOUD_PROJECT") or os.environ.get("GCP_PROJECT")
    if not p:
        raise SystemExit(
            "Set GOOGLE_CLOUD_PROJECT (or GCP_PROJECT) to run this demo."
        )
    return p


def _ensure_topic(publisher: pubsub_v1.PublisherClient, topic_path: str) -> None:
    try:
        publisher.get_topic(request={"topic": topic_path})
    except gcp_exceptions.NotFound:
        publisher.create_topic(request={"name": topic_path})


def main() -> None:
    project = _project()
    bucket_name = os.environ.get("GCS_BUCKET")
    if not bucket_name:
        raise SystemExit("Set GCS_BUCKET to a bucket in that project.")

    print("Starting GCP services integration example...")

    # 1. Object storage (S3 → GCS)
    data = json.dumps({"source": "demo", "ts": datetime.now().isoformat()})
    blob = storage.Client().bucket(bucket_name).blob("uploads/data.json")
    blob.upload_from_string(data, content_type="application/json")
    print(f"Uploaded gs://{bucket_name}/uploads/data.json")

    # 2. Firestore (DynamoDB analogue)
    fs = FirestoreManager(project_id=project)
    user_item = {
        "id": str(uuid.uuid4()),
        "name": "John Doe",
        "email": "john@example.com",
        "created_at": datetime.now().isoformat(),
    }
    fs.put_item("UserData", user_item)

    # 3. Pub/Sub topic publish (SNS-style fan-out)
    publisher = pubsub_v1.PublisherClient()
    notifications = publisher.topic_path(project, "user-notifications")
    _ensure_topic(publisher, notifications)
    publisher.publish(
        notifications,
        json.dumps(
            {"event": "user_created", "user_id": user_item["id"]}
        ).encode("utf-8"),
        type="notification",
    ).result()
    print("Published to Pub/Sub topic user-notifications")

    # 4. Second topic as queue-shaped work item (SQS analogue — pull via subscription)
    processing = publisher.topic_path(project, "processing-queue")
    _ensure_topic(publisher, processing)
    publisher.publish(
        processing,
        json.dumps(
            {"action": "process_user", "user_id": user_item["id"]}
        ).encode("utf-8"),
    ).result()
    print("Published to Pub/Sub topic processing-queue (add subscription + worker)")

    # 5. Structured log → Cloud Logging (lightweight stand-in for custom metric)
    log_client = google.cloud.logging.Client(project=project)
    log_client.logger("MyApp").log_struct(
        {
            "metric": "UsersCreated",
            "value": 1,
            "user_id": user_item["id"],
        },
        severity="INFO",
    )
    print("Wrote structured log (metric-style) to Cloud Logging")

    print("GCP services integration example completed!")


def extended_main() -> None:
    project = _project()
    print("Starting extended GCP demo (subset of AWS extended_main)...")

    # Secrets Manager (Secrets Manager sample)
    sm = secretmanager.SecretManagerServiceClient()
    parent = f"projects/{project}"
    secret_id = "db-password-demo"
    try:
        sm.create_secret(
            request={
                "parent": parent,
                "secret_id": secret_id,
                "secret": {"replication": {"automatic": {}}},
            }
        )
    except gcp_exceptions.AlreadyExists:
        pass
    sm.add_secret_version(
        request={
            "parent": f"{parent}/secrets/{secret_id}",
            "payload": {"data": b"my-secret-password"},
        }
    )
    print(f"Secret version added: {secret_id}")

    # Kinesis / ElastiCache / ECS / EKS / SES / Route53: use native GCP products
    # (Dataflow, Memorystore, Cloud Run, GKE, SendGrid/Mailgun or third-party, Cloud DNS).
    print(
        "Skipping Kinesis, ElastiCache, ECS, EKS, SES, Route53 — "
        "no single drop-in; see Cloud docs and other gcp_reference/*.py files."
    )

    print("Extended GCP demo completed!")


if __name__ == "__main__":
    main()
    extended_main()
