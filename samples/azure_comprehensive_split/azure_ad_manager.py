"""Azure Active Directory — identity (Graph RBAC).

Uses legacy **azure-graphrbac** (consider Microsoft Graph for new code). Not HTTP
handlers — no ``functions_framework``.

GCP partial analogue: ``gcp_reference/azure_ad_gcp_iam_analogue.py`` (service
accounts); directory users → Workspace / Cloud Identity.
"""


class AzureADManager:
    """Manages Azure Active Directory operations"""

    def __init__(self, tenant_id, client_id, client_secret):
        from azure.identity import ClientSecretCredential
        from azure.graphrbac import GraphRbacManagementClient

        credential = ClientSecretCredential(
            tenant_id=tenant_id,
            client_id=client_id,
            client_secret=client_secret
        )
        self.graph_client = GraphRbacManagementClient(credential, tenant_id)

    def create_user(self, user_principal_name, display_name, password):
        """Create a user in Azure AD"""
        try:
            from azure.graphrbac.models import UserCreateParameters, PasswordProfile

            user_params = UserCreateParameters(
                user_principal_name=user_principal_name,
                display_name=display_name,
                mail_nickname=user_principal_name.split('@')[0],
                account_enabled=True,
                password_profile=PasswordProfile(
                    password=password,
                    force_change_password_next_login=False
                )
            )
            user = self.graph_client.users.create(user_params)
            print(f"User {display_name} created successfully")
            return user
        except Exception as e:
            print(f"Error creating user: {e}")
            return None

    def list_users(self):
        """List all users in Azure AD"""
        try:
            users = self.graph_client.users.list()
            return list(users)
        except Exception as e:
            print(f"Error listing users: {e}")
            return []

    def create_service_principal(self, app_id):
        """Create a service principal"""
        try:
            from azure.graphrbac.models import ServicePrincipalCreateParameters

            sp_params = ServicePrincipalCreateParameters(
                app_id=app_id,
                account_enabled=True
            )
            sp = self.graph_client.service_principals.create(sp_params)
            print(f"Service principal created for app {app_id}")
            return sp
        except Exception as e:
            print(f"Error creating service principal: {e}")
            return None
