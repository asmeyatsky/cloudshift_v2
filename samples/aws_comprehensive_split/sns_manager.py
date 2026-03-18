"""Split from aws_comprehensive_example.py."""
import boto3
import json
from botocore.exceptions import ClientError

class SNSManager:
    """Manages SNS topics and subscriptions"""
    
    def __init__(self, region_name='us-east-1'):
        self.sns_client = boto3.client('sns', region_name=region_name)
    
    def create_topic(self, topic_name):
        """Create an SNS topic"""
        try:
            response = self.sns_client.create_topic(Name=topic_name)
            topic_arn = response['TopicArn']
            print(f"Topic {topic_name} created with ARN: {topic_arn}")
            return topic_arn
        except ClientError as e:
            print(f"Error creating topic: {e}")
            return None
    
    def publish_message(self, topic_arn, message, subject=None):
        """Publish a message to an SNS topic"""
        try:
            params = {
                'TopicArn': topic_arn,
                'Message': json.dumps(message) if isinstance(message, dict) else message
            }
            if subject:
                params['Subject'] = subject
            
            response = self.sns_client.publish(**params)
            message_id = response['MessageId']
            print(f"Message published with ID: {message_id}")
            return message_id
        except ClientError as e:
            print(f"Error publishing message: {e}")
            return None
    
    def subscribe(self, topic_arn, protocol, endpoint):
        """Subscribe to an SNS topic"""
        try:
            response = self.sns_client.subscribe(
                TopicArn=topic_arn,
                Protocol=protocol,
                Endpoint=endpoint
            )
            subscription_arn = response['SubscriptionArn']
            print(f"Subscribed {endpoint} to topic {topic_arn}")
            return subscription_arn
        except ClientError as e:
            print(f"Error subscribing: {e}")
            return None
    
    def list_topics(self):
        """List all SNS topics"""
        try:
            response = self.sns_client.list_topics()
            return response.get('Topics', [])
        except ClientError as e:
            print(f"Error listing topics: {e}")
            return []
    
    def delete_topic(self, topic_arn):
        """Delete an SNS topic"""
        try:
            self.sns_client.delete_topic(TopicArn=topic_arn)
            print(f"Topic {topic_arn} deleted")
            return True
        except ClientError as e:
            print(f"Error deleting topic: {e}")
            return False
