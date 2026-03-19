"""Azure SQL Database — relational DB.

``SqlManagementClient`` — not HTTP handlers; no ``functions_framework``.
``create_database`` includes ``admin_login`` / ``admin_password`` for parity
with samples that provision servers separately.

GCP analogue: ``gcp_reference/cloud_sql_manager.py``.
"""
from azure.identity import DefaultAzureCredential
from azure.mgmt.sql import SqlManagementClient


class SQLDatabaseManager:
    """Manages Azure SQL Database"""

    def __init__(self, subscription_id, resource_group_name):
        credential = DefaultAzureCredential()
        self.sql_client = SqlManagementClient(credential, subscription_id)
        self.resource_group_name = resource_group_name

    def create_database(self, server_name, database_name, location, admin_login, admin_password):
        """Create an Azure SQL Database"""
        try:
            from azure.mgmt.sql.models import Database

            database_parameters = Database(
                location=location,
                create_mode='Default',
                requested_backup_storage_redundancy='Local'
            )

            async_db_creation = self.sql_client.databases.begin_create_or_update(
                self.resource_group_name,
                server_name,
                database_name,
                database_parameters
            )
            database_result = async_db_creation.result()
            print(f"SQL Database {database_name} created successfully")
            return database_result
        except Exception as e:
            print(f"Error creating SQL Database: {e}")
            return None

    def list_databases(self, server_name):
        """List all databases on a SQL server"""
        try:
            databases = self.sql_client.databases.list_by_server(
                self.resource_group_name,
                server_name
            )
            return list(databases)
        except Exception as e:
            print(f"Error listing databases: {e}")
            return []

    def delete_database(self, server_name, database_name):
        """Delete an Azure SQL Database"""
        try:
            async_db_delete = self.sql_client.databases.begin_delete(
                self.resource_group_name,
                server_name,
                database_name
            )
            async_db_delete.wait()
            print(f"SQL Database {database_name} deleted")
            return True
        except Exception as e:
            print(f"Error deleting SQL Database: {e}")
            return False
