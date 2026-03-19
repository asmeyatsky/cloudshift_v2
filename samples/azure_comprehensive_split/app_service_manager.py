"""Azure App Service — web apps.

``WebSiteManagementClient`` — not HTTP handlers; no ``functions_framework``.
Zip/one-deploy differs from GCP’s container model.

GCP analogue: **Cloud Run** (build a container from your app), e.g.
``gcp_reference/lambda_cloud_run_manager.py``.
"""
from azure.identity import DefaultAzureCredential


class AppServiceManager:
    """Manages Azure App Service web apps"""

    def __init__(self, subscription_id, resource_group_name):
        credential = DefaultAzureCredential()
        from azure.mgmt.web import WebSiteManagementClient
        self.web_client = WebSiteManagementClient(credential, subscription_id)
        self.resource_group_name = resource_group_name

    def create_web_app(self, app_name, location, app_service_plan_id):
        """Create a web app"""
        try:
            from azure.mgmt.web.models import Site, SiteConfig

            site_config = SiteConfig(python_version='3.11')
            site_envelope = Site(
                location=location,
                server_farm_id=app_service_plan_id,
                site_config=site_config
            )

            web_app = self.web_client.web_apps.begin_create_or_update(
                self.resource_group_name,
                app_name,
                site_envelope
            ).result()

            print(f"Web app {app_name} created successfully")
            return web_app
        except Exception as e:
            print(f"Error creating web app: {e}")
            return None

    def deploy_app(self, app_name, package_path):
        """Deploy application to web app"""
        try:
            with open(package_path, 'rb') as f:
                self.web_client.web_apps.begin_create_one_deploy_slot(
                    self.resource_group_name,
                    app_name,
                    'production',
                    {'package': f.read()}
                ).result()
            print(f"Application deployed to {app_name}")
            return True
        except Exception as e:
            print(f"Error deploying app: {e}")
            return False

    def list_web_apps(self):
        """List all web apps"""
        try:
            web_apps = self.web_client.web_apps.list_by_resource_group(self.resource_group_name)
            return list(web_apps)
        except Exception as e:
            print(f"Error listing web apps: {e}")
            return []
