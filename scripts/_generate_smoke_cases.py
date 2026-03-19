#!/usr/bin/env python3
"""Regenerate scripts/data/pattern_smoke_cases.json. Run from repo root.

Snippets use an intermediate client variable (e.g. _c.method(...)) so tree-sitter
patterns with object: (identifier) match, and so boto3.client('x') spans do not
overlap method spans in the transform report.
"""
import json
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
root = REPO / "patterns" / "python"
OUT = REPO / "scripts" / "data" / "pattern_smoke_cases.json"


def aws_cli(svc: str, method_expr: str) -> str:
    return f"import boto3\n_c=boto3.client('{svc}')\n_c.{method_expr}"


def main() -> None:
    out = []

    def add(stem: str, source: str, code: str) -> None:
        t = tomllib.loads((root / f"{stem}.toml").read_text())
        out.append(
            {
                "stem": stem,
                "source": source,
                "code": code.strip() + "\n",
                "pattern_id": t["pattern"]["id"],
            }
        )

    for p in sorted(root.glob("aws_boto3_client_*.toml")):
        svc = p.stem.replace("aws_boto3_client_", "")
        if p.stem == "aws_boto3_client_dynamodb":
            add(p.stem, "aws", "import boto3\nboto3.resource('dynamodb')")
        else:
            add(p.stem, "aws", f"import boto3\nboto3.client('{svc}')")
    for p in sorted(root.glob("aws_boto3_resource_*.toml")):
        svc = p.stem.replace("aws_boto3_resource_", "")
        add(p.stem, "aws", f"import boto3\nboto3.resource('{svc}')")

    s3m = {
        "aws_s3_put_object": aws_cli("s3", "put_object(Bucket='b', Key='k', Body=b'x')"),
        "aws_s3_get_object": aws_cli("s3", "get_object(Bucket='b', Key='k')"),
        "aws_s3_list_objects": aws_cli("s3", "list_objects_v2(Bucket='b')"),
        "aws_s3_delete_object": aws_cli("s3", "delete_object(Bucket='b', Key='k')"),
        "aws_s3_create_bucket": aws_cli("s3", "create_bucket(Bucket='b')"),
        "aws_s3_copy_object": aws_cli("s3", "copy_object(Bucket='b', CopySource='a/k', Key='k2')"),
        "aws_s3_head_object": aws_cli("s3", "head_object(Bucket='b', Key='k')"),
        "aws_s3_presigned_url": aws_cli(
            "s3", "generate_presigned_url('get_object', Params={{'Bucket':'b','Key':'k'}})"
        ),
        "aws_s3_upload_file": aws_cli("s3", "upload_file('/tmp/x', 'b', 'k')"),
    }
    for k, v in s3m.items():
        add(k, "aws", v)

    add(
        "aws_dynamodb_put_item",
        "aws",
        "import boto3\n_t=boto3.resource('dynamodb').Table('t')\n_t.put_item(Item={'id':'1'})",
    )
    add(
        "aws_dynamodb_get_item",
        "aws",
        "import boto3\n_t=boto3.resource('dynamodb').Table('t')\n_t.get_item(Key={'id':'1'})",
    )
    add(
        "aws_dynamodb_delete_item",
        "aws",
        "import boto3\n_t=boto3.resource('dynamodb').Table('t')\n_t.delete_item(Key={'id':'1'})",
    )
    add(
        "aws_dynamodb_update_item",
        "aws",
        "import boto3\n_t=boto3.resource('dynamodb').Table('t')\n_t.update_item(Key={'id':'1'}, UpdateExpression='SET #a=:v', ExpressionAttributeNames={'#a':'n'}, ExpressionAttributeValues={':v':'x'})",
    )
    add(
        "aws_dynamodb_query",
        "aws",
        "import boto3\n_t=boto3.resource('dynamodb').Table('t')\n_t.query(KeyConditionExpression='id = :i', ExpressionAttributeValues={':i':'1'})",
    )
    add(
        "aws_dynamodb_scan",
        "aws",
        "import boto3\n_t=boto3.resource('dynamodb').Table('t')\n_t.scan()",
    )
    add(
        "aws_dynamodb_batch_write",
        "aws",
        aws_cli(
            "dynamodb",
            "batch_write_item(RequestItems={'t': [{'PutRequest': {'Item': {'id': {'S': '1'}}}}]})",
        ),
    )

    add("aws_sqs_send_message", "aws", aws_cli("sqs", "send_message(QueueUrl='https://q', MessageBody='m')"))
    add(
        "aws_sqs_receive_message",
        "aws",
        aws_cli("sqs", "receive_message(QueueUrl='https://q')"),
    )
    add("aws_sns_publish", "aws", aws_cli("sns", "publish(TopicArn='arn:aws:sns:x', Message='m')"))
    add(
        "aws_ses_send_email",
        "aws",
        aws_cli(
            "ses",
            "send_email(Source='a@b.c', Destination={'ToAddresses':['x@y.z']}, Message={'Subject':{'Data':'s'},'Body':{'Text':{'Data':'b'}}})",
        ),
    )
    add("aws_secrets_manager", "aws", aws_cli("secretsmanager", "get_secret_value(SecretId='s')"))
    add(
        "aws_kinesis_put_record",
        "aws",
        aws_cli("kinesis", "put_record(StreamName='s', Data=b'x', PartitionKey='p')"),
    )
    add(
        "aws_step_functions_start",
        "aws",
        aws_cli(
            "stepfunctions",
            "start_execution(stateMachineArn='arn:aws:states:x', name='n', input='{}')",
        ),
    )
    add(
        "aws_eventbridge_put_events",
        "aws",
        aws_cli(
            "events",
            "put_events(Entries=[{'Source':'s','DetailType':'t','Detail':'{}'}])",
        ),
    )
    add("aws_glue_start_job", "aws", aws_cli("glue", "start_job_run(JobName='j')"))
    add(
        "aws_athena_query",
        "aws",
        aws_cli(
            "athena",
            "start_query_execution(QueryString='SELECT 1', ResultConfiguration={'OutputLocation':'s3://b/'})",
        ),
    )
    add("aws_rds_connection", "aws", "import psycopg2\npsycopg2.connect(host='x.rds.amazonaws.com', database='d', user='u', password='p')")
    add(
        "aws_redshift_query",
        "aws",
        aws_cli(
            "redshift-data",
            "execute_statement(ClusterIdentifier='c', Database='d', DbUser='u', Sql='SELECT 1')",
        ),
    )
    add(
        "aws_sts_assume_role",
        "aws",
        aws_cli("sts", "assume_role(RoleArn='arn:aws:iam::1:role/r', RoleSessionName='s')"),
    )
    add("aws_ecr_get_auth", "aws", aws_cli("ecr", "get_authorization_token()"))
    add(
        "aws_cloudwatch_put_metric",
        "aws",
        aws_cli(
            "cloudwatch",
            "put_metric_data(Namespace='n', MetricData=[{'MetricName':'m','Value':1.0}])",
        ),
    )
    add(
        "aws_cloudwatch_get_metric",
        "aws",
        """import boto3
from datetime import datetime, timedelta
now = datetime.utcnow()
_c=boto3.client('cloudwatch')
_c.get_metric_data(MetricDataQueries=[{'Id':'x','MetricStat':{'Metric':{'Namespace':'AWS/EC2','MetricName':'CPUUtilization'},'Period':60,'Stat':'Average'}}], StartTime=now-timedelta(hours=1), EndTime=now)""",
    )
    add(
        "aws_comprehend_detect_sentiment",
        "aws",
        aws_cli("comprehend", "detect_sentiment(Text='hello', LanguageCode='en')"),
    )
    add(
        "aws_translate_text",
        "aws",
        aws_cli("translate", "translate_text(Text='hi', SourceLanguageCode='en', TargetLanguageCode='es')"),
    )
    add(
        "aws_textract_detect",
        "aws",
        aws_cli(
            "textract",
            "detect_document_text(Document={'S3Object':{'Bucket':'b','Name':'n'}})",
        ),
    )
    add(
        "aws_rekognition_detect_labels",
        "aws",
        aws_cli(
            "rekognition",
            "detect_labels(Image={'S3Object':{'Bucket':'b','Name':'n'}})",
        ),
    )
    add(
        "aws_polly_synthesize",
        "aws",
        aws_cli("polly", "synthesize_speech(Text='x', OutputFormat='mp3', VoiceId='Joanna')"),
    )
    add(
        "aws_sagemaker_create_endpoint",
        "aws",
        aws_cli("sagemaker", "create_endpoint(EndpointName='e', EndpointConfigName='c')"),
    )
    add(
        "aws_bedrock_invoke_model",
        "aws",
        aws_cli("bedrock-runtime", "invoke_model(modelId='m', body=b'{}')"),
    )
    add("aws_lambda_handler", "aws", "def lambda_handler(event, context):\n    return 0\n")
    add(
        "aws_ec2_terminate_instances",
        "aws",
        "from botocore.exceptions import ClientError\nimport boto3\n_ec2=boto3.client('ec2')\n_ec2.terminate_instances(InstanceIds=['i-1'])",
    )
    add(
        "aws_route53_list_hosted_zones",
        "aws",
        "from botocore.exceptions import ClientError\nimport boto3\n_r=boto3.client('route53')\n_r.list_hosted_zones()",
    )

    add(
        "azure_blob_upload",
        "azure",
        "from azure.storage.blob import BlobServiceClient\n_bs=BlobServiceClient.from_connection_string('x')\n_bc=_bs.get_blob_client('c','b')\n_bc.upload_blob(b'd', overwrite=True)",
    )
    add(
        "azure_blob_download",
        "azure",
        "from azure.storage.blob import BlobServiceClient\n_bs=BlobServiceClient.from_connection_string('x')\n_bc=_bs.get_blob_client('c','b')\n_bc.download_blob()",
    )
    add(
        "azure_storage_queue_send",
        "azure",
        "from azure.storage.queue import QueueServiceClient\n_qs=QueueServiceClient.from_connection_string('x')\n_qc=_qs.get_queue_client('q')\n_qc.send_message('m')",
    )
    add(
        "azure_servicebus_send",
        "azure",
        "from azure.servicebus import ServiceBusClient, ServiceBusMessage\n_sb=ServiceBusClient.from_connection_string('x')\n_snd=_sb.get_queue_sender('q')\n_snd.send_messages(ServiceBusMessage('m'))",
    )
    add(
        "azure_keyvault_get_secret",
        "azure",
        "from azure.identity import DefaultAzureCredential\nfrom azure.keyvault.secrets import SecretClient\n_sc=SecretClient('https://v.vault.azure.net', DefaultAzureCredential())\n_sc.get_secret('s')",
    )
    add(
        "azure_cosmosdb_create_item",
        "azure",
        "from azure.cosmos import CosmosClient\n_cc=CosmosClient('https://x.documents.azure.com:443','k').get_database_client('d').get_container_client('c')\n_cc.create_item(body={'id':'1'})",
    )
    add(
        "azure_cognitive_text_analytics",
        "azure",
        "from azure.ai.textanalytics import TextAnalyticsClient\nfrom azure.core.credentials import AzureKeyCredential\n_c=TextAnalyticsClient('https://x.cognitiveservices.azure.com/', AzureKeyCredential('k'))\n_c.analyze_sentiment([{'id':'1','text':'hi'}])",
    )
    add(
        "azure_functions_handler",
        "azure",
        "import azure.functions as func\ndef main(req: func.HttpRequest) -> func.HttpResponse:\n    return func.HttpResponse('ok')\n",
    )
    add(
        "azure_openai_chat",
        "azure",
        "from openai import AzureOpenAI\n_c=AzureOpenAI(api_key='k',api_version='2024-02-01',azure_endpoint='https://x.openai.azure.com/')\n_c.chat.completions.create(model='gpt-4',messages=[{'role':'user','content':'hi'}])",
    )
    add(
        "azure_ml_invoke",
        "azure",
        """import azure.ai.ml
from azure.identity import DefaultAzureCredential
class MLClient:
    def invoke(self, endpoint_name=None, request_file=None):
        pass
c = MLClient()
c.invoke(endpoint_name='e', request_file='f.json')""",
    )
    add(
        "azure_eventgrid_publisher_new",
        "azure",
        "from azure.eventgrid import EventGridPublisherClient\nEventGridPublisherClient('https://x', 'key')",
    )
    add(
        "azure_tables_from_connection",
        "azure",
        "from azure.data.tables import TableServiceClient\nTableServiceClient.from_connection_string('x')",
    )
    add(
        "azure_applicationinsights_telemetryclient",
        "azure",
        "from applicationinsights import TelemetryClient\nTelemetryClient('key')",
    )
    add(
        "azure_compute_management_client",
        "azure",
        "from azure.identity import DefaultAzureCredential\nfrom azure.mgmt.compute import ComputeManagementClient\nComputeManagementClient(DefaultAzureCredential(),'sub')",
    )
    add(
        "azure_containerinstance_client",
        "azure",
        "from azure.identity import DefaultAzureCredential\nfrom azure.mgmt.containerinstance import ContainerInstanceManagementClient\nContainerInstanceManagementClient(DefaultAzureCredential(),'sub')",
    )
    add(
        "azure_webmanagement_client",
        "azure",
        "from azure.identity import DefaultAzureCredential\nfrom azure.mgmt.web import WebSiteManagementClient\nWebSiteManagementClient(DefaultAzureCredential(),'sub')",
    )
    add(
        "azure_resource_management_client",
        "azure",
        "from azure.identity import DefaultAzureCredential\nfrom azure.mgmt.resource import ResourceManagementClient\nResourceManagementClient(DefaultAzureCredential(),'sub')",
    )
    add(
        "azure_monitor_client",
        "azure",
        "from azure.identity import DefaultAzureCredential\nfrom azure.monitor import MonitorClient\nMonitorClient(DefaultAzureCredential(),'sub')",
    )
    add(
        "azure_graphrbac_management_client",
        "azure",
        "from azure.identity import ClientSecretCredential\nfrom azure.graphrbac import GraphRbacManagementClient\nGraphRbacManagementClient(ClientSecretCredential('t','c','s'),'t')",
    )
    add(
        "azure_sql_management_client",
        "azure",
        "from azure.identity import DefaultAzureCredential\nfrom azure.mgmt.sql import SqlManagementClient\nSqlManagementClient(DefaultAzureCredential(),'sub')",
    )

    seen = {x["stem"] for x in out}
    all_stems = {p.stem for p in root.glob("*.toml")}
    missing = sorted(all_stems - seen)
    if missing:
        raise SystemExit(f"Missing smoke cases for: {missing}")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(out, indent=2), encoding="utf-8")
    print(f"Wrote {len(out)} cases to {OUT}")


if __name__ == "__main__":
    main()
