"""
Lambda-shaped workflow on **Cloud Run (services)** — closest GCP analogue.

Zip uploads do **not** map to an API call; you **build a container image**
(Cloud Build) and pass **container_image**. `runtime` / `handler` apply to
how you build that image, not to Run’s API.

Invoke: HTTP **POST** to the service URL (with **ID token** if auth required).

Broken transforms (boto3 + functions_framework on class methods) are invalid.

Requires: pip install google-cloud-run google-auth requests google-api-core
"""
from __future__ import annotations

import json
import re
from typing import Any

import google.auth.transport.requests
import google.oauth2.id_token
import requests
from google.api_core import exceptions as gcp_exceptions
from google.cloud import run_v2


def _service_id(name: str) -> str:
    s = re.sub(r"[^a-z0-9-]", "-", name.lower()).strip("-")
    return (s or "fn")[:63]


class LambdaCloudRunManager:
    def __init__(self, project_id: str, region: str):
        self.project_id = project_id
        self.region = region
        self._parent = f"projects/{project_id}/locations/{region}"
        self._client = run_v2.ServicesClient()

    def create_function(
        self,
        function_name: str,
        runtime: str,
        handler: str,
        service_account: str,
        zip_file_path: str | None = None,
        *,
        container_image: str | None = None,
        memory_mb: int = 512,
        timeout_seconds: int = 300,
    ) -> run_v2.Service | None:
        """
        `zip_file_path` is ignored — use **container_image** (Artifact Registry
        or GCR). `runtime` / `handler` document your build; Run runs the
        container’s entrypoint.
        """
        _ = (runtime, handler, zip_file_path)
        image = container_image or ""
        if not image.strip():
            print("Provide container_image=... (built from your code / zip)")
            return None
        sid = _service_id(function_name)
        try:
            svc = run_v2.Service(
                ingress=run_v2.IngressTraffic.INGRESS_TRAFFIC_ALL,
                template=run_v2.RevisionTemplate(
                    service_account=service_account,
                    timeout=f"{timeout_seconds}s",
                    containers=[
                        run_v2.Container(
                            image=image.strip(),
                            ports=[run_v2.ContainerPort(container_port=8080)],
                            resources=run_v2.ResourceRequirements(
                                limits={
                                    "cpu": "1",
                                    "memory": f"{memory_mb}Mi",
                                }
                            ),
                        )
                    ],
                ),
            )
            op = self._client.create_service(
                parent=self._parent, service=svc, service_id=sid
            )
            out = op.result()
            print(f"Cloud Run service {sid} created")
            return out
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error creating Cloud Run service: {e}")
            return None

    def invoke_function(
        self, function_name: str, payload: dict[str, Any]
    ) -> dict[str, Any] | None:
        try:
            name = f"{self._parent}/services/{_service_id(function_name)}"
            svc = self._client.get_service(name=name)
            url = svc.uri
            if not url:
                print("Service has no URL yet")
                return None
            req = google.auth.transport.requests.Request()
            try:
                token = google.oauth2.id_token.fetch_id_token(req, url)
                headers = {"Authorization": f"Bearer {token}", "Content-Type": "application/json"}
            except Exception:
                headers = {"Content-Type": "application/json"}
            r = requests.post(
                url if url.endswith("/") else f"{url}/",
                data=json.dumps(payload),
                headers=headers,
                timeout=120,
            )
            r.raise_for_status()
            try:
                return r.json()
            except Exception:
                return {"raw": r.text}
        except Exception as e:
            print(f"Error invoking service: {e}")
            return None

    def update_function_code(
        self,
        function_name: str,
        zip_file_path: str | None = None,
        *,
        container_image: str | None = None,
    ) -> run_v2.Service | None:
        _ = zip_file_path
        if not container_image:
            print("Provide container_image for the new revision")
            return None
        name = f"{self._parent}/services/{_service_id(function_name)}"
        try:
            svc = self._client.get_service(name=name)
            tmpl = svc.template
            if tmpl.containers:
                c = tmpl.containers[0]
                c.image = container_image
            op = self._client.update_service(service=svc)
            out = op.result()
            print(f"Service {function_name} updated to {container_image}")
            return out
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error updating service: {e}")
            return None

    def list_functions(self) -> list[dict[str, Any]]:
        try:
            out = []
            for s in self._client.list_services(parent=self._parent):
                out.append(
                    {
                        "FunctionName": s.name.split("/")[-1],
                        "uri": s.uri,
                    }
                )
            return out
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error listing services: {e}")
            return []

    def delete_function(self, function_name: str) -> bool:
        try:
            name = f"{self._parent}/services/{_service_id(function_name)}"
            op = self._client.delete_service(name=name)
            op.result()
            print(f"Cloud Run service {function_name} deleted")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error deleting service: {e}")
            return False
