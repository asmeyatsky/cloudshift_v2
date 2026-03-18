"""Azure Blob Storage — object storage."""
import os
from azure.storage.blob import BlobServiceClient


class BlobStorageManager:
    """Manages Azure Blob Storage operations"""

    def __init__(self, connection_string=None, account_name=None, account_key=None):
        if connection_string:
            self.blob_service_client = BlobServiceClient.from_connection_string(connection_string)
        else:
            account_url = f"https://{account_name}.blob.core.windows.net"
            self.blob_service_client = BlobServiceClient(account_url, credential=account_key)
        self.container_name = os.environ.get('AZURE_STORAGE_CONTAINER', 'my-container')

    def create_container(self, container_name):
        """Create a blob container"""
        try:
            container_client = self.blob_service_client.create_container(container_name)
            print(f"Container {container_name} created successfully")
            return container_client
        except Exception as e:
            print(f"Error creating container: {e}")
            return None

    def upload_blob(self, blob_name, data, container_name=None):
        """Upload a blob to the container"""
        try:
            container = container_name or self.container_name
            blob_client = self.blob_service_client.get_blob_client(
                container=container,
                blob=blob_name
            )
            blob_client.upload_blob(data, overwrite=True)
            print(f"Blob {blob_name} uploaded successfully")
            return True
        except Exception as e:
            print(f"Error uploading blob: {e}")
            return False

    def download_blob(self, blob_name, container_name=None):
        """Download a blob from the container"""
        try:
            container = container_name or self.container_name
            blob_client = self.blob_service_client.get_blob_client(
                container=container,
                blob=blob_name
            )
            blob_data = blob_client.download_blob().readall()
            print(f"Blob {blob_name} downloaded successfully")
            return blob_data
        except Exception as e:
            print(f"Error downloading blob: {e}")
            return None

    def list_blobs(self, container_name=None, prefix=''):
        """List all blobs in the container"""
        try:
            container = container_name or self.container_name
            container_client = self.blob_service_client.get_container_client(container)
            blobs = container_client.list_blobs(name_starts_with=prefix)
            return list(blobs)
        except Exception as e:
            print(f"Error listing blobs: {e}")
            return []

    def delete_blob(self, blob_name, container_name=None):
        """Delete a blob from the container"""
        try:
            container = container_name or self.container_name
            blob_client = self.blob_service_client.get_blob_client(
                container=container,
                blob=blob_name
            )
            blob_client.delete_blob()
            print(f"Blob {blob_name} deleted successfully")
            return True
        except Exception as e:
            print(f"Error deleting blob: {e}")
            return False

    def generate_sas_url(self, blob_name, expiry_minutes=60, container_name=None):
        """Generate a SAS URL for temporary access"""
        try:
            from azure.storage.blob import generate_blob_sas, BlobSasPermissions
            from datetime import datetime, timedelta

            container = container_name or self.container_name
            account_name = self.blob_service_client.account_name
            account_key = self.blob_service_client.credential.account_key

            sas_token = generate_blob_sas(
                account_name=account_name,
                container_name=container,
                blob_name=blob_name,
                account_key=account_key,
                permission=BlobSasPermissions(read=True),
                expiry=datetime.utcnow() + timedelta(minutes=expiry_minutes)
            )

            blob_url = f"https://{account_name}.blob.core.windows.net/{container}/{blob_name}?{sas_token}"
            return blob_url
        except Exception as e:
            print(f"Error generating SAS URL: {e}")
            return None
