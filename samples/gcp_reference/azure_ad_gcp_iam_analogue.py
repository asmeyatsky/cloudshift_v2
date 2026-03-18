"""
Partial GCP analogue for **Azure AD** patterns in the sample.

- **Service principal** ≈ **IAM service account** (project-scoped).
- **Azure AD users** (Graph) ≈ **Google Workspace / Cloud Identity** users — use
  **Admin SDK Directory API** or **Cloud Identity API**; not covered here.

Requires: ``pip install google-api-python-client google-auth google-api-core``
"""
from __future__ import annotations

from typing import Any

from google.auth import default as default_credentials
from googleapiclient.discovery import build


class AzureADGCPAnalogue:
    """Mirrors ``AzureADManager`` methods where a GCP API exists (service accounts)."""

    def __init__(self, project_id: str) -> None:
        self.project_id = project_id
        creds, _ = default_credentials(scopes=["https://www.googleapis.com/auth/cloud-platform"])
        self._iam = build("iam", "v1", credentials=creds, cache_discovery=False)

    def create_user(
        self,
        user_principal_name: str,
        display_name: str,
        password: str,
    ) -> None:
        """Not mapped. Use Workspace Admin SDK / Cloud Identity for human users."""
        _ = (user_principal_name, display_name, password)
        print(
            "GCP: create directory users via Workspace Admin SDK or Cloud Identity API, "
            "not IAM service accounts."
        )
        return None

    def list_users(self) -> list[Any]:
        """Lists **service accounts** (closest built-in analogue to directory principals)."""
        try:
            name = f"projects/{self.project_id}"
            out: list[Any] = []
            req = self._iam.projects().serviceAccounts().list(name=name)
            while req is not None:
                resp = req.execute()
                out.extend(resp.get("accounts", []))
                req = self._iam.projects().serviceAccounts().list_next(
                    previous_request=req, previous_response=resp
                )
            return out
        except Exception as e:
            print(f"Error listing users: {e}")
            return []

    def create_service_principal(self, app_id: str) -> Any | None:
        """
        Create a service account. ``app_id`` is used as **account_id** (e.g. ``my-bot``);
        must match ``^[a-z]([-a-z0-9]*[a-z0-9])?$`` and be 6–30 chars.
        """
        try:
            name = f"projects/{self.project_id}"
            body = {
                "accountId": app_id,
                "serviceAccount": {
                    "displayName": f"Service account {app_id}",
                },
            }
            return (
                self._iam.projects()
                .serviceAccounts()
                .create(name=name, body=body)
                .execute()
            )
        except Exception as e:
            print(f"Error creating service principal: {e}")
            return None
