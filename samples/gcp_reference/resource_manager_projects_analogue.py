"""
GCP **Resource Manager (projects)** — loose analogue for **Azure resource groups**.

Azure **resource groups** are subscription-scoped containers. On GCP the usual
boundary is a **project** (org/folder hierarchy). Listing “groups” maps to
listing/searching projects; **creating** a project is a different workflow
than creating an RG (needs org parent, billing, etc.).

Requires: ``pip install google-cloud-resource-manager google-api-core``
"""
from __future__ import annotations

from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud import resourcemanager_v3
from google.cloud.resourcemanager_v3.types import Project


class ResourceManagerGCPAnalogue:
    """Shape similar to ``ResourceManager`` (list; create/delete are project-scoped)."""

    def __init__(self) -> None:
        self._client = resourcemanager_v3.ProjectsClient()

    def list_resource_groups(self) -> list[Any]:
        """List accessible projects (analogue of ``list_resource_groups``)."""
        try:
            return list(self._client.search_projects(request={}))
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error listing resource groups: {e}")
            return []

    def create_resource_group(
        self,
        project_id: str,
        display_name: str,
        *,
        parent: str,
    ) -> Any | None:
        """
        Create a **project** (not an Azure RG). ``parent`` must be like
        ``organizations/123456789`` or ``folders/123456789``.
        """
        try:
            op = self._client.create_project(
                project=Project(
                    project_id=project_id,
                    display_name=display_name,
                    parent=parent,
                )
            )
            return op.result()
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error creating resource group: {e}")
            return None

    def delete_resource_group(self, project_id_or_name: str) -> bool:
        """
        Delete a **project**. Pass ``projects/{project_id}`` or bare project id.
        Schedules deletion; unlike Azure RG delete.
        """
        try:
            name = project_id_or_name
            if not name.startswith("projects/"):
                name = f"projects/{name}"
            self._client.delete_project(name=name)
            print(f"Resource group {name} deletion scheduled")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error deleting resource group: {e}")
            return False
