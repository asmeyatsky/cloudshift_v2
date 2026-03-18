"""
SNS-shaped API on top of **Cloud Pub/Sub** (valid GCP code).

The broken transform mixes `PublisherClient` with SNS kwargs (`Name=`,
`TopicArn=`) — those methods do not exist on the Python client.

Mapping:
  TopicArn / topic name     -> full topic resource name
  publish                   -> PublisherClient.publish
  subscribe (HTTP/S)        -> push subscription
  subscribe (email/sms)     -> not native; use SendGrid + subscriber or Eventarc

Requires: pip install google-cloud-pubsub google-api-core
"""
from __future__ import annotations

import json
import uuid
from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud import pubsub_v1
from google.cloud.pubsub_v1.types import PushConfig


class SNSPubSubManager:
    """Pub/Sub backend with method names similar to the AWS SNS sample."""

    def __init__(self, project_id: str):
        self.project_id = project_id
        self._publisher = pubsub_v1.PublisherClient()
        self._subscriber = pubsub_v1.SubscriberClient()

    def create_topic(self, topic_name: str) -> str | None:
        path = self._publisher.topic_path(self.project_id, topic_name)
        try:
            self._publisher.create_topic(request={"name": path})
            print(f"Topic {topic_name} created: {path}")
            return path
        except gcp_exceptions.AlreadyExists:
            print(f"Topic {topic_name} already exists: {path}")
            return path
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error creating topic: {e}")
            return None

    def publish_message(
        self,
        topic_arn: str,
        message: str | dict[str, Any],
        subject: str | None = None,
    ) -> str | None:
        try:
            body = json.dumps(message) if isinstance(message, dict) else str(message)
            kwargs: dict[str, str] = {}
            if subject:
                kwargs["subject"] = subject
            mid = self._publisher.publish(
                topic_arn, body.encode("utf-8"), **kwargs
            ).result()
            print(f"Message published: {mid}")
            return mid
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error publishing message: {e}")
            return None

    def subscribe(self, topic_arn: str, protocol: str, endpoint: str) -> str | None:
        """
        `https`/`http`: push subscription to URL (replaces SNS → HTTPS/Lambda).
        `sqs`-like: creates a **pull** subscription; consumer uses SubscriberClient.pull.
        `email`: not supported — use a different product or forward from a push endpoint.
        """
        sub_id = f"sub-{uuid.uuid4().hex[:12]}"
        sub_path = self._subscriber.subscription_path(self.project_id, sub_id)
        try:
            proto = protocol.lower().strip()
            if proto in ("https", "http", "lambda"):
                self._subscriber.create_subscription(
                    request={
                        "name": sub_path,
                        "topic": topic_arn,
                        "push_config": PushConfig(push_endpoint=endpoint),
                    }
                )
            elif proto == "sqs":
                self._subscriber.create_subscription(
                    request={"name": sub_path, "topic": topic_arn}
                )
            else:
                print(
                    f"Protocol {protocol!r} not mapped; use https push or sqs (pull). "
                    "Email/SMS are not direct Pub/Sub equivalents."
                )
                return None
            print(f"Subscription created: {sub_path}")
            return sub_path
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error subscribing: {e}")
            return None

    def list_topics(self) -> list[dict[str, str]]:
        try:
            project = f"projects/{self.project_id}"
            return [
                {"TopicArn": t.name}
                for t in self._publisher.list_topics(request={"project": project})
            ]
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error listing topics: {e}")
            return []

    def delete_topic(self, topic_arn: str) -> bool:
        try:
            self._publisher.delete_topic(request={"topic": topic_arn})
            print(f"Topic deleted: {topic_arn}")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error deleting topic: {e}")
            return False
