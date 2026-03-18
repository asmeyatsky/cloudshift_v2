"""
GCP-oriented analogue of ECS Fargate-style **ECSManager** using **Cloud Run Jobs**.

Mapping (approximate):
  ECS cluster              -> region + project (no cluster resource)
  task definition          -> **Job** (container image, CPU, memory)
  run_task                 -> **run_job** (creates an execution)
  list_tasks               -> **list_executions** on that Job

ECS task networking (subnets / security groups) maps to **VPC connector** or
default public egress — pass `vpc_connector` when you need private VPC.

Broken boto3 + functions_framework output is not valid GCP code.

Requires: pip install google-cloud-run google-api-core
"""
from __future__ import annotations

import re
from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud import run_v2


def _job_id_safe(family: str) -> str:
    s = re.sub(r"[^a-z0-9-]", "-", family.lower()).strip("-")
    return (s or "job")[:63]


class CloudRunJobsManager:
    """Cloud Run Jobs — similar responsibilities to ECS Fargate in the AWS sample."""

    def __init__(self, project_id: str, region: str):
        self.project_id = project_id
        self.region = region
        self._parent = f"projects/{project_id}/locations/{region}"
        self._jobs = run_v2.JobsClient()
        self._executions = run_v2.ExecutionsClient()
        self._cluster_label: str | None = None
        self._last_job_id: str | None = None

    def create_cluster(self, cluster_name: str) -> dict[str, Any]:
        """Cloud Run has no ECS-like cluster; record a logical name for logging."""
        self._cluster_label = cluster_name
        print(f"Logical environment {cluster_name!r} (region={self.region})")
        return {"clusterName": cluster_name, "region": self.region}

    def register_task_definition(
        self,
        family: str,
        container_definitions: list[dict[str, Any]],
        cpu: str = "256",
        memory: str = "512",
        *,
        service_account: str | None = None,
        vpc_connector: str | None = None,
    ) -> run_v2.Job | None:
        """
        Registers a **Job** (job_id derived from `family`). Uses the first
        container's **image** from ECS-style `container_definitions`.
        """
        try:
            if not container_definitions:
                raise ValueError("container_definitions must not be empty")
            image = str(container_definitions[0].get("image") or "").strip()
            if not image:
                raise ValueError("container_definitions[0] needs 'image'")

            # ECS CPU units (256 ≈ 0.25 vCPU) → Cloud Run CPU string
            try:
                cu = int(cpu)
                cpu_str = str(max(1, cu // 1024)) if cu >= 1024 else "1"
            except ValueError:
                cpu_str = "1"
            mem_str = f"{memory}Mi" if memory.isdigit() else memory

            container = run_v2.Container(
                image=image,
                resources=run_v2.ResourceRequirements(
                    limits={"cpu": cpu_str, "memory": mem_str},
                ),
            )
            task_t = run_v2.TaskTemplate(containers=[container])
            if service_account:
                task_t.service_account = service_account
            if vpc_connector:
                task_t.vpc_access = run_v2.VpcAccess(connector=vpc_connector)

            job = run_v2.Job(
                template=run_v2.ExecutionTemplate(template=task_t),
            )
            job_id = _job_id_safe(family)
            op = self._jobs.create_job(
                parent=self._parent,
                job=job,
                job_id=job_id,
            )
            created = op.result()
            self._last_job_id = job_id
            print(f"Cloud Run Job {job_id!r} registered (from family {family!r})")
            return created
        except (gcp_exceptions.GoogleAPICallError, ValueError) as e:
            print(f"Error registering job: {e}")
            return None

    def run_task(
        self,
        cluster_name: str,
        task_definition: str,
        subnets: list[str] | None = None,
        security_groups: list[str] | None = None,
    ) -> run_v2.Execution | None:
        """
        Start a Job execution. `task_definition` is the **job id** (ECS family).
        `subnets` / `security_groups` are ignored unless the Job uses **vpc_access**.
        """
        _ = (cluster_name, subnets, security_groups)
        job_id = _job_id_safe(task_definition)
        name = f"{self._parent}/jobs/{job_id}"
        try:
            op = self._jobs.run_job(name=name)
            exec_name = op.result().name
            print(f"Execution started: {exec_name}")
            return self._executions.get_execution(name=exec_name)
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error running job: {e}")
            return None

    def list_tasks(self, cluster_name: str) -> list[str]:
        """List execution names for the **last registered** Job in this manager."""
        _ = cluster_name
        if not self._last_job_id:
            print("No Job registered yet; call register_task_definition first")
            return []
        parent = f"{self._parent}/jobs/{self._last_job_id}"
        try:
            out = []
            for ex in self._executions.list_executions(parent=parent):
                out.append(ex.name)
            return out
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error listing executions: {e}")
            return []
