import json
from google.cloud import pubsub_v1
from concurrent import futures

publisher = pubsub_v1.PublisherClient()
subscriber = pubsub_v1.SubscriberClient()

TOPIC_PATH = 'projects/my-project/topics/my-topic'
SUBSCRIPTION_PATH = 'projects/my-project/subscriptions/my-subscription'


def send_message(message_body, attributes=None):
    """Publish a message to a Pub/Sub topic."""
    data = json.dumps(message_body).encode('utf-8')
    attrs = {}
    if attributes:
        attrs = {key: str(val) for key, val in attributes.items()}

    future = publisher.publish(TOPIC_PATH, data=data, **attrs)
    return future.result()


def send_batch(messages):
    """Publish a batch of messages to a Pub/Sub topic."""
    publish_futures = []
    for msg in messages:
        data = json.dumps(msg).encode('utf-8')
        future = publisher.publish(TOPIC_PATH, data=data)
        publish_futures.append(future)

    results = []
    for future in futures.as_completed(set(publish_futures)):
        results.append(future.result())
    return results


def receive_messages(max_messages=10, wait_time=20):
    """Pull messages from a Pub/Sub subscription."""
    response = subscriber.pull(
        request={
            'subscription': SUBSCRIPTION_PATH,
            'max_messages': max_messages,
        },
        timeout=wait_time,
    )

    for msg in response.received_messages:
        yield json.loads(msg.message.data.decode('utf-8')), msg.ack_id


def delete_message(ack_id):
    """Acknowledge a processed message."""
    subscriber.acknowledge(
        request={
            'subscription': SUBSCRIPTION_PATH,
            'ack_ids': [ack_id],
        }
    )
