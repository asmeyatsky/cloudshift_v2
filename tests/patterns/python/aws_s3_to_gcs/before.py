import boto3

s3 = boto3.client('s3')

def upload_document(bucket_name, key, content):
    s3.put_object(
        Bucket=bucket_name,
        Key=key,
        Body=content
    )

def download_document(bucket_name, key):
    response = s3.get_object(Bucket=bucket_name, Key=key)
    return response['Body'].read()

def list_documents(bucket_name, prefix):
    response = s3.list_objects_v2(Bucket=bucket_name, Prefix=prefix)
    return [obj['Key'] for obj in response.get('Contents', [])]
