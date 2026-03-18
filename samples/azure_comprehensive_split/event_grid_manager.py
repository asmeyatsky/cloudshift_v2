"""Azure Event Grid — event routing."""
from azure.eventgrid import EventGridPublisherClient, EventGridEvent


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
