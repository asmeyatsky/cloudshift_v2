"""
Pub/Sub analogue for **Azure Event Grid** topic publishing.

Azure maps:
  EventGridPublisherClient + topic endpoint/key  →  Pub/Sub topic + IAM (ADC or service account).
  EventGridEvent (type, subject, data)         →  message data (JSON) + string attributes.

**Subscriptions / routing** (Event Grid → HTTP, Functions, Storage): use **Eventarc**
triggers (CloudEvents) or Pub/Sub push/pull subscriptions — not shown here.

Requires: pip install google-cloud-pubsub google-api-core
"""
from __future__ import annotations

import json
from typing import Any, Mapping

from google.api_core import exceptions as gcp_exceptions
from google.cloud import pubsub_v1


def _event_payload(data: Any) -> bytes:
    if isinstance(data, bytes):
        return data
    if isinstance(data, str):
        return data.encode("utf-8")
    return json.dumps(data).encode("utf-8")


def _attrs(event_type: str, subject: str, data_version: str = "1.0") -> dict[str, str]:
    return {
        "event_type": str(event_type)[:1024],
        "subject": str(subject)[:1024],
        "data_version": str(data_version)[:1024],
    }


class EventGridPubSubManager:
    """Publish events to a Pub/Sub topic (custom Event Grid topic analogue)."""

    def __init__(self, project_id: str, topic_id: str) -> None:
        self._publisher = pubsub_v1.PublisherClient()
        self._topic_path = self._publisher.topic_path(project_id, topic_id)

    def publish_event(self, event_type: str, subject: str, data: Any) -> bool:
        """Publish one event (same shape as Azure EventGridEvent fields)."""
        try:
            future = self._publisher.publish(
                self._topic_path,
                _event_payload(data),
                **_attrs(event_type, subject),
            )
            future.result()
            print(f"Event {event_type} published successfully")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error publishing event: {e}")
            return False
        except Exception as e:
            print(f"Error publishing event: {e}")
            return False

    def publish_events_batch(self, events: list[Any]) -> bool:
        """Publish multiple events. Each item: mapping with event_type, subject, data — or objects with those attrs."""
        try:
            futures = []
            for ev in events:
                if isinstance(ev, Mapping):
                    et, sub, dat = ev["event_type"], ev["subject"], ev["data"]
                    ver = str(ev.get("data_version", "1.0"))
                else:
                    et, sub, dat = ev.event_type, ev.subject, ev.data
                    ver = getattr(ev, "data_version", None) or "1.0"
                futures.append(
                    self._publisher.publish(
                        self._topic_path,
                        _event_payload(dat),
                        **_attrs(et, sub, ver),
                    )
                )
            for f in futures:
                f.result()
            print(f"{len(events)} events published successfully")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error publishing events batch: {e}")
            return False
        except Exception as e:
            print(f"Error publishing events batch: {e}")
            return False
