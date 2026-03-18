"""
GCP analogue of DynamoDBManager — **Cloud Firestore** (Native mode).

Firestore has **no tables or create_table**; a **collection** exists when you
first write a document. Partition/sort keys map to **document IDs** and
optional subcollections or fields.

This is **not** what broken transforms produce (Firestore client + DynamoDB API
+ Cloud Functions decorators is invalid).

Requires: pip install google-cloud-firestore google-api-core
"""
from __future__ import annotations

import os
from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud import firestore


class FirestoreManager:
    """Document-store operations analogous to the DynamoDB sample."""

    def __init__(self, project_id: str | None = None):
        self._db = firestore.Client(project=project_id) if project_id else firestore.Client()
        self.default_collection = os.environ.get("FIRESTORE_COLLECTION", "UserData")

    def create_table(
        self,
        collection_name: str,
        partition_key_field: str,
        sort_key_field: str | None = None,
    ) -> bool:
        """
        DynamoDB create_table → Firestore: **no-op / convention only**.
        Store key layout in app config; first `put_item` creates the collection.
        """
        _ = (collection_name, partition_key_field, sort_key_field)
        print(
            f"Firestore: collection {collection_name!r} will appear on first write; "
            f"partition field {partition_key_field!r}"
            + (f", sort field {sort_key_field!r}" if sort_key_field else "")
        )
        return True

    def put_item(self, collection_name: str, item: dict[str, Any]) -> dict[str, Any] | None:
        """DynamoDB put_item → set document (use stable doc id from partition key)."""
        try:
            doc_id = str(item.get("id") or item.get("pk") or item.get("user_id", ""))
            if not doc_id:
                doc_ref = self._db.collection(collection_name).document()
            else:
                doc_ref = self._db.collection(collection_name).document(doc_id)
            doc_ref.set(item)
            print(f"Document written to {collection_name}/{doc_ref.id}")
            return {"DocumentId": doc_ref.id}
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error putting document: {e}")
            return None

    def get_item(self, collection_name: str, key: dict[str, Any]) -> dict[str, Any] | None:
        """key like {'id': '...'} or DynamoDB-style single hash key."""
        try:
            doc_id = next(iter(key.values()))
            snap = self._db.collection(collection_name).document(str(doc_id)).get()
            if snap.exists:
                return dict(snap.to_dict() or {})
            return None
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error getting document: {e}")
            return None

    def query_items(
        self,
        collection_name: str,
        partition_key_value: Any,
        *,
        field_name: str = "id",
        index_name: str | None = None,
    ) -> list[dict[str, Any]]:
        """
        Simple equality query. `index_name` is ignored unless you map it to
        another field (Firestore composite indexes are declared separately).
        """
        _ = index_name
        try:
            q = self._db.collection(collection_name).where(
                field_name, "==", partition_key_value
            )
            return [dict(d.to_dict() or {}) | {"_id": d.id} for d in q.stream()]
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error querying: {e}")
            return []

    def scan_table(
        self,
        collection_name: str,
        *,
        limit: int | None = None,
    ) -> list[dict[str, Any]]:
        """Full collection read (like scan); use pagination in production."""
        try:
            ref = self._db.collection(collection_name)
            docs = ref.limit(limit).stream() if limit else ref.stream()
            return [dict(d.to_dict() or {}) | {"_id": d.id} for d in docs]
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error scanning: {e}")
            return []

    def update_item(
        self,
        collection_name: str,
        key: dict[str, Any],
        updates: dict[str, Any],
    ) -> dict[str, Any] | None:
        """Pass field updates as a flat dict (not DynamoDB UpdateExpression)."""
        try:
            doc_id = str(next(iter(key.values())))
            self._db.collection(collection_name).document(doc_id).update(updates)
            return {"updated": True, "id": doc_id}
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error updating document: {e}")
            return None

    def delete_item(self, collection_name: str, key: dict[str, Any]) -> bool:
        try:
            doc_id = str(next(iter(key.values())))
            self._db.collection(collection_name).document(doc_id).delete()
            print(f"Document deleted from {collection_name}")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error deleting document: {e}")
            return False
