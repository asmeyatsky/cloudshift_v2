"""Azure Resource Manager — resource groups."""
from azure.identity import DefaultAzureCredential
from azure.mgmt.resource import ResourceManagementClient


class ResourceManager:
    """Manages Azure resources"""

    def __init__(self, subscription_id):
        credential = DefaultAzureCredential()
        self.resource_client = ResourceManagementClient(credential, subscription_id)

    def create_resource_group(self, resource_group_name, location):
        """Create a resource group"""
        try:
            resource_group_params = {'location': location}
            resource_group = self.resource_client.resource_groups.create_or_update(
                resource_group_name,
                resource_group_params
            )
            print(f"Resource group {resource_group_name} created")
            return resource_group
        except Exception as e:
            print(f"Error creating resource group: {e}")
            return None

    def list_resource_groups(self):
        """List all resource groups"""
        try:
            resource_groups = self.resource_client.resource_groups.list()
            return list(resource_groups)
        except Exception as e:
            print(f"Error listing resource groups: {e}")
            return []

    def delete_resource_group(self, resource_group_name):
        """Delete a resource group and all its resources"""
        try:
            async_delete = self.resource_client.resource_groups.begin_delete(resource_group_name)
            async_delete.wait()
            print(f"Resource group {resource_group_name} deleted")
            return True
        except Exception as e:
            print(f"Error deleting resource group: {e}")
            return False
