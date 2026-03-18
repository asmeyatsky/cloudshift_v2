"""
GCP analogue of **AzureMonitorManager** — **Cloud Monitoring** (metrics + alerts).

Not Cloud Functions. See:
https://cloud.google.com/python/docs/reference/monitoring/latest

Requires: pip install google-cloud-monitoring google-api-core
"""
from __future__ import annotations

import time
from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud import monitoring_v3


class GCPMonitoringManager:
    def __init__(self, project_id: str):
        self._project = f"projects/{project_id}"
        self._metrics = monitoring_v3.MetricServiceClient()
        self._alerts = monitoring_v3.AlertPolicyServiceClient()

    def get_metrics(
        self,
        filter_expr: str,
        *,
        seconds_back: int = 3600,
    ) -> list[Any]:
        """Query time series with a Monitoring filter string."""
        try:
            now = time.time()
            interval = monitoring_v3.TimeInterval(
                {
                    "end_time": {"seconds": int(now)},
                    "start_time": {"seconds": int(now) - seconds_back},
                }
            )
            return list(
                self._metrics.list_time_series(
                    request={
                        "name": self._project,
                        "filter": filter_expr,
                        "interval": interval,
                    }
                )
            )
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error listing time series: {e}")
            return []

    def create_metric_alert(
        self,
        display_name: str,
        metric_filter: str,
        threshold: float,
    ) -> monitoring_v3.AlertPolicy | None:
        """Minimal threshold alert; tune notification channels in console."""
        try:
            cond = monitoring_v3.AlertPolicy.Condition(
                display_name=display_name,
                condition_threshold=monitoring_v3.AlertPolicy.Condition.MetricThreshold(
                    filter=metric_filter,
                    comparison=monitoring_v3.ComparisonType.COMPARISON_GT,
                    threshold_value=threshold,
                    duration={"seconds": 60},
                ),
            )
            policy = monitoring_v3.AlertPolicy(
                display_name=display_name,
                conditions=[cond],
                combiner=monitoring_v3.AlertPolicy.ConditionCombinerType.OR,
            )
            return self._alerts.create_alert_policy(
                name=self._project, alert_policy=policy
            )
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error creating alert policy: {e}")
            return None
