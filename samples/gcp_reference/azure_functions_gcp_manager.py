"""
GCP analogue for **Azure Functions** HTTP invoke + sample handler.

**Invoke:** Cloud Run / Cloud Functions (2nd gen) URLs often need an **ID token**
(``Authorization: Bearer``) when the service is not public. Optional API key
header if you configure one (unlike Azure ``x-functions-key`` by default).

**Handler:** Azure ``HttpRequest`` / ``HttpResponse`` → Flask ``request`` +
tuple ``(body, status)`` or ``functions_framework``.

See also ``lambda_cloud_run_manager.LambdaCloudRunManager`` for deploy + invoke
by service name.

Requires: ``pip install google-auth requests`` (and ``functions-framework`` to
run the example handler locally).
"""
from __future__ import annotations

from typing import Any

import google.auth.transport.requests
import google.oauth2.id_token

try:
    import functions_framework
except ImportError:
    functions_framework = None  # type: ignore[misc, assignment]


class AzureFunctionsGCPManager:
    """HTTP client to a Cloud Run / Gen2 function URL (Azure ``invoke_function`` shape)."""

    def __init__(
        self,
        base_url: str,
        *,
        audience: str | None = None,
        id_token: bool = True,
        api_key_header: tuple[str, str] | None = None,
    ) -> None:
        """
        ``base_url``: e.g. ``https://my-service-xxxxx-uc.a.run.app``
        ``audience``: OIDC audience (usually same as service URL).
        ``id_token``: set ``False`` for **public** (unauthenticated) Cloud Run URLs.
        ``api_key_header``: e.g. ``("X-Api-Key", "secret")`` if you front with API Gateway.
        """
        self.base_url = base_url.rstrip("/")
        self.audience = (audience or self.base_url).rstrip("/")
        self.id_token = id_token
        self.api_key_header = api_key_header

    def invoke_function(
        self,
        function_name: str,
        method: str = "POST",
        data: dict[str, Any] | None = None,
    ) -> Any | None:
        """Call ``{base_url}/api/{function_name}`` (same path style as Azure Functions)."""
        import requests

        url = f"{self.base_url}/api/{function_name.lstrip('/')}"
        headers: dict[str, str] = {"Content-Type": "application/json"}
        if self.api_key_header:
            headers[self.api_key_header[0]] = self.api_key_header[1]
        if self.id_token:
            try:
                auth_req = google.auth.transport.requests.Request()
                token = google.oauth2.id_token.fetch_id_token(
                    auth_req, self.audience
                )
                headers["Authorization"] = f"Bearer {token}"
            except Exception:
                pass  # public Cloud Run — use ``id_token=False`` to skip token attempt
        try:
            method_u = (method or "POST").upper()
            if method_u == "POST":
                response = requests.post(
                    url, json=data, headers=headers, timeout=120
                )
            else:
                response = requests.get(url, headers=headers, timeout=120)
            if response.status_code == 200:
                try:
                    return response.json()
                except Exception:
                    return response.text
            print(f"Function invocation failed: {response.status_code}")
            return None
        except Exception as e:
            print(f"Error invoking function: {e}")
            return None


def _example_handler_impl(request: Any) -> Any:
    name = request.args.get("name")
    if not name and getattr(request, "is_json", False):
        body = request.get_json(silent=True) or {}
        name = body.get("name")
    if name:
        return (
            f"Hello, {name}. This HTTP triggered function executed successfully.",
            200,
            {"Content-Type": "text/plain; charset=utf-8"},
        )
    return (
        "Please pass a name on the query string or in the request body",
        400,
        {"Content-Type": "text/plain; charset=utf-8"},
    )


if functions_framework:

    @functions_framework.http
    def example_http_function(request):
        """Same behaviour as Azure ``example_http_function`` (query/body ``name``)."""
        try:
            body, status, headers = _example_handler_impl(request)
            from flask import Response

            return Response(body, status=status, headers=headers)
        except Exception as e:
            from flask import Response

            return Response(f"Error: {e!s}", status=500)

else:

    def example_http_function(request):
        """Install ``functions-framework`` for the Cloud Functions–style entrypoint."""
        return _example_handler_impl(request)
