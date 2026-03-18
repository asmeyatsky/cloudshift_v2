"""Azure Key Vault — secrets.

``SecretClient`` + ``DefaultAzureCredential`` — not HTTP handlers. Do not use
``functions_framework`` on these methods.
"""
from azure.identity import DefaultAzureCredential
from azure.keyvault.secrets import SecretClient


class KeyVaultManager:
    """Manages Azure Key Vault secrets"""

    def __init__(self, vault_url):
        credential = DefaultAzureCredential()
        self.secret_client = SecretClient(vault_url=vault_url, credential=credential)

    def set_secret(self, secret_name, secret_value):
        """Set a secret in Key Vault"""
        try:
            secret = self.secret_client.set_secret(secret_name, secret_value)
            print(f"Secret {secret_name} set successfully")
            return secret
        except Exception as e:
            print(f"Error setting secret: {e}")
            return None

    def get_secret(self, secret_name):
        """Get a secret from Key Vault"""
        try:
            secret = self.secret_client.get_secret(secret_name)
            return secret.value
        except Exception as e:
            print(f"Error getting secret: {e}")
            return None

    def list_secrets(self):
        """List all secrets in Key Vault"""
        try:
            secrets = self.secret_client.list_properties_of_secrets()
            return list(secrets)
        except Exception as e:
            print(f"Error listing secrets: {e}")
            return []

    def delete_secret(self, secret_name):
        """Delete a secret from Key Vault"""
        try:
            poller = self.secret_client.begin_delete_secret(secret_name)
            deleted_secret = poller.result()
            print(f"Secret {secret_name} deleted successfully")
            return deleted_secret
        except Exception as e:
            print(f"Error deleting secret: {e}")
            return None
