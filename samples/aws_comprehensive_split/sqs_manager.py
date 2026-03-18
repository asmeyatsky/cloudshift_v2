"""Split from aws_comprehensive_example.py."""
import boto3
from botocore.exceptions import ClientError

class SQSManager:
    """Manages SQS queues"""
    
    def __init__(self, region_name='us-east-1'):
        self.sqs_client = boto3.client('sqs', region_name=region_name)
        self.sqs_resource = boto3.resource('sqs', region_name=region_name)
    
    def create_queue(self, queue_name, attributes=None):
        """Create an SQS queue"""
        try:
            if attributes is None:
                attributes = {
                    'DelaySeconds': '0',
                    'MessageRetentionPeriod': '86400'
                }
            
            response = self.sqs_client.create_queue(
                QueueName=queue_name,
                Attributes=attributes
            )
            queue_url = response['QueueUrl']
            print(f"Queue {queue_name} created with URL: {queue_url}")
            return queue_url
        except ClientError as e:
            print(f"Error creating queue: {e}")
            return None
    
    def send_message(self, queue_url, message_body, attributes=None):
        """Send a message to an SQS queue"""
        try:
            params = {'QueueUrl': queue_url, 'MessageBody': message_body}
            if attributes:
                params['MessageAttributes'] = attributes
            
            response = self.sqs_client.send_message(**params)
            message_id = response['MessageId']
            print(f"Message sent with ID: {message_id}")
            return message_id
        except ClientError as e:
            print(f"Error sending message: {e}")
            return None
    
    def receive_messages(self, queue_url, max_messages=1, wait_time=20):
        """Receive messages from an SQS queue"""
        try:
            response = self.sqs_client.receive_message(
                QueueUrl=queue_url,
                MaxNumberOfMessages=max_messages,
                WaitTimeSeconds=wait_time,
                MessageAttributeNames=['All']
            )
            return response.get('Messages', [])
        except ClientError as e:
            print(f"Error receiving messages: {e}")
            return []
    
    def delete_message(self, queue_url, receipt_handle):
        """Delete a message from an SQS queue"""
        try:
            self.sqs_client.delete_message(
                QueueUrl=queue_url,
                ReceiptHandle=receipt_handle
            )
            print("Message deleted successfully")
            return True
        except ClientError as e:
            print(f"Error deleting message: {e}")
            return False
    
    def get_queue_url(self, queue_name):
        """Get the URL of an SQS queue"""
        try:
            response = self.sqs_client.get_queue_url(QueueName=queue_name)
            return response['QueueUrl']
        except ClientError as e:
            print(f"Error getting queue URL: {e}")
            return None
    
    def purge_queue(self, queue_url):
        """Purge all messages from an SQS queue"""
        try:
            self.sqs_client.purge_queue(QueueUrl=queue_url)
            print("Queue purged successfully")
            return True
        except ClientError as e:
            print(f"Error purging queue: {e}")
            return False
