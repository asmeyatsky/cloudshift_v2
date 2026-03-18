"""
Step Functions–shaped API on **Google Cloud Workflows**.

ASL (Amazon States Language) is **not** Workflows YAML — you must author
**Workflows syntax** (`source_yaml`). `definition` (dict) is stored only in
the workflow **description** as a JSON hint for migration.

`roleArn` → **service account email** the workflow runs as (`service_account`).

Requires: pip install google-cloud-workflows google-api-core
"""
from __future__ import annotations

import json
import re
from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud.workflows import executions_v1
from google.cloud.workflows_v1 import WorkflowsClient
from google.cloud.workflows_v1.types import Workflow


def _wf_id(name: str) -> str:
    s = re.sub(r"[^a-z0-9-]", "-", name.lower()).strip("-")
    return (s or "workflow")[:63]


def _sa_email(role_arn: str) -> str:
    """Accept full SA resource name or email."""
    if "@" in role_arn and "/" not in role_arn.split("@")[0]:
        return role_arn
    if "@" in role_arn:
        parts = role_arn.split("/")
        for p in parts:
            if "@" in p and ".iam.gserviceaccount.com" in p:
                return p
    return role_arn


_DEFAULT_YAML = """main:
  params: [input]
  steps:
    - init:
        assign:
          - payload: ${input}
    - done:
        return: ${payload}
"""


class WorkflowsManager:
    def __init__(self, project_id: str, location: str):
        self.project_id = project_id
        self.location = location
        self._parent = f"projects/{project_id}/locations/{location}"
        self._wf = WorkflowsClient()
        self._exec = executions_v1.ExecutionsClient()

    def create_state_machine(
        self,
        name: str,
        definition: dict[str, Any],
        role_arn: str,
        *,
        source_yaml: str | None = None,
    ) -> dict[str, Any] | None:
        try:
            wid = _wf_id(name)
            contents = source_yaml.strip() if source_yaml else _DEFAULT_YAML
            wf = Workflow(
                description=json.dumps({"migrated_from_asl_hint": definition})[:500],
                source_contents=contents,
                service_account=_sa_email(role_arn),
            )
            op = self._wf.create_workflow(
                parent=self._parent,
                workflow=wf,
                workflow_id=wid,
            )
            created = op.result()
            arn = created.name
            print(f"Workflow {wid} created: {arn}")
            return {"stateMachineArn": arn, "name": wid}
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error creating workflow: {e}")
            return None

    def start_execution(
        self, state_machine_arn: str, input_data: dict[str, Any]
    ) -> dict[str, Any] | None:
        try:
            arg = json.dumps(input_data)
            ex = executions_v1.Execution(argument=arg)
            op = self._exec.create_execution(
                parent=state_machine_arn,
                execution=ex,
            )
            started = op.result()
            print(f"Execution started: {started.name}")
            return {
                "executionArn": started.name,
                "startDate": str(started.start_time),
            }
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error starting execution: {e}")
            return None

    def describe_execution(self, execution_arn: str) -> dict[str, Any] | None:
        try:
            ex = self._exec.get_execution(name=execution_arn)
            st = getattr(ex, "state", None)
            st_name = st.name if hasattr(st, "name") else str(st)
            return {
                "executionArn": ex.name,
                "status": st_name,
                "input": getattr(ex, "argument", "") or "",
                "output": getattr(ex, "result", "") or "",
            }
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error describing execution: {e}")
            return None
