"""
GCP analogue of the AWS EC2Manager sample (Compute Engine).

Mapping:
  EC2 instance / AMI / type / SG  ->  GCE VM / image / machine type / firewall + network tags

Requires: pip install google-cloud-compute
"""
from __future__ import annotations

import uuid

from google.api_core import exceptions as gcp_exceptions
from google.cloud import compute_v1


class ComputeEngineManager:
    """Compute Engine VMs + VPC firewall rules (similar responsibilities to EC2Manager)."""

    def __init__(self, project_id: str, zone: str, network: str = "global/networks/default"):
        self.project_id = project_id
        self.zone = zone
        self.network = network
        self._instances = compute_v1.InstancesClient()
        self._firewalls = compute_v1.FirewallsClient()

    def create_instance(
        self,
        name: str,
        source_image: str,
        machine_type: str,
        *,
        network_tags: list[str] | None = None,
    ) -> compute_v1.Instance | None:
        """
        Create a VM. Example source_image:
          projects/debian-cloud/global/images/family/debian-12
        machine_type: e2-medium, n1-standard-1, etc.
        network_tags: VMs get these tags; firewall rules can target them (like SG membership).
        """
        try:
            disk = compute_v1.AttachedDisk(
                boot=True,
                auto_delete=True,
                initialize_params=compute_v1.AttachedDiskInitializeParams(
                    source_image=source_image,
                    disk_size_gb=10,
                ),
            )
            instance = compute_v1.Instance(
                name=name,
                machine_type=f"zones/{self.zone}/machineTypes/{machine_type}",
                disks=[disk],
                network_interfaces=[
                    compute_v1.NetworkInterface(network=self.network),
                ],
                tags=compute_v1.Tags(items=network_tags or ["http-server"]),
                labels={"environment": "production", "app": "myapp"},
            )
            op = self._instances.insert(
                project=self.project_id,
                zone=self.zone,
                instance_resource=instance,
            )
            op.result()
            return self._instances.get(
                project=self.project_id, zone=self.zone, instance=name
            )
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error creating instance: {e}")
            return None

    def list_instances(self, status_filter: str | None = "RUNNING") -> list[compute_v1.Instance]:
        try:
            it = self._instances.list(project=self.project_id, zone=self.zone)
            out = list(it)
            if status_filter:
                out = [i for i in out if i.status == status_filter]
            return out
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error listing instances: {e}")
            return []

    def delete_instance(self, instance_name: str) -> bool:
        try:
            op = self._instances.delete(
                project=self.project_id,
                zone=self.zone,
                instance=instance_name,
            )
            op.result()
            print(f"Instance {instance_name} deleted")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error deleting instance: {e}")
            return False

    def create_firewall_rule(
        self,
        rule_name: str,
        description: str,
        *,
        target_tags: list[str],
        protocol: str,
        ports: str,
        source_cidr: str = "0.0.0.0/0",
    ) -> str | None:
        """
        VPC ingress firewall (closest to EC2 security group + ingress rule).
        rule_name must be unique per project.
        """
        try:
            fw = compute_v1.Firewall(
                name=rule_name,
                network=self.network,
                description=description,
                direction="INGRESS",
                target_tags=target_tags,
                source_ranges=[source_cidr],
                allowed=[
                    compute_v1.Allowed(IPProtocol=protocol, ports=[ports]),
                ],
            )
            op = self._firewalls.insert(
                project=self.project_id,
                firewall_resource=fw,
            )
            op.result()
            print(f"Firewall {rule_name} created")
            return rule_name
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error creating firewall: {e}")
            return None

    def add_security_group_like_rule(
        self,
        base_name: str,
        target_tag: str,
        protocol: str,
        port: int,
        cidr: str,
    ) -> bool:
        """One ingress allow (like authorize_security_group_ingress)."""
        name = f"{base_name}-{uuid.uuid4().hex[:8]}"[:63]
        return (
            self.create_firewall_rule(
                name,
                description=f"Allow {protocol}/{port} from {cidr}",
                target_tags=[target_tag],
                protocol=protocol,
                ports=str(port),
                source_cidr=cidr,
            )
            is not None
        )
