import json
import boto3

sqs = boto3.client('sqs')

QUEUE_URL = 'https://sqs.us-east-1.amazonaws.com/123456789012/my-queue'


def send_message(message_body, attributes=None):
    """Send a message to an SQS queue."""
    params = {
        'QueueUrl': QUEUE_URL,
        'MessageBody': json.dumps(message_body),
    }
    if attributes:
        params['MessageAttributes'] = {
            key: {'DataType': 'String', 'StringValue': str(val)}
            for key, val in attributes.items()
        }

    response = sqs.send_message(**params)
    return response['MessageId']


def send_batch(messages):
    """Send a batch of messages to an SQS queue."""
    entries = [
        {
            'Id': str(i),
            'MessageBody': json.dumps(msg),
        }
        for i, msg in enumerate(messages)
    ]

    response = sqs.send_message_batch(
        QueueUrl=QUEUE_URL,
        Entries=entries,
    )
    return response.get('Successful', [])


def receive_messages(max_messages=10, wait_time=20):
    """Receive messages from an SQS queue with long polling."""
    response = sqs.receive_message(
        QueueUrl=QUEUE_URL,
        MaxNumberOfMessages=max_messages,
        WaitTimeSeconds=wait_time,
        MessageAttributeNames=['All'],
    )

    messages = response.get('Messages', [])

    for message in messages:
        yield json.loads(message['Body']), message['ReceiptHandle']


def delete_message(receipt_handle):
    """Delete a processed message from the queue."""
    sqs.delete_message(
        QueueUrl=QUEUE_URL,
        ReceiptHandle=receipt_handle,
    )
