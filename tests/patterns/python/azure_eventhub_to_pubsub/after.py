from google.cloud import pubsub_v1

TOPIC_PATH = "projects/my-project/topics/telemetry"


def send_events(events: list[bytes]):
    publisher = pubsub_v1.PublisherClient()
    for e in events:
        publisher.publish(TOPIC_PATH, data=e)
