"""Orchestrator — wire split Azure managers together.

Run from repo root::

  PYTHONPATH=samples/azure_comprehensive_split python samples/azure_comprehensive_split/main_demo.py

Each block runs only if the relevant env vars are set; otherwise it prints a skip
message. Imports are **lazy** so you can see skips even without Azure packages
installed (install SDKs only for services you call).

**main()** — optional env:

- ``AZURE_STORAGE_ACCOUNT``, ``AZURE_STORAGE_KEY`` — Blob upload
- ``COSMOS_ENDPOINT``, ``COSMOS_KEY`` — Cosmos (emulator: host + key in env)
- ``SERVICE_BUS_CONNECTION_STRING`` — Service Bus queue send
- ``EVENT_GRID_ENDPOINT``, ``EVENT_GRID_KEY`` — Event Grid publish

**extended_main()** — optional:

- ``AZURE_STORAGE_CONNECTION_STRING`` — Queue + Table storage
- ``COGNITIVE_SERVICES_ENDPOINT``, ``COGNITIVE_SERVICES_KEY`` — sentiment
"""
import json
import os
import uuid
from datetime import datetime


def main() -> None:
    """Main application demonstrating Azure services integration."""

    print("Starting Azure services integration example...")

    user_item = {
        "id": str(uuid.uuid4()),
        "name": "Jane Doe",
        "email": "jane@example.com",
        "created_at": datetime.now().isoformat(),
    }

    acc = os.environ.get("AZURE_STORAGE_ACCOUNT", "").strip()
    key = os.environ.get("AZURE_STORAGE_KEY", "").strip()
    if acc and key:
        try:
            from blob_storage_manager import BlobStorageManager

            blob_manager = BlobStorageManager(account_name=acc, account_key=key)
            blob_manager.upload_blob("data.json", json.dumps({"key": "value"}))
        except ModuleNotFoundError as e:
            print(f"Blob storage: install Azure SDK ({e}).")
        except Exception as e:
            print(f"Blob storage step failed: {e}")
    else:
        print("Skip blob storage (set AZURE_STORAGE_ACCOUNT, AZURE_STORAGE_KEY).")

    cosmos_ep = os.environ.get("COSMOS_ENDPOINT", "").strip()
    cosmos_key = os.environ.get("COSMOS_KEY", "").strip()
    if cosmos_ep and cosmos_key:
        try:
            from cosmos_db_manager import CosmosDBManager

            cosmos_manager = CosmosDBManager(
                endpoint=cosmos_ep,
                key=cosmos_key,
                database_name="MyDatabase",
            )
            cosmos_manager.create_item("Users", user_item)
        except ModuleNotFoundError as e:
            print(f"Cosmos DB: install Azure SDK ({e}).")
        except Exception as e:
            print(f"Cosmos DB step failed: {e}")
    else:
        print("Skip Cosmos DB (set COSMOS_ENDPOINT, COSMOS_KEY).")

    sb = os.environ.get("SERVICE_BUS_CONNECTION_STRING", "").strip()
    if sb:
        try:
            from service_bus_manager import ServiceBusManager

            servicebus_manager = ServiceBusManager(connection_string=sb)
            servicebus_manager.send_queue_message(
                "user-queue",
                json.dumps({"event": "user_created", "user_id": user_item["id"]}),
            )
        except ModuleNotFoundError as e:
            print(f"Service Bus: install Azure SDK ({e}).")
        except Exception as e:
            print(f"Service Bus step failed: {e}")
    else:
        print("Skip Service Bus (set SERVICE_BUS_CONNECTION_STRING).")

    eg_ep = os.environ.get("EVENT_GRID_ENDPOINT", "").strip()
    eg_key = os.environ.get("EVENT_GRID_KEY", "").strip()
    if eg_ep and eg_key:
        try:
            from event_grid_manager import EventGridManager

            event_grid_manager = EventGridManager(
                topic_endpoint=eg_ep, topic_key=eg_key
            )
            event_grid_manager.publish_event(
                event_type="User.Created",
                subject=f"/users/{user_item['id']}",
                data={
                    "userId": user_item["id"],
                    "email": user_item["email"],
                },
            )
        except ModuleNotFoundError as e:
            print(f"Event Grid: install Azure SDK ({e}).")
        except Exception as e:
            print(f"Event Grid step failed: {e}")
    else:
        print("Skip Event Grid (set EVENT_GRID_ENDPOINT, EVENT_GRID_KEY).")

    print("Azure services integration example completed!")


def extended_main() -> None:
    """Extended example using additional Azure services."""

    print("Starting extended Azure services integration example...")

    conn = os.environ.get("AZURE_STORAGE_CONNECTION_STRING", "").strip()
    if conn:
        try:
            from queue_storage_manager import QueueStorageManager
            from table_storage_manager import TableStorageManager

            queue_storage_manager = QueueStorageManager(connection_string=conn)
            queue_storage_manager.create_queue("task-queue")
            queue_storage_manager.send_message(
                "task-queue",
                json.dumps({"task": "process_data", "id": str(uuid.uuid4())}),
            )

            table_storage_manager = TableStorageManager(connection_string=conn)
            table_storage_manager.create_table("UserSessions")
            entity = {
                "PartitionKey": "users",
                "RowKey": str(uuid.uuid4()),
                "SessionId": str(uuid.uuid4()),
                "UserId": "user123",
                "StartTime": datetime.now().isoformat(),
            }
            table_storage_manager.create_entity("UserSessions", entity)
        except ModuleNotFoundError as e:
            print(f"Queue/table storage: install Azure SDK ({e}).")
        except Exception as e:
            print(f"Queue/table storage step failed: {e}")
    else:
        print(
            "Skip queue + table storage (set AZURE_STORAGE_CONNECTION_STRING)."
        )

    cs_ep = os.environ.get("COGNITIVE_SERVICES_ENDPOINT", "").strip()
    cs_key = os.environ.get("COGNITIVE_SERVICES_KEY", "").strip()
    if cs_ep and cs_key:
        try:
            from cognitive_services_manager import CognitiveServicesManager

            cognitive_services_manager = CognitiveServicesManager(
                endpoint=cs_ep, key=cs_key
            )
            sentiment = cognitive_services_manager.analyze_text_sentiment(
                "I love this product!"
            )
            print(f"Sentiment analysis result: {sentiment}")
        except ModuleNotFoundError as e:
            print(f"Cognitive Services: install Azure SDK ({e}).")
        except Exception as e:
            print(f"Cognitive Services step failed: {e}")
    else:
        print(
            "Skip Cognitive Services (set COGNITIVE_SERVICES_ENDPOINT, "
            "COGNITIVE_SERVICES_KEY)."
        )

    print("Extended Azure services integration example completed!")


if __name__ == "__main__":
    main()
    extended_main()
