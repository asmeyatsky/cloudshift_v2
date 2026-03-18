"""
RDS-shaped workflow on **Cloud SQL Admin API** (PostgreSQL / MySQL instances).

`db_instance_class` is mapped to a **tier** when it looks like AWS (e.g.
`db.t3.micro`); otherwise pass a Cloud SQL tier (`db-f1-micro`,
`db-custom-2-7680`, …).

`db_name`: initial DB is created via **databases.insert** after the instance
is RUNNABLE (async — this sample only starts creation).

Requires:
  pip install google-api-python-client google-auth-httplib2 google-auth
"""
from __future__ import annotations

from typing import Any

from google.auth import default as default_credentials
from googleapiclient.discovery import build
from googleapiclient.errors import HttpError


def _region_gcp(aws_region: str) -> str:
    ps = aws_region.split("-")
    if len(ps) == 3 and ps[2].isdigit():
        return f"{ps[0]}-{ps[1]}{ps[2]}"
    return aws_region


def _tier_from_class(db_instance_class: str, engine: str) -> str:
    c = db_instance_class.lower()
    if "custom" in c or c.startswith("db-"):
        return db_instance_class
    if "postgres" in engine.lower() or engine == "postgres":
        if "micro" in c or "small" in c:
            return "db-f1-micro"
        return "db-custom-1-3840"
    if "micro" in c:
        return "db-f1-micro"
    return "db-g1-small"


class CloudSQLManager:
    def __init__(self, project_id: str, region_name: str = "us-east-1"):
        self.project_id = project_id
        self.region = _region_gcp(region_name)
        creds, _ = default_credentials()
        self._sql = build("sqladmin", "v1", credentials=creds, cache_discovery=False)

    def create_database_instance(
        self,
        db_instance_identifier: str,
        db_name: str,
        master_username: str,
        master_password: str,
        db_instance_class: str,
        *,
        engine: str = "postgres",
        allocated_storage_gb: int = 20,
        backup_retention_days: int = 7,
        multi_az: bool = False,
        publicly_accessible: bool = False,
    ) -> dict[str, Any] | None:
        _ = master_username  # Cloud SQL default superuser is fixed per engine
        ver = "POSTGRES_15" if "postgres" in engine.lower() else "MYSQL_8_0"
        tier = _tier_from_class(db_instance_class, engine)
        body: dict[str, Any] = {
            "name": db_instance_identifier[:95],
            "databaseVersion": ver,
            "region": self.region,
            "rootPassword": master_password,
            "settings": {
                "tier": tier,
                "dataDiskSizeGb": str(allocated_storage_gb),
                "dataDiskType": "PD_SSD",
                "backupConfiguration": {
                    "enabled": True,
                    "backupRetentionSettings": {
                        "retainedBackups": backup_retention_days,
                    },
                },
                "ipConfiguration": {
                    "ipv4Enabled": publicly_accessible,
                    "requireSsl": False,
                },
                "availabilityType": "REGIONAL" if multi_az else "ZONAL",
            },
        }
        try:
            op = (
                self._sql.instances()
                .insert(project=self.project_id, body=body)
                .execute()
            )
            print(
                f"Cloud SQL instance {db_instance_identifier} creation started "
                f"(op {op.get('name', '')}); add DB {db_name!r} when RUNNABLE."
            )
            return op
        except HttpError as e:
            print(f"Error creating Cloud SQL instance: {e}")
            return None

    def describe_db_instances(self) -> list[dict[str, Any]]:
        try:
            resp = self._sql.instances().list(project=self.project_id).execute()
            return resp.get("items", [])
        except HttpError as e:
            print(f"Error listing Cloud SQL instances: {e}")
            return []

    def delete_db_instance(
        self, db_instance_identifier: str, skip_final_snapshot: bool = True
    ) -> bool:
        try:
            _ = skip_final_snapshot
            self._sql.instances().delete(
                project=self.project_id,
                instance=db_instance_identifier,
            ).execute()
            print(f"Cloud SQL instance {db_instance_identifier} delete requested")
            return True
        except HttpError as e:
            print(f"Error deleting Cloud SQL instance: {e}")
            return False
