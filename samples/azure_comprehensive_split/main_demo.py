"""Orchestrator — wire split Azure managers together.
Run: PYTHONPATH=samples/azure_comprehensive_split python samples/azure_comprehensive_split/main_demo.py
(Requires env vars / Azure packages; may call real APIs.)
"""
import json
import os
import uuid
from datetime import datetime

from blob_storage_manager import BlobStorageManager
from cosmos_db_manager import CosmosDBManager
from service_bus_manager import ServiceBusManager
from event_grid_manager import EventGridManager
from queue_storage_manager import QueueStorageManager
from table_storage_manager import TableStorageManager
from cognitive_services_manager import CognitiveServicesManager


def main():
    """Main application demonstrating Azure services integration"""

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

    blob_manager.upload_blob('data.json', json.dumps({'key': 'value'}))

    user_item = {
        'id': str(uuid.uuid4()),
        'name': 'Jane Doe',
        'email': 'jane@example.com',
        'created_at': datetime.now().isoformat()
    }
    cosmos_manager.create_item('Users', user_item)

    servicebus_manager.send_queue_message(
        'user-queue', json.dumps({'event': 'user_created', 'user_id': user_item['id']})
    )

    event_grid_manager.publish_event(
        event_type='User.Created',
        subject=f'/users/{user_item["id"]}',
        data={'userId': user_item['id'], 'email': user_item['email']}
    )

    print("Azure services integration example completed!")


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

    queue_storage_manager.create_queue('task-queue')
    queue_storage_manager.send_message(
        'task-queue', json.dumps({'task': 'process_data', 'id': str(uuid.uuid4())})
    )

    table_storage_manager.create_table('UserSessions')
    entity = {
        'PartitionKey': 'users',
        'RowKey': str(uuid.uuid4()),
        'SessionId': str(uuid.uuid4()),
        'UserId': 'user123',
        'StartTime': datetime.now().isoformat()
    }
    table_storage_manager.create_entity('UserSessions', entity)

    sentiment = cognitive_services_manager.analyze_text_sentiment("I love this product!")
    print(f"Sentiment analysis result: {sentiment}")

    print("Extended Azure services integration example completed!")


if __name__ == '__main__':
    main()
    extended_main()
