import json
import logging
import boto3

logger = logging.getLogger()
logger.setLevel(logging.INFO)

s3 = boto3.client('s3')


def lambda_handler(event, context):
    """Process S3 events and transform uploaded documents."""
    for record in event['Records']:
        bucket = record['s3']['bucket']['name']
        key = record['s3']['object']['key']

        logger.info(f"Processing s3://{bucket}/{key}")

        response = s3.get_object(Bucket=bucket, Key=key)
        body = response['Body'].read().decode('utf-8')

        result = process_document(body)

        s3.put_object(
            Bucket=bucket,
            Key=f"processed/{key}",
            Body=json.dumps(result),
            ContentType='application/json',
        )

    return {
        'statusCode': 200,
        'body': json.dumps({'processed': len(event['Records'])}),
    }


def process_document(content):
    """Transform document content."""
    lines = content.strip().split('\n')
    return {
        'line_count': len(lines),
        'char_count': len(content),
        'preview': lines[0] if lines else '',
    }
