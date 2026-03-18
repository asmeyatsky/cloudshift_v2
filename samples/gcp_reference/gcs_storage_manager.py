"""
GCS-native analogue of **S3Manager** (Cloud Storage client API only).

Broken transforms call `storage.Client().upload_file(Bucket=, Key=)` — that is
**S3/boto3**, not GCS. This module uses **bucket.blob(...).upload_from_filename**
and friends.

**Signed URLs** need credentials that can sign (typically a **service account
JSON key**), or use **IAM Credentials signBlob**; `generate_signed_url` may
fail with user ADC.

Requires: pip install google-cloud-storage google-api-core
"""
from __future__ import annotations

import os
from datetime import timedelta
from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud import storage


class GCSStorageManager:
    """Same responsibilities as the AWS S3 sample, using google.cloud.storage."""

    def __init__(self, bucket_name: str | None = None):
        self._client = storage.Client()
        self.bucket_name = bucket_name or os.environ.get(
            "GCS_BUCKET_NAME", os.environ.get("S3_BUCKET_NAME", "my-app-bucket")
        )

    def _bucket(self) -> storage.Bucket:
        return self._client.bucket(self.bucket_name)

    def upload_file(self, local_file_path: str, object_key: str) -> bool:
        try:
            blob = self._bucket().blob(object_key)
            blob.upload_from_filename(
                local_file_path, content_type="application/json"
            )
            print(f"Uploaded {local_file_path} -> gs://{self.bucket_name}/{object_key}")
            return True
        except gcp_exceptions.GoogleCloudError as e:
            print(f"Error uploading: {e}")
            return False

    def download_file(self, object_key: str, local_file_path: str) -> bool:
        try:
            self._bucket().blob(object_key).download_to_filename(local_file_path)
            print(f"Downloaded gs://{self.bucket_name}/{object_key} -> {local_file_path}")
            return True
        except gcp_exceptions.GoogleCloudError as e:
            print(f"Error downloading: {e}")
            return False

    def list_objects(self, prefix: str = "") -> list[dict[str, Any]]:
        """Return S3-shaped dicts (`Key`, `Size`) for compatibility."""
        try:
            out = []
            for b in self._client.list_blobs(self.bucket_name, prefix=prefix or None):
                out.append({"Key": b.name, "Size": b.size})
            return out
        except gcp_exceptions.GoogleCloudError as e:
            print(f"Error listing: {e}")
            return []

    def delete_object(self, object_key: str) -> bool:
        try:
            self._bucket().blob(object_key).delete()
            print(f"Deleted gs://{self.bucket_name}/{object_key}")
            return True
        except gcp_exceptions.GoogleCloudError as e:
            print(f"Error deleting: {e}")
            return False

    def generate_presigned_url(self, object_key: str, expiration: int = 3600) -> str | None:
        try:
            blob = self._bucket().blob(object_key)
            return blob.generate_signed_url(
                expiration=timedelta(seconds=expiration),
                method="GET",
                version="v4",
            )
        except Exception as e:
            print(
                f"Error generating signed URL (need signable credentials, e.g. SA key): {e}"
            )
            return None
