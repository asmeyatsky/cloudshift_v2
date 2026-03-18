"""Azure Monitor — metrics and alerts."""
from azure.identity import DefaultAzureCredential
from azure.monitor import MonitorClient


class AzureMonitorManager:
    """Manages Azure Monitor metrics and logs"""

    def __init__(self, subscription_id):
        credential = DefaultAzureCredential()
        self.subscription_id = subscription_id
        self.monitor_client = MonitorClient(credential, subscription_id)

    def get_metrics(self, resource_id, metric_names, start_time, end_time):
        """Get metrics for a resource"""
        try:
            metrics_data = self.monitor_client.metrics.list(
                resource_id,
                timespan=f"{start_time}/{end_time}",
                interval='PT1H',
                metricnames=','.join(metric_names)
            )
            return metrics_data
        except Exception as e:
            print(f"Error getting metrics: {e}")
            return None

    def create_metric_alert(self, resource_group_name, alert_name, target_resource_id, metric_name, threshold):
        """Create a metric alert"""
        try:
            from azure.mgmt.monitor import MonitorManagementClient
            from azure.mgmt.monitor.models import MetricAlertResource, MetricAlertSingleResourceMultipleMetricCriteria, \
                MetricCriteria

            monitor_management_client = MonitorManagementClient(
                DefaultAzureCredential(), self.subscription_id
            )

            criteria = MetricCriteria(
                metric_name=metric_name,
                metric_namespace='Microsoft.Compute/virtualMachines',
                operator='GreaterThan',
                threshold=threshold,
                time_aggregation='Average'
            )

            alert_criteria = MetricAlertSingleResourceMultipleMetricCriteria(
                all_of=[criteria]
            )

            alert_resource = MetricAlertResource(
                location='global',
                description='Alert when CPU exceeds threshold',
                severity=2,
                enabled=True,
                scopes=[target_resource_id],
                evaluation_frequency='PT1M',
                window_size='PT5M',
                criteria=alert_criteria
            )

            alert = monitor_management_client.metric_alerts.create_or_update(
                resource_group_name,
                alert_name,
                alert_resource
            )
            print(f"Metric alert {alert_name} created")
            return alert
        except Exception as e:
            print(f"Error creating metric alert: {e}")
            return None
