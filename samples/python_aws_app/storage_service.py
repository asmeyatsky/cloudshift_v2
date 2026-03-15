import boto3
import json
from datetime import datetime, timedelta

s3 = boto3.client('s3')
BUCKET_NAME = 'my-data-pipeline'

def upload_json_document(key: str, data: dict) -> str:
    """Upload a JSON document to S3."""
    body = json.dumps(data, default=str)
    s3.put_object(
        Bucket=BUCKET_NAME,
        Key=f"documents/{key}.json",
        Body=body,
        ContentType='application/json'
    )
    return f"s3://{BUCKET_NAME}/documents/{key}.json"

def download_document(key: str) -> dict:
    """Download and parse a JSON document from S3."""
    response = s3.get_object(Bucket=BUCKET_NAME, Key=key)
    content = response['Body'].read().decode('utf-8')
    return json.loads(content)

def list_documents(prefix: str, max_keys: int = 100) -> list:
    """List documents matching a prefix."""
    response = s3.list_objects_v2(
        Bucket=BUCKET_NAME,
        Prefix=prefix,
        MaxKeys=max_keys
    )
    return [obj['Key'] for obj in response.get('Contents', [])]

def delete_document(key: str) -> None:
    """Delete a document from S3."""
    s3.delete_object(Bucket=BUCKET_NAME, Key=key)

def get_presigned_url(key: str, expiry_seconds: int = 3600) -> str:
    """Generate a presigned URL for temporary access."""
    url = s3.generate_presigned_url(
        'get_object',
        Params={'Bucket': BUCKET_NAME, 'Key': key},
        ExpiresIn=expiry_seconds
    )
    return url

def copy_document(source_key: str, dest_key: str) -> None:
    """Copy a document within the bucket."""
    s3.copy_object(
        Bucket=BUCKET_NAME,
        CopySource={'Bucket': BUCKET_NAME, 'Key': source_key},
        Key=dest_key
    )

def check_document_exists(key: str) -> bool:
    """Check if a document exists."""
    try:
        s3.head_object(Bucket=BUCKET_NAME, Key=key)
        return True
    except s3.exceptions.ClientError:
        return False
