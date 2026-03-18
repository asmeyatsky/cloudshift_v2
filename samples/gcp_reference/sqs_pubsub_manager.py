"""
SQS-shaped API on **Cloud Pub/Sub** (topic + single pull subscription per queue).

The broken transform calls SQS methods on `PublisherClient` — those do not exist.

Model:
  Queue URL     -> **topic** resource name (publish here)
  Consumer      -> **pull subscription** `{topic-id}-sqs` (receive / ack / purge)

`ReceiptHandle` -> Pub/Sub **ack_id**. Pass it to `delete_message`.

Requires: pip install google-cloud-pubsub google-api-core
"""
from __future__ import annotations

import re
from datetime import datetime, timedelta, timezone
from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud import pubsub_v1


def _sanitize_queue_id(queue_name: str) -> str:
    s = re.sub(r"[^a-zA-Z0-9-]", "-", queue_name.lower()).strip("-")
    return (s or "queue")[:100]


class SQSPubSubManager:
    def __init__(self, project_id: str):
        self.project_id = project_id
        self._publisher = pubsub_v1.PublisherClient()
        self._subscriber = pubsub_v1.SubscriberClient()
        self._topic_to_sub: dict[str, str] = {}

    def _subscription_for_topic(self, queue_url: str) -> str:
        if queue_url in self._topic_to_sub:
            return self._topic_to_sub[queue_url]
        tid = queue_url.split("/")[-1]
        return self._subscriber.subscription_path(self.project_id, f"{tid}-sqs")

    def create_queue(
        self, queue_name: str, attributes: dict[str, str] | None = None
    ) -> str | None:
        _ = attributes  # retention/delay not mapped 1:1; tune via console/Terraform
        tid = _sanitize_queue_id(queue_name)
        topic_path = self._publisher.topic_path(self.project_id, tid)
        sub_path = self._subscriber.subscription_path(self.project_id, f"{tid}-sqs")
        try:
            self._publisher.create_topic(request={"name": topic_path})
        except gcp_exceptions.AlreadyExists:
            pass
        try:
            self._subscriber.create_subscription(
                request={
                    "name": sub_path,
                    "topic": topic_path,
                    "ack_deadline_seconds": 600,
                }
            )
        except gcp_exceptions.AlreadyExists:
            pass
        self._topic_to_sub[topic_path] = sub_path
        print(f"Queue {queue_name!r} -> {topic_path}")
        return topic_path

    def send_message(
        self,
        queue_url: str,
        message_body: str,
        attributes: dict[str, Any] | None = None,
    ) -> str | None:
        try:
            kwargs: dict[str, str] = {}
            if attributes:
                for k, v in attributes.items():
                    if isinstance(v, dict) and "StringValue" in v:
                        kwargs[k] = str(v["StringValue"])
                    else:
                        kwargs[k] = str(v)
            mid = self._publisher.publish(
                queue_url, message_body.encode("utf-8"), **kwargs
            ).result()
            print(f"Message sent: {mid}")
            return mid
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error sending message: {e}")
            return None

    def receive_messages(
        self, queue_url: str, max_messages: int = 1, wait_time: int = 20
    ) -> list[dict[str, Any]]:
        try:
            sub = self._subscription_for_topic(queue_url)
            resp = self._subscriber.pull(
                request={
                    "subscription": sub,
                    "max_messages": min(max(1, max_messages), 1000),
                    "return_immediately": False,
                },
                timeout=float(wait_time),
            )
            out = []
            for m in resp.received_messages:
                attrs = dict(m.message.attributes or {})
                out.append(
                    {
                        "MessageId": m.message.message_id,
                        "ReceiptHandle": m.ack_id,
                        "Body": m.message.data.decode("utf-8", errors="replace"),
                        "MessageAttributes": {
                            k: {"StringValue": v, "DataType": "String"}
                            for k, v in attrs.items()
                        },
                    }
                )
            return out
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error receiving messages: {e}")
            return []

    def delete_message(self, queue_url: str, receipt_handle: str) -> bool:
        try:
            sub = self._subscription_for_topic(queue_url)
            self._subscriber.acknowledge(
                request={"subscription": sub, "ack_ids": [receipt_handle]}
            )
            print("Message acked (deleted)")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error deleting message: {e}")
            return False

    def get_queue_url(self, queue_name: str) -> str | None:
        """Returns topic path for this queue name (create_queue must have run once)."""
        tid = _sanitize_queue_id(queue_name)
        path = self._publisher.topic_path(self.project_id, tid)
        try:
            self._publisher.get_topic(request={"topic": path})
            return path
        except gcp_exceptions.NotFound:
            print(f"Queue/topic for {queue_name!r} not found; call create_queue first")
            return None
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error getting queue URL: {e}")
            return None

    def purge_queue(self, queue_url: str) -> bool:
        """Seek subscription past backlog (approximate SQS purge)."""
        try:
            sub = self._subscription_for_topic(queue_url)
            future = datetime.now(timezone.utc) + timedelta(days=1)
            from google.protobuf import timestamp_pb2

            ts = timestamp_pb2.Timestamp()
            ts.FromDatetime(future)
            self._subscriber.seek(request={"subscription": sub, "time": ts})
            print("Subscription seek applied (messages dropped from backlog)")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error purging queue: {e}")
            return False
