"""Azure Cosmos DB — NoSQL."""
import uuid
from azure.cosmos import CosmosClient, PartitionKey, exceptions


class CosmosDBManager:
    """Manages Cosmos DB database and container operations"""

    def __init__(self, endpoint, key, database_name='MyDatabase'):
        self.cosmos_client = CosmosClient(endpoint, key)
        self.database_name = database_name
        self.database = self.cosmos_client.create_database_if_not_exists(id=database_name)

    def create_container(self, container_name, partition_key_path='/id'):
        """Create a Cosmos DB container"""
        try:
            container = self.database.create_container_if_not_exists(
                id=container_name,
                partition_key=PartitionKey(path=partition_key_path),
                offer_throughput=400
            )
            print(f"Container {container_name} created successfully")
            return container
        except exceptions.CosmosResourceExistsError:
            container = self.database.get_container_client(container_name)
            print(f"Container {container_name} already exists")
            return container
        except Exception as e:
            print(f"Error creating container: {e}")
            return None

    def create_item(self, container_name, item):
        """Create an item in Cosmos DB container"""
        try:
            container = self.database.get_container_client(container_name)
            if 'id' not in item:
                item['id'] = str(uuid.uuid4())
            created_item = container.create_item(body=item)
            print(f"Item created in container {container_name}")
            return created_item
        except Exception as e:
            print(f"Error creating item: {e}")
            return None

    def read_item(self, container_name, item_id, partition_key):
        """Read an item from Cosmos DB container"""
        try:
            container = self.database.get_container_client(container_name)
            item = container.read_item(item=item_id, partition_key=partition_key)
            return item
        except exceptions.CosmosResourceNotFoundError:
            print(f"Item {item_id} not found")
            return None
        except Exception as e:
            print(f"Error reading item: {e}")
            return None

    def query_items(self, container_name, query):
        """Query items from Cosmos DB container"""
        try:
            container = self.database.get_container_client(container_name)
            items = container.query_items(query=query, enable_cross_partition_query=True)
            return list(items)
        except Exception as e:
            print(f"Error querying items: {e}")
            return []

    def upsert_item(self, container_name, item):
        """Upsert an item in Cosmos DB container"""
        try:
            container = self.database.get_container_client(container_name)
            if 'id' not in item:
                item['id'] = str(uuid.uuid4())
            upserted_item = container.upsert_item(body=item)
            print(f"Item upserted in container {container_name}")
            return upserted_item
        except Exception as e:
            print(f"Error upserting item: {e}")
            return None

    def delete_item(self, container_name, item_id, partition_key):
        """Delete an item from Cosmos DB container"""
        try:
            container = self.database.get_container_client(container_name)
            container.delete_item(item=item_id, partition_key=partition_key)
            print(f"Item {item_id} deleted from container {container_name}")
            return True
        except Exception as e:
            print(f"Error deleting item: {e}")
            return False

    def replace_item(self, container_name, item_id, partition_key, updated_item):
        """Replace an item in Cosmos DB container"""
        try:
            container = self.database.get_container_client(container_name)
            updated_item['id'] = item_id
            replaced_item = container.replace_item(item=item_id, body=updated_item)
            return replaced_item
        except Exception as e:
            print(f"Error replacing item: {e}")
            return None
