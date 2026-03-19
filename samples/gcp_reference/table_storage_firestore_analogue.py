"""
Loose GCP analogue for **Azure Table Storage** using **Firestore** (native mode).

Azure **table** → Firestore **collection** (name must satisfy collection ID rules).
**PartitionKey** + **RowKey** → single document id ``{PartitionKey}__{RowKey}``.

Table Storage **OData filters** do not map 1:1; use Firestore queries where
possible. For wide-column / very high row counts, consider **Bigtable**.

Requires: ``pip install google-cloud-firestore google-api-core``
"""
from __future__ import annotations

import re
from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud import firestore


def _safe_collection_id(table_name: str) -> str:
    s = re.sub(r"[^a-zA-Z0-9_]", "_", table_name)
    return (s or "table")[:64]


def _doc_id(entity: dict[str, Any]) -> str:
    pk = str(entity.get("PartitionKey", ""))
    rk = str(entity.get("RowKey", ""))
    return f"{pk}__{rk}"


class TableStorageFirestoreAnalogue:
    """Similar method names to ``TableStorageManager`` (Firestore backend)."""

    def __init__(self, project_id: str | None = None) -> None:
        self._db = firestore.Client(project=project_id)

    def create_table(self, table_name: str) -> bool:
        """Firestore creates a collection on first write; this is a no-op success."""
        _ = table_name
        print(f"Table {table_name} ready (Firestore collection on first entity)")
        return True

    def create_entity(self, table_name: str, entity: dict[str, Any]) -> bool:
        try:
            col = _safe_collection_id(table_name)
            self._db.collection(col).document(_doc_id(entity)).set(entity)
            print(f"Entity created in table {table_name}")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error creating entity: {e}")
            return False

    def query_entities(
        self, table_name: str, filter_query: str | None = None
    ) -> list[dict[str, Any]]:
        try:
            col = _safe_collection_id(table_name)
            ref = self._db.collection(col)
            if filter_query:
                # OData → Firestore not implemented; list with warning
                print(
                    "Firestore: OData filter ignored; returning all docs in collection."
                )
            return [d.to_dict() for d in ref.stream() if d.to_dict()]
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error querying entities: {e}")
            return []

    def update_entity(self, table_name: str, entity: dict[str, Any]) -> bool:
        try:
            col = _safe_collection_id(table_name)
            self._db.collection(col).document(_doc_id(entity)).set(entity, merge=True)
            print(f"Entity updated in table {table_name}")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error updating entity: {e}")
            return False

    def delete_entity(
        self, table_name: str, partition_key: str, row_key: str
    ) -> bool:
        try:
            col = _safe_collection_id(table_name)
            did = _doc_id({"PartitionKey": partition_key, "RowKey": row_key})
            self._db.collection(col).document(did).delete()
            print(f"Entity deleted from table {table_name}")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error deleting entity: {e}")
            return False
