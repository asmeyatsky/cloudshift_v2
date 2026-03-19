"""Azure Table Storage.

``TableServiceClient`` / ``TableClient`` — not HTTP handlers.

GCP loose analogue: ``gcp_reference/table_storage_firestore_analogue.py``.
"""


class TableStorageManager:
    """Manages Azure Table Storage operations"""

    def __init__(self, connection_string):
        from azure.data.tables import TableServiceClient
        self.table_service_client = TableServiceClient.from_connection_string(connection_string)

    def create_table(self, table_name):
        """Create a table"""
        try:
            table_client = self.table_service_client.create_table(table_name)
            print(f"Table {table_name} created successfully")
            return table_client
        except Exception as e:
            print(f"Error creating table: {e}")
            return None

    def create_entity(self, table_name, entity):
        """Create an entity in the table"""
        try:
            table_client = self.table_service_client.get_table_client(table_name)
            table_client.create_entity(entity=entity)
            print(f"Entity created in table {table_name}")
            return True
        except Exception as e:
            print(f"Error creating entity: {e}")
            return False

    def query_entities(self, table_name, filter_query=None):
        """Query entities from the table"""
        try:
            table_client = self.table_service_client.get_table_client(table_name)
            if filter_query:
                entities = table_client.query_entities(query_filter=filter_query)
            else:
                entities = table_client.list_entities()
            return list(entities)
        except Exception as e:
            print(f"Error querying entities: {e}")
            return []

    def update_entity(self, table_name, entity):
        """Update an entity in the table"""
        try:
            table_client = self.table_service_client.get_table_client(table_name)
            table_client.update_entity(entity=entity)
            print(f"Entity updated in table {table_name}")
            return True
        except Exception as e:
            print(f"Error updating entity: {e}")
            return False

    def delete_entity(self, table_name, partition_key, row_key):
        """Delete an entity from the table"""
        try:
            table_client = self.table_service_client.get_table_client(table_name)
            table_client.delete_entity(partition_key=partition_key, row_key=row_key)
            print(f"Entity deleted from table {table_name}")
            return True
        except Exception as e:
            print(f"Error deleting entity: {e}")
            return False
