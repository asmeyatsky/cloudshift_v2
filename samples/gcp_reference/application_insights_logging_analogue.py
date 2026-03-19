"""
GCP **Cloud Logging** — loose analogue for **Application Insights** custom
telemetry (events, traces, exceptions as structured logs).

**Custom metrics** in App Insights map more directly to **Cloud Monitoring**
custom metrics; here ``track_metric`` writes a structured log entry (see also
``gcp_monitoring_manager.py``).

Requires: ``pip install google-cloud-logging google-api-core``  
Auth: Application Default Credentials.
"""
from __future__ import annotations

import traceback
from typing import Any

from google.api_core import exceptions as gcp_exceptions
from google.cloud import logging as cloud_logging

_SEVERITY = {
    "Verbose": "DEBUG",
    "Information": "INFO",
    "Warning": "WARNING",
    "Error": "ERROR",
    "Critical": "CRITICAL",
}


class ApplicationInsightsLoggingAnalogue:
    """Same method names as ``ApplicationInsightsManager`` (Logging backend)."""

    def __init__(self, log_name: str = "application-insights-analogue") -> None:
        self._client = cloud_logging.Client()
        self._logger = self._client.logger(log_name)

    def track_event(self, event_name: str, properties: dict | None = None) -> bool:
        try:
            payload = {"telemetry": "event", "name": event_name, **(properties or {})}
            self._logger.log_struct(payload, severity="INFO")
            print(f"Event {event_name} tracked")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error tracking event: {e}")
            return False

    def track_exception(
        self, exception: BaseException, properties: dict | None = None
    ) -> bool:
        try:
            payload = {
                "telemetry": "exception",
                "type": type(exception).__name__,
                "message": str(exception),
                "traceback": traceback.format_exc(),
                **(properties or {}),
            }
            self._logger.log_struct(payload, severity="ERROR")
            print("Exception tracked")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error tracking exception: {e}")
            return False

    def track_metric(
        self, metric_name: str, value: float, properties: dict | None = None
    ) -> bool:
        try:
            payload = {
                "telemetry": "metric",
                "name": metric_name,
                "value": value,
                **(properties or {}),
            }
            self._logger.log_struct(payload, severity="INFO")
            print(f"Metric {metric_name} tracked")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error tracking metric: {e}")
            return False

    def track_trace(
        self,
        message: str,
        severity_level: str = "Information",
        properties: dict | None = None,
    ) -> bool:
        try:
            sev = _SEVERITY.get(severity_level, "INFO")
            payload = {"telemetry": "trace", "message": message, **(properties or {})}
            self._logger.log_struct(payload, severity=sev)
            print(f"Trace message tracked: {message}")
            return True
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error tracking trace: {e}")
            return False
