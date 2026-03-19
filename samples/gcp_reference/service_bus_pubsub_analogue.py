"""
**Cloud Pub/Sub** — analogue for **Azure Service Bus** (queue + topic patterns).

| Azure              | GCP                                      |
|-------------------|------------------------------------------|
| Queue send/receive| Topic + **pull subscription** (see also ``sqs_pubsub_manager``) |
| Topic publish     | **Publish** to a Pub/Sub topic           |
| Subscription      | **Push or pull** subscription on topic   |

This module uses one **topic** per logical queue/topic name and a dedicated
**subscription** for queue-style pull consumption.

Requires: ``pip install google-cloud-pubsub google-api-core``
"""
from __future__ import annotations

from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud import pubsub_v1


class ServiceBusPubSubAnalogue:
    """Rough shape of ``ServiceBusManager`` (Pub/Sub backend)."""

    def __init__(self, project_id: str) -> None:
        self._project_id = project_id
        self._publisher = pubsub_v1.PublisherClient()
        self._subscriber = pubsub_v1.SubscriberClient()

    def _topic_path(self, name: str) -> str:
        safe = name.replace(".", "-").replace("/", "-")[:250]
        return self._publisher.topic_path(self._project_id, safe)

    def _sub_path(self, topic_name: str, sub_name: str) -> str:
        return self._subscriber.subscription_path(
            self._project_id, f"{topic_name}-{sub_name}"[:250]
        )

    def _ensure_topic(self, name: str) -> str:
        path = self._topic_path(name)
        try:
            self._publisher.get_topic(request={"topic": path})
        except gcp_exceptions.NotFound:
            self._publisher.create_topic(request={"name": path})
        return path

    def _ensure_subscription(self, topic_name: str, sub_suffix: str = "queue") -> str:
        tpath = self._ensure_topic(topic_name)
        spath = self._sub_path(topic_name, sub_suffix)
        try:
            self._subscriber.get_subscription(request={"subscription": spath})
        except gcp_exceptions.NotFound:
            self._subscriber.create_subscription(
                request={"name": spath, "topic": tpath}
            )
        return spath

    def send_queue_message(
        self,
        queue_name: str,
        message_body: str | bytes,
        properties: dict[str, Any] | None = None,
    ) -> bool:
        try:
            path = self._ensure_topic(queue_name)
            data = (
                message_body.encode("utf-8")
                if isinstance(message_body, str)
                else message_body
            )
            attrs = {k: str(v)[:1024] for k, v in (properties or {}).items()}
            future = self._publisher.publish(path, data, **attrs)
            future.result(timeout=30)
            print(f"Message sent to queue {queue_name}")
            return True
        except Exception as e:
            print(f"Error sending message: {e}")
            return False

    def receive_queue_messages(
        self, queue_name: str, max_messages: int = 1
    ) -> list[dict[str, Any]]:
        try:
            spath = self._ensure_subscription(queue_name, "pull")
            out: list[dict[str, Any]] = []
            response = self._subscriber.pull(
                request={
                    "subscription": spath,
                    "max_messages": min(max_messages, 1000),
                },
                timeout=10.0,
            )
            ack_ids = []
            for received in response.received_messages:
                m = received.message
                body = (
                    m.data.decode("utf-8", errors="replace")
                    if m.data
                    else ""
                )
                props = dict(m.attributes) if m.attributes else {}
                out.append({"body": body, "properties": props})
                ack_ids.append(received.ack_id)
            if ack_ids:
                self._subscriber.acknowledge(
                    request={"subscription": spath, "ack_ids": ack_ids}
                )
            return out
        except Exception as e:
            print(f"Error receiving messages: {e}")
            return []

    def create_topic(self, topic_name: str) -> bool:
        try:
            self._ensure_topic(topic_name)
            print(f"Topic {topic_name} creation initiated")
            return True
        except Exception as e:
            print(f"Error creating topic: {e}")
            return False

    def send_topic_message(
        self,
        topic_name: str,
        message_body: str | bytes,
        properties: dict[str, Any] | None = None,
    ) -> bool:
        return self.send_queue_message(topic_name, message_body, properties)

    def create_subscription(self, topic_name: str, subscription_name: str) -> bool:
        try:
            tpath = self._ensure_topic(topic_name)
            spath = self._subscriber.subscription_path(
                self._project_id,
                f"{topic_name}-{subscription_name}"[:250],
            )
            try:
                self._subscriber.get_subscription(request={"subscription": spath})
            except gcp_exceptions.NotFound:
                self._subscriber.create_subscription(
                    request={"name": spath, "topic": tpath}
                )
            print(f"Subscription {subscription_name} created for topic {topic_name}")
            return True
        except Exception as e:
            print(f"Error creating subscription: {e}")
            return False
