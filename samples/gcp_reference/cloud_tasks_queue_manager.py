"""GCP Cloud Tasks — analogue for Azure Storage Queue (named queue + enqueue + dequeue via worker)."""

from typing import Any, Optional

from google.cloud import tasks_v2
from google.cloud.tasks_v2.types import HttpMethod
from google.protobuf import duration_pb2


class CloudTasksQueueManager:
    """Create queue, enqueue HTTP tasks; workers pull/process (unlike pull-based SQS — see sqs_pubsub_manager for Pub/Sub)."""

    def __init__(self, project_id: str, location: str) -> None:
        self.project_id = project_id
        self.location = location
        self.client = tasks_v2.CloudTasksClient()

    def _queue_path(self, queue_name: str) -> str:
        return self.client.queue_path(self.project_id, self.location, queue_name)

    def create_queue(self, queue_name: str) -> Optional[Any]:
        """Create a Cloud Tasks queue (idempotent if already exists)."""
        parent = self.client.common_location_path(self.project_id, self.location)
        try:
            q = self.client.create_queue(
                parent=parent,
                queue={"name": self._queue_path(queue_name)},
            )
            print(f"Queue {queue_name} created")
            return q
        except Exception as e:
            if "ALREADY_EXISTS" in str(e) or "409" in str(e):
                return self.client.get_queue(name=self._queue_path(queue_name))
            print(f"Error creating queue: {e}")
            return None

    def send_message(
        self,
        queue_name: str,
        url: str,
        payload: bytes,
        *,
        method: HttpMethod = HttpMethod.POST,
    ) -> bool:
        """Enqueue a task that POSTs payload to `url` (worker endpoint)."""
        try:
            task = {
                "http_request": {
                    "http_method": method,
                    "url": url,
                    "headers": {"Content-Type": "application/json"},
                    "body": payload,
                }
            }
            self.client.create_task(parent=self._queue_path(queue_name), task=task)
            print(f"Task enqueued to {queue_name}")
            return True
        except Exception as e:
            print(f"Error enqueueing: {e}")
            return False

    def set_retry(self, queue_name: str, max_attempts: int = 5) -> bool:
        """Optional: tune queue retry (Azure has similar via queue metadata)."""
        try:
            path = self._queue_path(queue_name)
            q = self.client.get_queue(name=path)
            q.retry_config.max_attempts = max_attempts
            q.retry_config.max_retry_duration = duration_pb2.Duration(seconds=3600)
            self.client.update_queue(queue=q, update_mask={"paths": ["retry_config"]})
            return True
        except Exception as e:
            print(f"Error updating retry: {e}")
            return False
