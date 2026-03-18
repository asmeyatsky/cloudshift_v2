"""Azure Container Instances."""
from azure.identity import DefaultAzureCredential


class ContainerInstancesManager:
    """Manages Azure Container Instances"""

    def __init__(self, subscription_id, resource_group_name):
        credential = DefaultAzureCredential()
        from azure.mgmt.containerinstance import ContainerInstanceManagementClient
        self.container_client = ContainerInstanceManagementClient(credential, subscription_id)
        self.resource_group_name = resource_group_name

    def create_container_group(self, container_group_name, location, image_name, cpu=1.0, memory=1.5):
        """Create a container group"""
        try:
            from azure.mgmt.containerinstance.models import Container, \
                ResourceRequirements, ResourceRequests

            container_resource_requests = ResourceRequests(
                memory_in_gb=memory,
                cpu=cpu
            )
            container_resource_requirements = ResourceRequirements(requests=container_resource_requests)

            container = Container(
                name=container_group_name,
                image=image_name,
                resources=container_resource_requirements,
                ports=[{'port': 80}]
            )

            from azure.mgmt.containerinstance.models import ContainerGroup
            container_group = ContainerGroup(
                location=location,
                containers=[container],
                os_type='Linux',
                restart_policy='Always'
            )

            cg = self.container_client.container_groups.begin_create_or_update(
                self.resource_group_name,
                container_group_name,
                container_group
            ).result()

            print(f"Container group {container_group_name} created")
            return cg
        except Exception as e:
            print(f"Error creating container group: {e}")
            return None

    def list_container_groups(self):
        """List all container groups"""
        try:
            container_groups = self.container_client.container_groups.list_by_resource_group(self.resource_group_name)
            return list(container_groups)
        except Exception as e:
            print(f"Error listing container groups: {e}")
            return []

    def delete_container_group(self, container_group_name):
        """Delete a container group"""
        try:
            self.container_client.container_groups.begin_delete(
                self.resource_group_name,
                container_group_name
            ).wait()
            print(f"Container group {container_group_name} deleted")
            return True
        except Exception as e:
            print(f"Error deleting container group: {e}")
            return False
