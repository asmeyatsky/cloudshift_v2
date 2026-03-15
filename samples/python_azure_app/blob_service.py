from azure.storage.blob import BlobServiceClient
from azure.identity import DefaultAzureCredential
from azure.keyvault.secrets import SecretClient
import json

credential = DefaultAzureCredential()

blob_service = BlobServiceClient(
    account_url="https://mystorageaccount.blob.core.windows.net",
    credential=credential
)
container_client = blob_service.get_container_client("documents")

vault_client = SecretClient(
    vault_url="https://myvault.vault.azure.net",
    credential=credential
)

def upload_document(name: str, content: bytes) -> str:
    """Upload a document to Azure Blob Storage."""
    blob_client = container_client.get_blob_client(name)
    blob_client.upload_blob(content, overwrite=True)
    return f"https://mystorageaccount.blob.core.windows.net/documents/{name}"

def download_document(name: str) -> bytes:
    """Download a document from Azure Blob Storage."""
    blob_client = container_client.get_blob_client(name)
    return blob_client.download_blob().readall()

def get_secret(secret_name: str) -> str:
    """Get a secret from Azure Key Vault."""
    secret = vault_client.get_secret(secret_name)
    return secret.value

def get_database_config() -> dict:
    """Get database configuration from Key Vault."""
    db_config = vault_client.get_secret("database-config")
    return json.loads(db_config.value)
