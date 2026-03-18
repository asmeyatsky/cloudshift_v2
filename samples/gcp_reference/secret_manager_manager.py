"""
GCP **Secret Manager** — analogue for **Azure Key Vault** secrets API.

Azure maps:
  ``SecretClient(vault_url, DefaultAzureCredential)``  →  ``SecretManagerServiceClient`` + project id (ADC).

Requires: pip install google-cloud-secret-manager google-api-core
"""
from __future__ import annotations

from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud import secretmanager


class SecretManagerKeyVaultAnalogue:
    """Same responsibilities as ``KeyVaultManager`` (set/get/list/delete secret names)."""

    def __init__(self, project_id: str) -> None:
        self.project_id = project_id
        self._client = secretmanager.SecretManagerServiceClient()
        self._parent = f"projects/{project_id}"

    def _secret_path(self, secret_id: str) -> str:
        return f"{self._parent}/secrets/{secret_id}"

    def set_secret(self, secret_name: str, secret_value: str) -> Any | None:
        """Create secret if needed, then add a new version (like Key Vault set)."""
        try:
            try:
                self._client.create_secret(
                    request={
                        "parent": self._parent,
                        "secret_id": secret_name,
                        "secret": {"replication": {"automatic": {}}},
                    }
                )
            except gcp_exceptions.AlreadyExists:
                pass
            self._client.add_secret_version(
                request={
                    "parent": self._secret_path(secret_name),
                    "payload": {"data": secret_value.encode("utf-8")},
                }
            )
            print(f"Secret {secret_name} set successfully")
            return secret_name
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error setting secret: {e}")
            return None

    def get_secret(self, secret_name: str) -> str | None:
        try:
            name = f"{self._secret_path(secret_name)}/versions/latest"
            resp = self._client.access_secret_version(request={"name": name})
            return resp.payload.data.decode("utf-8")
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error getting secret: {e}")
            return None

    def list_secrets(self) -> list[Any]:
        try:
            return list(
                self._client.list_secrets(request={"parent": self._parent})
            )
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error listing secrets: {e}")
            return []

    def delete_secret(self, secret_name: str) -> Any | None:
        """Deletes the secret resource (all versions). Azure uses soft-delete by default."""
        try:
            self._client.delete_secret(request={"name": self._secret_path(secret_name)})
            print(f"Secret {secret_name} deleted successfully")
            return secret_name
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error deleting secret: {e}")
            return None
