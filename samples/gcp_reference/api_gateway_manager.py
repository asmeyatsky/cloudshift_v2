"""
GCP analogue of AWS APIGatewayManager (REST API + resources + methods + deploy).

Important: **Google Cloud API Gateway** is **OpenAPI 2.0–driven**. You do not
imperatively create `/widgets` + `GET` like API Gateway REST; you define paths
and methods in the spec, then publish an **ApiConfig** and attach a **Gateway**.

Mapping (conceptual):
  create_rest_api          -> create_api (global API resource)
  create_resource + put_method -> paths/methods inside OpenAPI (+ x-google-backend)
  create_deployment / stage -> create_api_config + create_gateway (regional)

Requires: pip install google-cloud-apigateway google-api-core
"""
from __future__ import annotations

from google.api_core import exceptions as gcp_exceptions
from google.cloud import apigateway_v1 as agw


def _minimal_openapi_v2(
    title: str,
    base_path: str,
    path_part: str,
    http_method: str,
    backend_url: str,
) -> str:
    """Build a tiny OpenAPI 2 snippet (paths + backend)."""
    method_lower = http_method.lower()
    return f"""swagger: '2.0'
info:
  title: {title}
  version: '1.0.0'
schemes:
  - https
consumes:
  - application/json
produces:
  - application/json
paths:
  {base_path}/{path_part}:
    {method_lower}:
      operationId: op_{path_part.replace('/', '_')}
      x-google-backend:
        address: {backend_url}
        path_translation: APPEND_PATH_ADDRESS
      responses:
        '200':
          description: OK
"""


class CloudApiGatewayManager:
    """Google Cloud API Gateway — Api + ApiConfig + Gateway."""

    def __init__(self, project_id: str):
        self.project_id = project_id
        self._client = agw.ApiGatewayServiceClient()
        self._apis_parent = f"projects/{project_id}/locations/global"

    def create_rest_api(self, api_id: str, name: str, description: str = "") -> agw.Api | None:
        """Register an API container (like AWS create_rest_api)."""
        try:
            api = agw.Api(
                display_name=name,
                labels={"description": description[:63]} if description else {},
            )
            op = self._client.create_api(
                parent=self._apis_parent,
                api_id=api_id,
                api=api,
            )
            created = op.result()
            print(f"API {api_id} created: {created.name}")
            return created
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error creating API: {e}")
            return None

    def create_resource_and_method(
        self,
        path_part: str,
        http_method: str,
        backend_url: str,
        *,
        base_path: str = "",
    ) -> str:
        """
        AWS create_resource + put_method collapse into **OpenAPI paths**.
        This helper returns YAML you merge into a larger spec or pass to deploy.
        """
        return _minimal_openapi_v2(
            title="merged-spec",
            base_path=base_path.rstrip("/") or "",
            path_part=path_part.strip("/"),
            http_method=http_method,
            backend_url=backend_url.rstrip("/") + "/",
        )

    def deploy_api_stage(
        self,
        api_id: str,
        config_id: str,
        gateway_id: str,
        region: str,
        openapi_yaml: str,
        gateway_service_account: str,
    ) -> agw.Gateway | None:
        """
        Publish a revision (ApiConfig) and expose it on a regional Gateway
        (closest to create_deployment + stage).
        """
        api_name = f"{self._apis_parent}/apis/{api_id}"
        try:
            doc = agw.ApiConfig.OpenApiDocument(
                document=agw.ApiConfig.File(
                    path="openapi.yaml",
                    contents=openapi_yaml.encode("utf-8"),
                ),
            )
            cfg = agw.ApiConfig(
                openapi_documents=[doc],
                gateway_service_account=gateway_service_account,
            )
            op_cfg = self._client.create_api_config(
                parent=api_name,
                api_config_id=config_id,
                api_config=cfg,
            )
            op_cfg.result()

            gw_parent = f"projects/{self.project_id}/locations/{region}"
            gateway = agw.Gateway(
                display_name=gateway_id,
                api_config=f"{api_name}/configs/{config_id}",
            )
            op_gw = self._client.create_gateway(
                parent=gw_parent,
                gateway_id=gateway_id,
                gateway=gateway,
            )
            out = op_gw.result()
            print(f"Gateway {gateway_id} deployed in {region}")
            return out
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error deploying API config/gateway: {e}")
            return None
