export type CloudProvider = 'aws' | 'azure'

export type CloudExample = {
  id: string
  title: string
  tag: 'AWS' | 'Azure'
  tagColor: string
  language: string
  cloud: CloudProvider
  code: string
}

const AWS_COLOR = 'bg-orange-500/10 text-orange-400 border-orange-500/20'
const AZURE_COLOR = 'bg-sky-500/10 text-sky-400 border-sky-500/20'

/** Top AWS services — Python / boto3 style snippets */
export const AWS_EXAMPLES: CloudExample[] = [
  {
    id: 'aws-s3',
    title: 'S3 — Object storage',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `# AWS S3 = SOURCE for migration. Transform → GCP Cloud Storage patterns.
import boto3

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
    id: 'aws-dynamodb',
    title: 'DynamoDB — NoSQL',
    tag: 'AWS',
    tagColor: AWS_COLOR,
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
    id: 'aws-lambda',
    title: 'Lambda — Serverless handler',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import json
import boto3

def lambda_handler(event, context):
    dynamodb = boto3.resource('dynamodb')
    table = dynamodb.Table('events')
    table.put_item(Item={'id': event['id'], 'payload': json.dumps(event)})
    return {'statusCode': 200, 'body': json.dumps({'ok': True})}`,
  },
  {
    id: 'aws-sqs',
    title: 'SQS — Message queues',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import boto3

sqs = boto3.client('sqs')
QUEUE_URL = 'https://sqs.us-east-1.amazonaws.com/123456789/my-queue'

def send_message(body):
    return sqs.send_message(QueueUrl=QUEUE_URL, MessageBody=body)

def receive_messages():
    resp = sqs.receive_message(QueueUrl=QUEUE_URL, MaxNumberOfMessages=10)
    return resp.get('Messages', [])`,
  },
  {
    id: 'aws-sns',
    title: 'SNS — Pub/sub notifications',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import boto3

sns = boto3.client('sns')
TOPIC_ARN = 'arn:aws:sns:us-east-1:123456789:alerts'

def publish_alert(subject, message):
    return sns.publish(
        TopicArn=TOPIC_ARN,
        Subject=subject,
        Message=message,
    )`,
  },
  {
    id: 'aws-secrets',
    title: 'Secrets Manager',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import boto3
import json

sm = boto3.client('secretsmanager')

def get_api_key(secret_id):
    resp = sm.get_secret_value(SecretId=secret_id)
    return json.loads(resp['SecretString'])['api_key']

def rotate_secret(secret_id):
    sm.rotate_secret(SecretId=secret_id)`,
  },
  {
    id: 'aws-kms',
    title: 'KMS — Encryption keys',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import boto3

kms = boto3.client('kms')
KEY_ID = 'alias/my-app-key'

def encrypt_data(plaintext: bytes):
    return kms.encrypt(KeyId=KEY_ID, Plaintext=plaintext)['CiphertextBlob']

def decrypt_data(blob: bytes):
    return kms.decrypt(CiphertextBlob=blob)['Plaintext']`,
  },
  {
    id: 'aws-rds-data',
    title: 'RDS — Data API (SQL)',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import boto3

rds = boto3.client('rds-data')
RESOURCE_ARN = 'arn:aws:rds:us-east-1:123:cluster:mydb'
SECRET_ARN = 'arn:aws:secretsmanager:us-east-1:123:secret:rds'

def run_query(sql: str):
    return rds.execute_statement(
        resourceArn=RESOURCE_ARN,
        secretArn=SECRET_ARN,
        database='app',
        sql=sql,
    )`,
  },
  {
    id: 'aws-ses',
    title: 'SES — Email',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import boto3

ses = boto3.client('ses', region_name='us-east-1')

def send_email(to_addr, subject, body_html):
    return ses.send_email(
        Source='noreply@example.com',
        Destination={'ToAddresses': [to_addr]},
        Message={
            'Subject': {'Data': subject},
            'Body': {'Html': {'Data': body_html}},
        },
    )`,
  },
  {
    id: 'aws-cloudwatch',
    title: 'CloudWatch Logs',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import boto3

logs = boto3.client('logs')
LOG_GROUP = '/app/prod'

def write_log(message):
    logs.put_log_events(
        logGroupName=LOG_GROUP,
        logStreamName='app-stream',
        logEvents=[{'timestamp': 0, 'message': message}],
    )`,
  },
  {
    id: 'aws-ssm',
    title: 'Systems Manager — Parameters',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import boto3

ssm = boto3.client('ssm')

def get_param(name: str):
    r = ssm.get_parameter(Name=name, WithDecryption=True)
    return r['Parameter']['Value']

def put_param(name: str, value: str):
    ssm.put_parameter(Name=name, Value=value, Type='SecureString', Overwrite=True)`,
  },
  {
    id: 'aws-eventbridge',
    title: 'EventBridge — Event bus',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import boto3
import json

events = boto3.client('events')

def publish_event(detail: dict):
    return events.put_events(
        Entries=[
            {
                'Source': 'my.app',
                'DetailType': 'OrderPlaced',
                'Detail': json.dumps(detail),
            }
        ]
    )`,
  },
  {
    id: 'aws-stepfunctions',
    title: 'Step Functions',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import boto3
import json

sfn = boto3.client('stepfunctions')
STATE_MACHINE_ARN = 'arn:aws:states:us-east-1:123:stateMachine:pipeline'

def start_workflow(order_id: str):
    return sfn.start_execution(
        stateMachineArn=STATE_MACHINE_ARN,
        input=json.dumps({'orderId': order_id}),
    )`,
  },
  {
    id: 'aws-cognito',
    title: 'Cognito — Auth',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import boto3

cognito = boto3.client('cognito-idp')
POOL_ID = 'us-east-1_XXXXX'

def get_user(username):
    return cognito.admin_get_user(UserPoolId=POOL_ID, Username=username)

def set_user_password(username, password):
    cognito.admin_set_user_password(
        UserPoolId=POOL_ID, Username=username, Password=password, Permanent=True
    )`,
  },
  {
    id: 'aws-ecs',
    title: 'ECS — Containers',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import boto3

ecs = boto3.client('ecs')

def run_task(cluster: str, task_def: str):
    return ecs.run_task(
        cluster=cluster,
        taskDefinition=task_def,
        launchType='FARGATE',
        networkConfiguration={
            'awsvpcConfiguration': {
                'subnets': ['subnet-abc'],
                'securityGroups': ['sg-123'],
                'assignPublicIp': 'ENABLED',
            }
        },
    )`,
  },
  {
    id: 'aws-ec2',
    title: 'EC2 — Compute',
    tag: 'AWS',
    tagColor: AWS_COLOR,
    language: 'python',
    cloud: 'aws',
    code: `import boto3

ec2 = boto3.client('ec2')

def list_running_instances():
    r = ec2.describe_instances(
        Filters=[{'Name': 'instance-state-name', 'Values': ['running']}]
    )
    return r['Reservations']

def stop_instance(instance_id: str):
    ec2.stop_instances(InstanceIds=[instance_id])`,
  },
]

/** Top Azure services — Python SDK style snippets */
export const AZURE_EXAMPLES: CloudExample[] = [
  {
    id: 'az-blob',
    title: 'Blob Storage',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `# Azure Blob = SOURCE code (system you migrate FROM). CloudShift Transform → GCP.
# This is not GCS: on GCP you’d use google.cloud.storage + ADC / service account JSON.
from azure.storage.blob import BlobServiceClient

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
  {
    id: 'az-cosmos',
    title: 'Cosmos DB',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `from azure.cosmos import CosmosClient

client = CosmosClient(url, credential=key)
db = client.get_database_client("app")
container = db.get_container_client("users")

def get_user(user_id):
    return container.read_item(item=user_id, partition_key=user_id)

def upsert_user(doc):
    container.upsert_item(doc)`,
  },
  {
    id: 'az-functions',
    title: 'Azure Functions',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `import azure.functions as func
import json

app = func.FunctionApp()

@app.function_name("ProcessOrder")
@app.service_bus_queue_trigger(arg_name="msg", queue_name="orders", connection="SERVICEBUS")
def process_order(msg: func.ServiceBusMessage):
    body = json.loads(msg.get_body().decode())
    # process order
    return None`,
  },
  {
    id: 'az-servicebus',
    title: 'Service Bus',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `from azure.servicebus import ServiceBusClient, ServiceBusMessage

conn_str = "Endpoint=sb://..."
QUEUE = "tasks"

def send_message(body: str):
    with ServiceBusClient.from_connection_string(conn_str) as client:
        with client.get_queue_sender(QUEUE) as sender:
            sender.send_messages(ServiceBusMessage(body))`,
  },
  {
    id: 'az-keyvault',
    title: 'Key Vault — Secrets',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `from azure.keyvault.secrets import SecretClient
from azure.identity import DefaultAzureCredential

vault_url = "https://myvault.vault.azure.net/"
client = SecretClient(vault_url=vault_url, credential=DefaultAzureCredential())

def get_secret(name: str):
    return client.get_secret(name).value`,
  },
  {
    id: 'az-table',
    title: 'Table Storage',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `from azure.data.tables import TableServiceClient

conn_str = "DefaultEndpointsProtocol=https;AccountName=...;AccountKey=...;EndpointSuffix=core.windows.net"
service = TableServiceClient.from_connection_string(conn_str)
table = service.get_table_client("customers")

def upsert_entity(entity: dict):
    table.upsert_entity(entity)

def query_partition(pk: str):
    return table.query_entities(f"PartitionKey eq '{pk}'")`,
  },
  {
    id: 'az-queue',
    title: 'Queue Storage',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `from azure.storage.queue import QueueServiceClient

conn_str = "DefaultEndpointsProtocol=https;AccountName=...;AccountKey=...;EndpointSuffix=core.windows.net"
queue_service = QueueServiceClient.from_connection_string(conn_str)
queue = queue_service.get_queue_client("jobs")

def enqueue(msg: str):
    queue.send_message(msg)

def dequeue():
    msgs = queue.receive_messages(messages_per_page=1)
    for m in msgs:
        queue.delete_message(m)
        return m.content`,
  },
  {
    id: 'az-sql',
    title: 'Azure SQL Database',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `import pyodbc

conn_str = (
    "Driver={ODBC Driver 18 for SQL Server};"
    "Server=myserver.database.windows.net;"
    "Database=mydb;Uid=user;Pwd=***;Encrypt=yes;"
)

def fetch_users():
    with pyodbc.connect(conn_str) as conn:
        cur = conn.cursor()
        cur.execute("SELECT id, name FROM users WHERE active = 1")
        return cur.fetchall()`,
  },
  {
    id: 'az-eventhub',
    title: 'Event Hubs',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `from azure.eventhub import EventHubProducerClient, EventData

conn_str = "Endpoint=sb://..."
EVENT_HUB = "telemetry"

def send_events(events: list[bytes]):
    producer = EventHubProducerClient.from_connection_string(
        conn_str, eventhub_name=EVENT_HUB
    )
    with producer:
        batch = producer.create_batch()
        for e in events:
            batch.add(EventData(e))
        producer.send_batch(batch)`,
  },
  {
    id: 'az-redis',
    title: 'Azure Cache for Redis',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `import redis

r = redis.Redis(host='mycache.redis.cache.windows.net', port=6380, password=key, ssl=True)

def cache_get(k: str):
    return r.get(k)

def cache_set(k: str, v: str, ttl=3600):
    r.setex(k, ttl, v)`,
  },
  {
    id: 'az-appconfig',
    title: 'App Configuration',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `from azure.appconfiguration import AzureAppConfigurationClient
from azure.core.credentials import AzureKeyCredential

endpoint = "https://myconfig.azconfig.io"
client = AzureAppConfigurationClient(endpoint, AzureKeyCredential("read-key"))

def get_setting(key: str):
    return client.get_configuration_setting(key=key, label="prod")`,
  },
  {
    id: 'az-files',
    title: 'Files — Azure Files',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `from azure.storage.fileshare import ShareServiceClient

service = ShareServiceClient.from_connection_string(conn_str)
share = service.get_share_client("shared-data")
directory = share.get_directory_client("reports")

def upload_file(name: str, data: bytes):
    f = directory.get_file_client(name)
    f.upload_file(data)`,
  },
  {
    id: 'az-monitor',
    title: 'Monitor — Metrics',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `from azure.monitor.query import MetricsQueryClient
from azure.identity import DefaultAzureCredential

client = MetricsQueryClient(DefaultAzureCredential())
resource_id = "/subscriptions/.../resourceGroups/rg/providers/Microsoft.Web/sites/myapp"

def cpu_avg(hours: int = 1):
    return client.query_resource(
        resource_id, metric_names=["CpuPercentage"], timespan=f"PT{hours}H"
    )`,
  },
  {
    id: 'az-acr',
    title: 'Container Registry',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `from azure.containerregistry import ContainerRegistryClient
from azure.identity import DefaultAzureCredential

endpoint = "myregistry.azurecr.io"
client = ContainerRegistryClient(endpoint, DefaultAzureCredential(), audience="https://management.azure.com")

def list_images(repo: str):
    return list(client.list_manifest_properties(repo))`,
  },
  {
    id: 'az-search',
    title: 'AI Search',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `from azure.search.documents import SearchClient
from azure.core.credentials import AzureKeyCredential

endpoint = "https://mysearch.search.windows.net"
client = SearchClient(endpoint, "products", AzureKeyCredential(admin_key))

def search_products(q: str):
    return list(client.search(search_text=q, top=20))`,
  },
  {
    id: 'az-arm',
    title: 'Resource Manager — Subscriptions',
    tag: 'Azure',
    tagColor: AZURE_COLOR,
    language: 'python',
    cloud: 'azure',
    code: `from azure.identity import DefaultAzureCredential
from azure.mgmt.resource import ResourceManagementClient

SUBSCRIPTION_ID = "00000000-0000-0000-0000-000000000000"

def list_resource_groups():
    client = ResourceManagementClient(DefaultAzureCredential(), SUBSCRIPTION_ID)
    return [g.name for g in client.resource_groups.list()]`,
  },
]

export const ALL_EXAMPLES: CloudExample[] = [...AWS_EXAMPLES, ...AZURE_EXAMPLES]

/** @deprecated use ALL_EXAMPLES */
export const EXAMPLES = ALL_EXAMPLES
