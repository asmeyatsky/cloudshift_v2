"""Azure Functions — serverless HTTP triggers.

``invoke_function`` is an HTTP **client** (any runtime). ``example_http_function``
uses **azure.functions** types inside Azure only — not Cloud Functions decorators.

GCP: ``gcp_reference/azure_functions_gcp_manager.py`` (invoke + sample handler).
"""
from azure.functions import HttpRequest, HttpResponse


class AzureFunctionsManager:
    """Manages Azure Functions (typically deployed via Azure Portal or CLI)"""

    def __init__(self, function_app_name, function_key=None):
        self.function_app_name = function_app_name
        self.function_key = function_key
        self.base_url = f"https://{function_app_name}.azurewebsites.net"

    def invoke_function(self, function_name, method='POST', data=None):
        """Invoke an Azure Function via HTTP"""
        import requests

        try:
            url = f"{self.base_url}/api/{function_name}"
            headers = {}
            if self.function_key:
                headers['x-functions-key'] = self.function_key

            if method == 'POST':
                response = requests.post(url, json=data, headers=headers)
            else:
                response = requests.get(url, headers=headers)

            if response.status_code == 200:
                return response.json()
            else:
                print(f"Function invocation failed: {response.status_code}")
                return None
        except Exception as e:
            print(f"Error invoking function: {e}")
            return None

    def example_http_function(self, req: HttpRequest) -> HttpResponse:
        """Example HTTP-triggered Azure Function"""
        try:
            name = req.params.get('name')
            if not name:
                try:
                    req_body = req.get_json()
                except ValueError:
                    pass
                else:
                    name = req_body.get('name')

            if name:
                return HttpResponse(
                    f"Hello, {name}. This HTTP triggered function executed successfully.",
                    status_code=200
                )
            else:
                return HttpResponse(
                    "Please pass a name on the query string or in the request body",
                    status_code=400
                )
        except Exception as e:
            return HttpResponse(f"Error: {str(e)}", status_code=500)
