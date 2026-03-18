"""
GCP analogue of Route53Manager — **Cloud DNS** (managed zones + record changes).

Route 53 hosted zone  -> Cloud DNS **managed zone**
change_resource_record_sets -> **ResourceRecordSet** + **Changes**

Broken transforms (boto3 + functions_framework) are invalid.

Requires: pip install google-cloud-dns google-api-core
"""
from __future__ import annotations

import re
import uuid
from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud import dns


def _dns_name(domain: str) -> str:
    d = domain.strip().rstrip(".")
    return f"{d}."


def _zone_resource_name(domain: str, caller_reference: str | None) -> str:
    base = re.sub(r"[^a-z0-9-]", "-", domain.lower().strip("."))[:50].strip("-") or "zone"
    suffix = (caller_reference or str(uuid.uuid4()))[:8]
    return f"{base}-{suffix}"[:63]


class CloudDnsManager:
    """Cloud DNS — similar workflow to the Route 53 sample."""

    def __init__(self, project_id: str):
        self._client = dns.Client(project=project_id)

    def _find_zone(self, hosted_zone_id: str) -> dns.zone.ManagedZone | None:
        """Resolve by managed zone name (returned from create_hosted_zone)."""
        for z in self._client.list_zones():
            if z.name == hosted_zone_id or z.dns_name.rstrip(".") == hosted_zone_id.rstrip(
                "."
            ):
                return z
        return None

    def create_hosted_zone(
        self, name: str, caller_reference: str | None = None
    ) -> dict[str, Any] | None:
        dns_name = _dns_name(name)
        zone_name = _zone_resource_name(name, caller_reference)
        try:
            zone = self._client.zone(zone_name, dns_name=dns_name)
            zone.create(description=f"caller_ref:{caller_reference or 'none'}")
            print(f"Managed zone {zone_name} for {dns_name}")
            return {
                "HostedZone": {
                    "Id": zone_name,
                    "Name": dns_name,
                    "ResourceRecordSetCount": 2,
                }
            }
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error creating managed zone: {e}")
            return None

    def create_record(
        self,
        hosted_zone_id: str,
        name: str,
        record_type: str,
        value: str,
        ttl: int = 300,
    ) -> dict[str, Any] | None:
        try:
            zone = self._find_zone(hosted_zone_id)
            if not zone:
                print(f"Managed zone {hosted_zone_id!r} not found")
                return None
            record_name = name if name.endswith(".") else f"{name}."
            rr = zone.resource_record_set(record_name, record_type, ttl, [value])
            changes = zone.changes()
            changes.add_record_set(rr)
            changes.create()
            print(f"DNS record {record_name} ({record_type}) added")
            return {"status": "done"}
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error creating DNS record: {e}")
            return None

    def list_hosted_zones(self) -> list[dict[str, Any]]:
        try:
            return [
                {"Id": z.name, "Name": z.dns_name}
                for z in self._client.list_zones()
            ]
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error listing zones: {e}")
            return []
