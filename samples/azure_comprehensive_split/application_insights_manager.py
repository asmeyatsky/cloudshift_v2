"""Azure Application Insights — telemetry.

``applicationinsights`` Python SDK (instrumentation key). Not HTTP handlers.

GCP analogue: ``gcp_reference/application_insights_logging_analogue.py``
(Cloud Logging); metrics → also consider Cloud Monitoring.

Requires: ``pip install applicationinsights``
"""


class ApplicationInsightsManager:
    """Manages Azure Application Insights telemetry"""

    def __init__(self, instrumentation_key):
        from applicationinsights import TelemetryClient
        self.telemetry_client = TelemetryClient(instrumentation_key)

    def track_event(self, event_name, properties=None):
        """Track a custom event"""
        try:
            self.telemetry_client.track_event(event_name, properties)
            self.telemetry_client.flush()
            print(f"Event {event_name} tracked")
            return True
        except Exception as e:
            print(f"Error tracking event: {e}")
            return False

    def track_exception(self, exception, properties=None):
        """Track an exception"""
        try:
            self.telemetry_client.track_exception(exception, properties)
            self.telemetry_client.flush()
            print("Exception tracked")
            return True
        except Exception as e:
            print(f"Error tracking exception: {e}")
            return False

    def track_metric(self, metric_name, value, properties=None):
        """Track a custom metric"""
        try:
            self.telemetry_client.track_metric(metric_name, value, properties)
            self.telemetry_client.flush()
            print(f"Metric {metric_name} tracked")
            return True
        except Exception as e:
            print(f"Error tracking metric: {e}")
            return False

    def track_trace(self, message, severity_level='Information', properties=None):
        """Track a trace message"""
        try:
            self.telemetry_client.track_trace(message, severity_level, properties)
            self.telemetry_client.flush()
            print(f"Trace message tracked: {message}")
            return True
        except Exception as e:
            print(f"Error tracking trace: {e}")
            return False
