"""
GCP analogue of the AWS EKSManager sample (GKE — Google Kubernetes Engine).

Mapping:
  EKS cluster              -> GKE cluster (regional or zonal)
  roleArn + vpcConfig      -> service account / network on the cluster (see comments)
  describe_cluster         -> get_cluster
  list_clusters            -> list_clusters

Requires: pip install google-cloud-container google-api-core
Auth: Application Default Credentials.
"""
from __future__ import annotations

from google.api_core import exceptions as gcp_exceptions
from google.cloud import container_v1


class GKEClusterManager:
    """Manage GKE clusters (similar surface to EKSManager)."""

    def __init__(self, project_id: str, location: str):
        """
        location: region (e.g. us-central1) for regional clusters, or zone for zonal.
        """
        self.project_id = project_id
        self.location = location
        self._parent = f"projects/{project_id}/locations/{location}"
        self._client = container_v1.ClusterManagerClient()

    def create_cluster(
        self,
        cluster_name: str,
        *,
        initial_node_count: int = 1,
        machine_type: str = "e2-medium",
        network: str | None = None,
        subnetwork: str | None = None,
    ) -> container_v1.Cluster | None:
        """
        Create a GKE cluster. On AWS, roleArn/vpcConfig map roughly to:
          - IAM for the control plane / nodes -> GCP service accounts + IAM bindings
          - VPC subnets -> VPC network + subnetwork (default network if omitted)
        """
        try:
            node_cfg = container_v1.NodeConfig(machine_type=machine_type)
            cluster = container_v1.Cluster(
                name=cluster_name,
                initial_node_count=initial_node_count,
                node_config=node_cfg,
            )
            if network:
                cluster.network = network
            if subnetwork:
                cluster.subnetwork = subnetwork
            op = self._client.create_cluster(parent=self._parent, cluster=cluster)
            op.result()
            return self._client.get_cluster(
                name=f"{self._parent}/clusters/{cluster_name}"
            )
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error creating GKE cluster: {e}")
            return None

    def describe_cluster(self, cluster_name: str) -> container_v1.Cluster | None:
        try:
            return self._client.get_cluster(
                name=f"{self._parent}/clusters/{cluster_name}"
            )
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error describing cluster: {e}")
            return None

    def list_clusters(self) -> list[str]:
        try:
            resp = self._client.list_clusters(parent=self._parent)
            # API returns full resource names; return short names like EKS list
            names: list[str] = []
            for c in resp.clusters:
                names.append(c.name.split("/")[-1])
            return names
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error listing clusters: {e}")
            return []
