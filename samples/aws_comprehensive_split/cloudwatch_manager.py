"""Split from aws_comprehensive_example.py."""
import boto3
from botocore.exceptions import ClientError

class CloudWatchManager:
    """Manages CloudWatch metrics and logs"""
    
    def __init__(self, region_name='us-east-1'):
        self.cloudwatch_client = boto3.client('cloudwatch', region_name=region_name)
        self.logs_client = boto3.client('logs', region_name=region_name)
    
    def put_metric(self, namespace, metric_name, value, unit='Count', dimensions=None):
        """Put a custom metric to CloudWatch"""
        try:
            params = {
                'Namespace': namespace,
                'MetricData': [{
                    'MetricName': metric_name,
                    'Value': value,
                    'Unit': unit
                }]
            }
            if dimensions:
                params['MetricData'][0]['Dimensions'] = dimensions
            
            self.cloudwatch_client.put_metric_data(**params)
            print(f"Metric {metric_name} published to CloudWatch")
            return True
        except ClientError as e:
            print(f"Error putting metric: {e}")
            return False
    
    def get_metric_statistics(self, namespace, metric_name, start_time, end_time, period, statistics):
        """Get metric statistics from CloudWatch"""
        try:
            response = self.cloudwatch_client.get_metric_statistics(
                Namespace=namespace,
                MetricName=metric_name,
                StartTime=start_time,
                EndTime=end_time,
                Period=period,
                Statistics=statistics
            )
            return response.get('Datapoints', [])
        except ClientError as e:
            print(f"Error getting metric statistics: {e}")
            return []
    
    def create_log_group(self, log_group_name):
        """Create a CloudWatch log group"""
        try:
            self.logs_client.create_log_group(logGroupName=log_group_name)
            print(f"Log group {log_group_name} created")
            return True
        except ClientError as e:
            print(f"Error creating log group: {e}")
            return False
    
    def put_log_events(self, log_group_name, log_stream_name, log_events):
        """Put log events to CloudWatch Logs"""
        try:
            self.logs_client.put_log_events(
                logGroupName=log_group_name,
                logStreamName=log_stream_name,
                logEvents=log_events
            )
            print(f"Log events written to {log_group_name}/{log_stream_name}")
            return True
        except ClientError as e:
            print(f"Error putting log events: {e}")
            return False
