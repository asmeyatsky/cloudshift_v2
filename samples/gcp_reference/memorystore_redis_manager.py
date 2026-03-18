"""
GCP analogue of ElastiCacheManager — **Cloud Memorystore for Redis**.

ElastiCache node types map loosely to **memory_size_gb** + **tier** (BASIC /
STANDARD_HA), not 1:1 instance classes.

Broken transforms (boto3 + functions_framework + GoogleCloudError) are not
valid GCP code.

Requires: pip install google-cloud-redis google-api-core
"""
from __future__ import annotations

from google.api_core import exceptions as gcp_exceptions
from google.cloud.redis_v1 import CloudRedisClient
from google.cloud.redis_v1.types import Instance


class MemorystoreRedisManager:
    """Create / list / delete Memorystore for Redis instances."""

    def __init__(self, project_id: str, region: str):
        self.project_id = project_id
        self.region = region
        self._parent = f"projects/{project_id}/locations/{region}"
        self._client = CloudRedisClient()

    def _instance_name(self, instance_id: str) -> str:
        return f"{self._parent}/instances/{instance_id}"

    def create_cache_cluster(
        self,
        cluster_id: str,
        node_type: str | None = None,
        num_cache_nodes: int = 1,
        engine: str = "redis",
        *,
        memory_size_gb: int = 1,
        tier_ha: bool = False,
        authorized_network: str | None = None,
    ) -> Instance | None:
        """
        `node_type` / `num_cache_nodes` / `engine` are AWS-shaped; on GCP use
        memory_size_gb (1–300) and tier. Memcached is a separate Memorystore
        product — this class is Redis only.
        """
        _ = (node_type, num_cache_nodes, engine)
        try:
            tier = Instance.Tier.STANDARD_HA if tier_ha else Instance.Tier.BASIC
            instance = Instance(
                tier=tier,
                memory_size_gb=memory_size_gb,
                redis_version="REDIS_7_0",
            )
            if authorized_network:
                instance.authorized_network = authorized_network
            op = self._client.create_instance(
                parent=self._parent,
                instance_id=cluster_id,
                instance=instance,
            )
            result = op.result()
            print(f"Memorystore Redis instance {cluster_id} created")
            return result
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error creating Memorystore instance: {e}")
            return None

    def describe_cache_clusters(self, cluster_id: str | None = None) -> list[Instance]:
        try:
            if cluster_id:
                inst = self._client.get_instance(
                    name=self._instance_name(cluster_id)
                )
                return [inst]
            out: list[Instance] = []
            for inst in self._client.list_instances(parent=self._parent):
                out.append(inst)
            return out
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error describing Memorystore instances: {e}")
            return []

    def delete_cache_cluster(self, cluster_id: str) -> bool:
        try:
            op = self._client.delete_instance(name=self._instance_name(cluster_id))
            op.result()
            print(f"Memorystore instance {cluster_id} deleted")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error deleting Memorystore instance: {e}")
            return False
