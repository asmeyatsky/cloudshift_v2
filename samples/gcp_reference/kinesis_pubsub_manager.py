"""
GCP-oriented analogue of KinesisManager using **Cloud Pub/Sub**.

There is no Kinesis-shaped API on GCP. **Pub/Sub** is the usual replacement:
topics ≈ streams, **publish** ≈ put_record, **pull subscriptions** ≈ consumers
(shards / shard iterators do not exist — partitioning is automatic; use
**ordering keys** to mimic partition keys).

Broken transforms (boto3 + Cloud Functions decorators) are invalid.

Requires: pip install google-cloud-pubsub google-api-core
"""
from __future__ import annotations

from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud import pubsub_v1


class KinesisPubSubManager:
    """
    Kinesis-like surface over Pub/Sub: one topic + one pull subscription per
    \"stream\". `get_shard_iterator` returns the subscription resource name
    (use it as opaque iterator with `get_records`).
    """

    def __init__(self, project_id: str):
        self.project_id = project_id
        self._publisher = pubsub_v1.PublisherClient()
        self._subscriber = pubsub_v1.SubscriberClient()

    def _topic(self, stream_name: str) -> str:
        return self._publisher.topic_path(self.project_id, stream_name)

    def _subscription(self, stream_name: str) -> str:
        return self._subscriber.subscription_path(
            self.project_id, f"{stream_name}-reader"
        )

    def create_stream(self, stream_name: str, shard_count: int = 1) -> bool:
        """Create topic + pull subscription. `shard_count` is informational only."""
        _ = shard_count
        topic, sub = self._topic(stream_name), self._subscription(stream_name)
        try:
            self._publisher.create_topic(request={"name": topic})
        except gcp_exceptions.AlreadyExists:
            pass
        try:
            self._subscriber.create_subscription(
                request={"name": sub, "topic": topic}
            )
        except gcp_exceptions.AlreadyExists:
            pass
        print(f"Pub/Sub topic (stream) {stream_name!r} ready with subscription {sub!r}")
        return True

    def put_record(
        self, stream_name: str, data: str, partition_key: str
    ) -> dict[str, Any] | None:
        """Publish bytes; ordering_key aligns records for the same key."""
        try:
            future = self._publisher.publish(
                self._topic(stream_name),
                data.encode("utf-8"),
                partition_key=partition_key,
            )
            message_id = future.result()
            print(f"Published to {stream_name}, message_id={message_id}")
            return {"SequenceNumber": message_id, "ShardId": "pubsub"}
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error publishing: {e}")
            return None

    def get_shard_iterator(
        self,
        stream_name: str,
        shard_id: str = "0",
        shard_iterator_type: str = "TRIM_HORIZON",
    ) -> str | None:
        """
        Returns subscription resource name to pass to `get_records`.
        `shard_id` / `shard_iterator_type` are ignored (no shards in Pub/Sub).
        """
        _ = (shard_id, shard_iterator_type)
        try:
            sub = self._subscription(stream_name)
            self._subscriber.get_subscription(request={"subscription": sub})
            return sub
        except gcp_exceptions.NotFound:
            print(f"No subscription for stream {stream_name!r}; call create_stream first")
            return None
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error resolving iterator: {e}")
            return None

    def get_records(self, shard_iterator: str, limit: int = 10) -> list[dict[str, Any]]:
        """
        Pull up to `limit` messages from the subscription (iterator = subscription path).
        Messages are **acked** after read (at-most-once style demo).
        """
        try:
            resp = self._subscriber.pull(
                request={
                    "subscription": shard_iterator,
                    "max_messages": min(limit, 1000),
                    "return_immediately": True,
                },
                timeout=10.0,
            )
            if not resp.received_messages:
                return []
            ack_ids = [m.ack_id for m in resp.received_messages]
            self._subscriber.acknowledge(
                request={"subscription": shard_iterator, "ack_ids": ack_ids}
            )
            out = []
            for m in resp.received_messages:
                raw = m.message.data.decode("utf-8", errors="replace")
                pk = (m.message.attributes or {}).get("partition_key", "")
                out.append(
                    {
                        "Data": raw.encode("utf-8"),
                        "PartitionKey": pk,
                        "ApproximateArrivalTimestamp": m.message.publish_time,
                    }
                )
            return out
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error pulling records: {e}")
            return []
