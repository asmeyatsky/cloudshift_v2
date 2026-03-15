import boto3
import json

sqs = boto3.client('sqs')
sns = boto3.client('sns')

QUEUE_URL = 'https://sqs.us-east-1.amazonaws.com/123456789/processing-queue'
TOPIC_ARN = 'arn:aws:sns:us-east-1:123456789:notifications'

def send_processing_job(job_data: dict) -> str:
    """Send a job to the processing queue."""
    response = sqs.send_message(
        QueueUrl=QUEUE_URL,
        MessageBody=json.dumps(job_data),
        MessageAttributes={
            'JobType': {'DataType': 'String', 'StringValue': job_data.get('type', 'default')}
        }
    )
    return response['MessageId']

def receive_jobs(max_messages: int = 10) -> list:
    """Receive jobs from the processing queue."""
    response = sqs.receive_message(
        QueueUrl=QUEUE_URL,
        MaxNumberOfMessages=max_messages,
        WaitTimeSeconds=20
    )
    return response.get('Messages', [])

def publish_notification(subject: str, message: str) -> str:
    """Publish a notification via SNS."""
    response = sns.publish(
        TopicArn=TOPIC_ARN,
        Subject=subject,
        Message=message
    )
    return response['MessageId']

def send_event(stream_name: str, data: dict, partition_key: str) -> None:
    """Send an event to a Kinesis stream."""
    kinesis = boto3.client('kinesis')
    kinesis.put_record(
        StreamName=stream_name,
        Data=json.dumps(data).encode('utf-8'),
        PartitionKey=partition_key
    )
