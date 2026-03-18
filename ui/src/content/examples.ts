export const EXAMPLES = [
  {
    title: 'S3 to Cloud Storage',
    tag: 'AWS',
    tagColor: 'bg-orange-500/10 text-orange-400 border-orange-500/20',
    language: 'python',
    cloud: 'aws',
    code: `import boto3

s3 = boto3.client('s3')

def upload_file(bucket, key, data):
    s3.put_object(Bucket=bucket, Key=key, Body=data)

def download_file(bucket, key):
    response = s3.get_object(Bucket=bucket, Key=key)
    return response['Body'].read()

def list_files(bucket, prefix=''):
    response = s3.list_objects_v2(Bucket=bucket, Prefix=prefix)
    return [obj['Key'] for obj in response.get('Contents', [])]`,
  },
  {
    title: 'DynamoDB to Firestore',
    tag: 'AWS',
    tagColor: 'bg-orange-500/10 text-orange-400 border-orange-500/20',
    language: 'python',
    cloud: 'aws',
    code: `import boto3

dynamodb = boto3.resource('dynamodb')
table = dynamodb.Table('users')

def get_user(user_id):
    response = table.get_item(Key={'id': user_id})
    return response.get('Item')

def put_user(user):
    table.put_item(Item=user)

def query_users(status):
    response = table.query(
        KeyConditionExpression='status = :s',
        ExpressionAttributeValues={':s': status}
    )
    return response['Items']`,
  },
  {
    title: 'Lambda to Cloud Functions',
    tag: 'AWS',
    tagColor: 'bg-orange-500/10 text-orange-400 border-orange-500/20',
    language: 'python',
    cloud: 'aws',
    code: `import json
import boto3

def lambda_handler(event, context):
    sqs = boto3.client('sqs')
    sqs.send_message(
        QueueUrl='https://sqs.us-east-1.amazonaws.com/123/my-queue',
        MessageBody=json.dumps(event)
    )
    return {
        'statusCode': 200,
        'body': json.dumps({'message': 'Processed successfully'})
    }`,
  },
  {
    title: 'Blob to Cloud Storage',
    tag: 'Azure',
    tagColor: 'bg-sky-500/10 text-sky-400 border-sky-500/20',
    language: 'python',
    cloud: 'azure',
    code: `from azure.storage.blob import BlobServiceClient

connection_string = "DefaultEndpointsProtocol=https;AccountName=..."
blob_service = BlobServiceClient.from_connection_string(connection_string)
container = blob_service.get_container_client("my-container")

def upload_blob(name, data):
    blob = container.get_blob_client(name)
    blob.upload_blob(data, overwrite=True)

def download_blob(name):
    blob = container.get_blob_client(name)
    return blob.download_blob().readall()`,
  },
] as const
