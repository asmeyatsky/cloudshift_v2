"""
Comprehensive Azure Example Code
Demonstrates usage of multiple Azure managed services
This file contains ~1000 lines of Azure code for testing refactoring to GCP

For per-service transforms, use:
  samples/azure_comprehensive_split/
See azure_comprehensive_split/README.md
"""

from azure.storage.blob import BlobServiceClient, BlobClient, ContainerClient
from azure.cosmos import CosmosClient, PartitionKey, exceptions
from azure.functions import HttpRequest, HttpResponse
from azure.servicebus import ServiceBusClient, ServiceBusMessage
from azure.eventgrid import EventGridPublisherClient, EventGridEvent
from azure.compute import ComputeManagementClient
from azure.mgmt.sql import SqlManagementClient
from azure.keyvault.secrets import SecretClient
from azure.identity import DefaultAzureCredential
from azure.monitor import MonitorClient
from azure.mgmt.resource import ResourceManagementClient
from azure.mgmt.network import NetworkManagementClient
import os
import json
import uuid
from datetime import datetime, timedelta

# ============================================================================
# Blob Storage - Object Storage Service
# ============================================================================

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

# ============================================================================
# Cosmos DB - NoSQL Database
# ============================================================================

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

# ============================================================================
# Azure Functions - Serverless Functions
# ============================================================================

class AzureFunctionsManager:
    """Manages Azure Functions (typically deployed via Azure Portal or CLI)"""
    
    def __init__(self, function_app_name, function_key=None):
        self.function_app_name = function_app_name
        self.function_key = function_key
        self.base_url = f"https://{function_app_name}.azurewebsites.net"
    
    def invoke_function(self, function_name, method='POST', data=None):
        """Invoke an Azure Function via HTTP"""
        import requests
        
        try:
            url = f"{self.base_url}/api/{function_name}"
            headers = {}
            if self.function_key:
                headers['x-functions-key'] = self.function_key
            
            if method == 'POST':
                response = requests.post(url, json=data, headers=headers)
            else:
                response = requests.get(url, headers=headers)
            
            if response.status_code == 200:
                return response.json()
            else:
                print(f"Function invocation failed: {response.status_code}")
                return None
        except Exception as e:
            print(f"Error invoking function: {e}")
            return None
    
    def example_http_function(self, req: HttpRequest) -> HttpResponse:
        """Example HTTP-triggered Azure Function"""
        try:
            name = req.params.get('name')
            if not name:
                try:
                    req_body = req.get_json()
                except ValueError:
                    pass
                else:
                    name = req_body.get('name')
            
            if name:
                return HttpResponse(
                    f"Hello, {name}. This HTTP triggered function executed successfully.",
                    status_code=200
                )
            else:
                return HttpResponse(
                    "Please pass a name on the query string or in the request body",
                    status_code=400
                )
        except Exception as e:
            return HttpResponse(f"Error: {str(e)}", status_code=500)

# ============================================================================
# Service Bus - Messaging Service
# ============================================================================

class ServiceBusManager:
    """Manages Azure Service Bus queues and topics"""
    
    def __init__(self, connection_string):
        self.servicebus_client = ServiceBusClient.from_connection_string(connection_string)
    
    def send_queue_message(self, queue_name, message_body, properties=None):
        """Send a message to a Service Bus queue"""
        try:
            with self.servicebus_client:
                sender = self.servicebus_client.get_queue_sender(queue_name=queue_name)
                message = ServiceBusMessage(message_body)
                if properties:
                    for key, value in properties.items():
                        message.properties = {**message.properties, key: value}
                sender.send_messages(message)
                print(f"Message sent to queue {queue_name}")
                return True
        except Exception as e:
            print(f"Error sending message: {e}")
            return False
    
    def receive_queue_messages(self, queue_name, max_messages=1):
        """Receive messages from a Service Bus queue"""
        try:
            messages = []
            with self.servicebus_client:
                receiver = self.servicebus_client.get_queue_receiver(queue_name=queue_name)
                received_messages = receiver.receive_messages(max_messages=max_messages, max_wait_time=5)
                for msg in received_messages:
                    messages.append({
                        'body': str(msg),
                        'properties': dict(msg.properties) if msg.properties else {}
                    })
                    receiver.complete_message(msg)
            return messages
        except Exception as e:
            print(f"Error receiving messages: {e}")
            return []
    
    def create_topic(self, topic_name):
        """Create a Service Bus topic (requires management client)"""
        try:
            from azure.mgmt.servicebus import ServiceBusManagementClient
            # Note: This requires proper Azure credentials and resource group
            print(f"Topic {topic_name} creation initiated")
            return True
        except Exception as e:
            print(f"Error creating topic: {e}")
            return False
    
    def send_topic_message(self, topic_name, message_body, properties=None):
        """Send a message to a Service Bus topic"""
        try:
            with self.servicebus_client:
                sender = self.servicebus_client.get_topic_sender(topic_name=topic_name)
                message = ServiceBusMessage(message_body)
                if properties:
                    for key, value in properties.items():
                        message.properties = {**message.properties, key: value}
                sender.send_messages(message)
                print(f"Message sent to topic {topic_name}")
                return True
        except Exception as e:
            print(f"Error sending topic message: {e}")
            return False
    
    def create_subscription(self, topic_name, subscription_name):
        """Create a subscription to a Service Bus topic"""
        try:
            # Note: This typically requires Azure Portal or management client
            print(f"Subscription {subscription_name} created for topic {topic_name}")
            return True
        except Exception as e:
            print(f"Error creating subscription: {e}")
            return False

# ============================================================================
# Event Grid - Event Routing Service
# ============================================================================

class EventGridManager:
    """Manages Azure Event Grid events"""
    
    def __init__(self, topic_endpoint, topic_key):
        self.event_grid_client = EventGridPublisherClient(
            topic_endpoint,
            credential=topic_key
        )
    
    def publish_event(self, event_type, subject, data):
        """Publish an event to Event Grid"""
        try:
            event = EventGridEvent(
                subject=subject,
                data=data,
                event_type=event_type,
                data_version="1.0"
            )
            self.event_grid_client.send(event)
            print(f"Event {event_type} published successfully")
            return True
        except Exception as e:
            print(f"Error publishing event: {e}")
            return False
    
    def publish_events_batch(self, events):
        """Publish multiple events to Event Grid"""
        try:
            self.event_grid_client.send(events)
            print(f"{len(events)} events published successfully")
            return True
        except Exception as e:
            print(f"Error publishing events batch: {e}")
            return False

# ============================================================================
# Virtual Machines - Compute Service
# ============================================================================

class VirtualMachineManager:
    """Manages Azure Virtual Machines"""
    
    def __init__(self, subscription_id, resource_group_name):
        credential = DefaultAzureCredential()
        self.compute_client = ComputeManagementClient(credential, subscription_id)
        self.resource_group_name = resource_group_name
    
    def create_vm(self, vm_name, location, vm_size, admin_username, admin_password, image_reference):
        """Create a virtual machine"""
        try:
            from azure.mgmt.compute.models import VirtualMachine, NetworkProfile, NetworkInterfaceReference, \
                StorageProfile, ImageReference, OSDisk, ManagedDiskParameters, HardwareProfile
            
            vm_parameters = VirtualMachine(
                location=location,
                hardware_profile=HardwareProfile(vm_size=vm_size),
                storage_profile=StorageProfile(
                    image_reference=ImageReference(
                        publisher=image_reference['publisher'],
                        offer=image_reference['offer'],
                        sku=image_reference['sku'],
                        version=image_reference['version']
                    ),
                    os_disk=OSDisk(
                        create_option='FromImage',
                        managed_disk=ManagedDiskParameters(storage_account_type='Premium_LRS')
                    )
                ),
                os_profile={
                    'computer_name': vm_name,
                    'admin_username': admin_username,
                    'admin_password': admin_password
                }
            )
            
            async_vm_creation = self.compute_client.virtual_machines.begin_create_or_update(
                self.resource_group_name,
                vm_name,
                vm_parameters
            )
            vm_result = async_vm_creation.result()
            print(f"Virtual machine {vm_name} created successfully")
            return vm_result
        except Exception as e:
            print(f"Error creating virtual machine: {e}")
            return None
    
    def list_vms(self):
        """List all virtual machines in resource group"""
        try:
            vms = self.compute_client.virtual_machines.list(self.resource_group_name)
            return list(vms)
        except Exception as e:
            print(f"Error listing virtual machines: {e}")
            return []
    
    def start_vm(self, vm_name):
        """Start a virtual machine"""
        try:
            async_vm_start = self.compute_client.virtual_machines.begin_start(
                self.resource_group_name,
                vm_name
            )
            async_vm_start.wait()
            print(f"Virtual machine {vm_name} started")
            return True
        except Exception as e:
            print(f"Error starting virtual machine: {e}")
            return False
    
    def stop_vm(self, vm_name):
        """Stop a virtual machine"""
        try:
            async_vm_stop = self.compute_client.virtual_machines.begin_power_off(
                self.resource_group_name,
                vm_name
            )
            async_vm_stop.wait()
            print(f"Virtual machine {vm_name} stopped")
            return True
        except Exception as e:
            print(f"Error stopping virtual machine: {e}")
            return False
    
    def delete_vm(self, vm_name):
        """Delete a virtual machine"""
        try:
            async_vm_delete = self.compute_client.virtual_machines.begin_delete(
                self.resource_group_name,
                vm_name
            )
            async_vm_delete.wait()
            print(f"Virtual machine {vm_name} deleted")
            return True
        except Exception as e:
            print(f"Error deleting virtual machine: {e}")
            return False

# ============================================================================
# SQL Database - Relational Database Service
# ============================================================================

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

# ============================================================================
# Key Vault - Secrets Management
# ============================================================================

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

# ============================================================================
# Application Insights - Monitoring and Logging
# ============================================================================

class ApplicationInsightsManager:
    """Manages Azure Application Insights telemetry"""
    
    def __init__(self, instrumentation_key):
        from applicationinsights import TelemetryClient
        self.telemetry_client = TelemetryClient(instrumentation_key)
    
    def track_event(self, event_name, properties=None):
        """Track a custom event"""
        try:
            self.telemetry_client.track_event(event_name, properties)
            self.telemetry_client.flush()
            print(f"Event {event_name} tracked")
            return True
        except Exception as e:
            print(f"Error tracking event: {e}")
            return False
    
    def track_exception(self, exception, properties=None):
        """Track an exception"""
        try:
            self.telemetry_client.track_exception(exception, properties)
            self.telemetry_client.flush()
            print("Exception tracked")
            return True
        except Exception as e:
            print(f"Error tracking exception: {e}")
            return False
    
    def track_metric(self, metric_name, value, properties=None):
        """Track a custom metric"""
        try:
            self.telemetry_client.track_metric(metric_name, value, properties)
            self.telemetry_client.flush()
            print(f"Metric {metric_name} tracked")
            return True
        except Exception as e:
            print(f"Error tracking metric: {e}")
            return False
    
    def track_trace(self, message, severity_level='Information', properties=None):
        """Track a trace message"""
        try:
            self.telemetry_client.track_trace(message, severity_level, properties)
            self.telemetry_client.flush()
            print(f"Trace message tracked: {message}")
            return True
        except Exception as e:
            print(f"Error tracking trace: {e}")
            return False

# ============================================================================
# Resource Manager - Resource Management
# ============================================================================

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

# ============================================================================
# Main Application Example
# ============================================================================

def main():
    """Main application demonstrating Azure services integration"""
    
    # Initialize managers (with placeholder credentials)
    blob_manager = BlobStorageManager(
        account_name=os.environ.get('AZURE_STORAGE_ACCOUNT'),
        account_key=os.environ.get('AZURE_STORAGE_KEY')
    )
    
    cosmos_manager = CosmosDBManager(
        endpoint=os.environ.get('COSMOS_ENDPOINT', 'https://localhost:8081'),
        key=os.environ.get('COSMOS_KEY', 'C2y6yDjf5/R+ob0N8A7Cgv30VRDJIWEHLM+4QDU5DE2nQ9nDuTqfmPtuXlZg=='),
        database_name='MyDatabase'
    )
    
    servicebus_manager = ServiceBusManager(
        connection_string=os.environ.get('SERVICE_BUS_CONNECTION_STRING', '')
    )
    
    event_grid_manager = EventGridManager(
        topic_endpoint=os.environ.get('EVENT_GRID_ENDPOINT', ''),
        topic_key=os.environ.get('EVENT_GRID_KEY', '')
    )
    
    print("Starting Azure services integration example...")
    
    # 1. Upload data to Blob Storage
    blob_manager.upload_blob('data.json', json.dumps({'key': 'value'}))
    
    # 2. Store metadata in Cosmos DB
    user_item = {
        'id': str(uuid.uuid4()),
        'name': 'Jane Doe',
        'email': 'jane@example.com',
        'created_at': datetime.now().isoformat()
    }
    cosmos_manager.create_item('Users', user_item)
    
    # 3. Send message via Service Bus
    servicebus_manager.send_queue_message('user-queue', json.dumps({'event': 'user_created', 'user_id': user_item['id']}))
    
    # 4. Publish event via Event Grid
    event_grid_manager.publish_event(
        event_type='User.Created',
        subject=f'/users/{user_item["id"]}',
        data={'userId': user_item['id'], 'email': user_item['email']}
    )
    
    print("Azure services integration example completed!")

# ============================================================================
# Additional Azure Services Examples
# ============================================================================

# ============================================================================
# Azure Active Directory - Identity Management
# ============================================================================

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

# ============================================================================
# Azure Storage Queue - Queue Storage Service
# ============================================================================

class QueueStorageManager:
    """Manages Azure Storage Queue operations"""
    
    def __init__(self, connection_string):
        from azure.storage.queue import QueueServiceClient
        self.queue_service_client = QueueServiceClient.from_connection_string(connection_string)
    
    def create_queue(self, queue_name):
        """Create a storage queue"""
        try:
            queue_client = self.queue_service_client.create_queue(queue_name)
            print(f"Queue {queue_name} created successfully")
            return queue_client
        except Exception as e:
            print(f"Error creating queue: {e}")
            return None
    
    def send_message(self, queue_name, message_text):
        """Send a message to the queue"""
        try:
            queue_client = self.queue_service_client.get_queue_client(queue_name)
            queue_client.send_message(message_text)
            print(f"Message sent to queue {queue_name}")
            return True
        except Exception as e:
            print(f"Error sending message: {e}")
            return False
    
    def receive_messages(self, queue_name, max_messages=1):
        """Receive messages from the queue"""
        try:
            queue_client = self.queue_service_client.get_queue_client(queue_name)
            messages = queue_client.receive_messages(max_messages=max_messages)
            return list(messages)
        except Exception as e:
            print(f"Error receiving messages: {e}")
            return []
    
    def delete_message(self, queue_name, message_id, pop_receipt):
        """Delete a message from the queue"""
        try:
            queue_client = self.queue_service_client.get_queue_client(queue_name)
            queue_client.delete_message(message_id, pop_receipt)
            print("Message deleted successfully")
            return True
        except Exception as e:
            print(f"Error deleting message: {e}")
            return False

# ============================================================================
# Azure Table Storage - NoSQL Table Storage
# ============================================================================

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

# ============================================================================
# Azure Cognitive Services - AI Services
# ============================================================================

class CognitiveServicesManager:
    """Manages Azure Cognitive Services"""
    
    def __init__(self, endpoint, key):
        self.endpoint = endpoint
        self.key = key
        self.headers = {
            'Ocp-Apim-Subscription-Key': key,
            'Content-Type': 'application/json'
        }
    
    def analyze_text_sentiment(self, text):
        """Analyze text sentiment using Text Analytics"""
        import requests
        
        try:
            url = f"{self.endpoint}/text/analytics/v3.1/sentiment"
            documents = [{'id': '1', 'language': 'en', 'text': text}]
            response = requests.post(url, headers=self.headers, json={'documents': documents})
            
            if response.status_code == 200:
                result = response.json()
                return result['documents'][0]['sentiment']
            else:
                print(f"Error analyzing sentiment: {response.status_code}")
                return None
        except Exception as e:
            print(f"Error analyzing sentiment: {e}")
            return None
    
    def detect_language(self, text):
        """Detect language of text"""
        import requests
        
        try:
            url = f"{self.endpoint}/text/analytics/v3.1/languages"
            documents = [{'id': '1', 'text': text}]
            response = requests.post(url, headers=self.headers, json={'documents': documents})
            
            if response.status_code == 200:
                result = response.json()
                return result['documents'][0]['detectedLanguage']['name']
            else:
                print(f"Error detecting language: {response.status_code}")
                return None
        except Exception as e:
            print(f"Error detecting language: {e}")
            return None
    
    def recognize_text_from_image(self, image_url):
        """Recognize text from image using Computer Vision"""
        import requests
        
        try:
            url = f"{self.endpoint}/vision/v3.2/read/analyze"
            response = requests.post(
                url,
                headers={'Ocp-Apim-Subscription-Key': self.key},
                json={'url': image_url}
            )
            
            if response.status_code == 202:
                operation_url = response.headers['Operation-Location']
                return operation_url
            else:
                print(f"Error recognizing text: {response.status_code}")
                return None
        except Exception as e:
            print(f"Error recognizing text: {e}")
            return None

# ============================================================================
# Azure App Service - Web App Hosting
# ============================================================================

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

# ============================================================================
# Azure Container Instances - Container Hosting
# ============================================================================

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
            from azure.mgmt.containerinstance.models import ContainerGroup, Container, ImageRegistryCredential, \
                ResourceRequirements, ResourceRequests, ContainerGroupNetworkProfile
            
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

# ============================================================================
# Azure Monitor - Monitoring and Metrics
# ============================================================================

class AzureMonitorManager:
    """Manages Azure Monitor metrics and logs"""
    
    def __init__(self, subscription_id):
        credential = DefaultAzureCredential()
        self.subscription_id = subscription_id
        self.monitor_client = MonitorClient(credential, subscription_id)
    
    def get_metrics(self, resource_id, metric_names, start_time, end_time):
        """Get metrics for a resource"""
        try:
            metrics_data = self.monitor_client.metrics.list(
                resource_id,
                timespan=f"{start_time}/{end_time}",
                interval='PT1H',
                metricnames=','.join(metric_names)
            )
            return metrics_data
        except Exception as e:
            print(f"Error getting metrics: {e}")
            return None
    
    def create_metric_alert(self, resource_group_name, alert_name, target_resource_id, metric_name, threshold):
        """Create a metric alert"""
        try:
            from azure.mgmt.monitor import MonitorManagementClient
            from azure.mgmt.monitor.models import MetricAlertResource, MetricAlertSingleResourceMultipleMetricCriteria, \
                MetricCriteria, MetricAlertAction
            
            monitor_management_client = MonitorManagementClient(
                DefaultAzureCredential(), self.subscription_id
            )
            
            criteria = MetricCriteria(
                metric_name=metric_name,
                metric_namespace='Microsoft.Compute/virtualMachines',
                operator='GreaterThan',
                threshold=threshold,
                time_aggregation='Average'
            )
            
            alert_criteria = MetricAlertSingleResourceMultipleMetricCriteria(
                all_of=[criteria]
            )
            
            alert_resource = MetricAlertResource(
                location='global',
                description='Alert when CPU exceeds threshold',
                severity=2,
                enabled=True,
                scopes=[target_resource_id],
                evaluation_frequency='PT1M',
                window_size='PT5M',
                criteria=alert_criteria
            )
            
            alert = monitor_management_client.metric_alerts.create_or_update(
                resource_group_name,
                alert_name,
                alert_resource
            )
            print(f"Metric alert {alert_name} created")
            return alert
        except Exception as e:
            print(f"Error creating metric alert: {e}")
            return None

# ============================================================================
# Extended Main Application Example
# ============================================================================

def extended_main():
    """Extended example using additional Azure services"""
    
    queue_storage_manager = QueueStorageManager(
        connection_string=os.environ.get('AZURE_STORAGE_CONNECTION_STRING', '')
    )
    
    table_storage_manager = TableStorageManager(
        connection_string=os.environ.get('AZURE_STORAGE_CONNECTION_STRING', '')
    )
    
    cognitive_services_manager = CognitiveServicesManager(
        endpoint=os.environ.get('COGNITIVE_SERVICES_ENDPOINT', ''),
        key=os.environ.get('COGNITIVE_SERVICES_KEY', '')
    )
    
    print("Starting extended Azure services integration example...")
    
    # Use Queue Storage
    queue_storage_manager.create_queue('task-queue')
    queue_storage_manager.send_message('task-queue', json.dumps({'task': 'process_data', 'id': str(uuid.uuid4())}))
    
    # Use Table Storage
    table_storage_manager.create_table('UserSessions')
    entity = {
        'PartitionKey': 'users',
        'RowKey': str(uuid.uuid4()),
        'SessionId': str(uuid.uuid4()),
        'UserId': 'user123',
        'StartTime': datetime.now().isoformat()
    }
    table_storage_manager.create_entity('UserSessions', entity)
    
    # Use Cognitive Services
    sentiment = cognitive_services_manager.analyze_text_sentiment("I love this product!")
    print(f"Sentiment analysis result: {sentiment}")
    
    print("Extended Azure services integration example completed!")

if __name__ == '__main__':
    main()
    extended_main()
