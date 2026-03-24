from azure.eventhub import EventHubProducerClient, EventData

conn_str = "Endpoint=sb://..."
EVENT_HUB = "telemetry"


def send_events(events: list[bytes]):
    producer = EventHubProducerClient.from_connection_string(
        conn_str, eventhub_name=EVENT_HUB
    )
    with producer:
        batch = producer.create_batch()
        for e in events:
            batch.add(EventData(e))
        producer.send_batch(batch)
