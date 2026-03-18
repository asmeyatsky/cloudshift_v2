"""
Comprehensive AWS Example Code
Demonstrates usage of multiple AWS managed services
This file contains ~1000 lines of AWS code for testing refactoring to GCP

For per-service transforms, use the split modules in:
  samples/aws_comprehensive_split/
See aws_comprehensive_split/README.md
"""

import boto3
import json
import os
from botocore.exceptions import ClientError
from datetime import datetime, timedelta
import uuid

# ============================================================================
# S3 - Simple Storage Service
# ============================================================================

class S3Manager:
    """Manages S3 bucket operations"""
    
    def __init__(self, region_name='us-east-1'):
        self.s3_client = boto3.client('s3', region_name=region_name)
        self.s3_resource = boto3.resource('s3', region_name=region_name)
        self.bucket_name = os.environ.get('S3_BUCKET_NAME', 'my-app-bucket')
    
    def upload_file(self, local_file_path, s3_key):
        """Upload a file to S3 bucket"""
        try:
            self.s3_client.upload_file(
                local_file_path,
                self.bucket_name,
                s3_key,
                ExtraArgs={'ContentType': 'application/json'}
            )
            print(f"Successfully uploaded {local_file_path} to s3://{self.bucket_name}/{s3_key}")
            return True
        except ClientError as e:
            print(f"Error uploading file: {e}")
            return False
    
    def download_file(self, s3_key, local_file_path):
        """Download a file from S3 bucket"""
        try:
            self.s3_resource.Bucket(self.bucket_name).download_file(s3_key, local_file_path)
            print(f"Successfully downloaded s3://{self.bucket_name}/{s3_key} to {local_file_path}")
            return True
        except ClientError as e:
            print(f"Error downloading file: {e}")
            return False
    
    def list_objects(self, prefix=''):
        """List all objects in S3 bucket with given prefix"""
        try:
            response = self.s3_client.list_objects_v2(
                Bucket=self.bucket_name,
                Prefix=prefix
            )
            return response.get('Contents', [])
        except ClientError as e:
            print(f"Error listing objects: {e}")
            return []
    
    def delete_object(self, s3_key):
        """Delete an object from S3 bucket"""
        try:
            self.s3_client.delete_object(Bucket=self.bucket_name, Key=s3_key)
            print(f"Successfully deleted s3://{self.bucket_name}/{s3_key}")
            return True
        except ClientError as e:
            print(f"Error deleting object: {e}")
            return False
    
    def generate_presigned_url(self, s3_key, expiration=3600):
        """Generate a presigned URL for temporary access"""
        try:
            url = self.s3_client.generate_presigned_url(
                'get_object',
                Params={'Bucket': self.bucket_name, 'Key': s3_key},
                ExpiresIn=expiration
            )
            return url
        except ClientError as e:
            print(f"Error generating presigned URL: {e}")
            return None

# ============================================================================
# DynamoDB - NoSQL Database
# ============================================================================

class DynamoDBManager:
    """Manages DynamoDB table operations"""
    
    def __init__(self, region_name='us-east-1'):
        self.dynamodb = boto3.resource('dynamodb', region_name=region_name)
        self.dynamodb_client = boto3.client('dynamodb', region_name=region_name)
        self.table_name = os.environ.get('DYNAMODB_TABLE', 'UserData')
    
    def create_table(self, table_name, partition_key, sort_key=None):
        """Create a DynamoDB table"""
        try:
            key_schema = [
                {'AttributeName': partition_key, 'KeyType': 'HASH'}
            ]
            attribute_definitions = [
                {'AttributeName': partition_key, 'AttributeType': 'S'}
            ]
            
            if sort_key:
                key_schema.append({'AttributeName': sort_key, 'KeyType': 'RANGE'})
                attribute_definitions.append({'AttributeName': sort_key, 'AttributeType': 'S'})
            
            table = self.dynamodb.create_table(
                TableName=table_name,
                KeySchema=key_schema,
                AttributeDefinitions=attribute_definitions,
                BillingMode='PAY_PER_REQUEST'
            )
            table.wait_until_exists()
            print(f"Table {table_name} created successfully")
            return table
        except ClientError as e:
            print(f"Error creating table: {e}")
            return None
    
    def put_item(self, table_name, item):
        """Put an item into DynamoDB table"""
        try:
            table = self.dynamodb.Table(table_name)
            response = table.put_item(Item=item)
            print(f"Item inserted into {table_name}")
            return response
        except ClientError as e:
            print(f"Error putting item: {e}")
            return None
    
    def get_item(self, table_name, key):
        """Get an item from DynamoDB table"""
        try:
            table = self.dynamodb.Table(table_name)
            response = table.get_item(Key=key)
            if 'Item' in response:
                return response['Item']
            return None
        except ClientError as e:
            print(f"Error getting item: {e}")
            return None
    
    def query_items(self, table_name, partition_key_value, index_name=None):
        """Query items from DynamoDB table"""
        try:
            table = self.dynamodb.Table(table_name)
            if index_name:
                response = table.query(
                    IndexName=index_name,
                    KeyConditionExpression='partition_key = :pk',
                    ExpressionAttributeValues={':pk': partition_key_value}
                )
            else:
                response = table.query(
                    KeyConditionExpression='id = :pk',
                    ExpressionAttributeValues={':pk': partition_key_value}
                )
            return response.get('Items', [])
        except ClientError as e:
            print(f"Error querying items: {e}")
            return []
    
    def scan_table(self, table_name, filter_expression=None):
        """Scan all items in DynamoDB table"""
        try:
            table = self.dynamodb.Table(table_name)
            if filter_expression:
                response = table.scan(FilterExpression=filter_expression)
            else:
                response = table.scan()
            return response.get('Items', [])
        except ClientError as e:
            print(f"Error scanning table: {e}")
            return []
    
    def update_item(self, table_name, key, update_expression, expression_attribute_values):
        """Update an item in DynamoDB table"""
        try:
            table = self.dynamodb.Table(table_name)
            response = table.update_item(
                Key=key,
                UpdateExpression=update_expression,
                ExpressionAttributeValues=expression_attribute_values,
                ReturnValues='UPDATED_NEW'
            )
            return response
        except ClientError as e:
            print(f"Error updating item: {e}")
            return None
    
    def delete_item(self, table_name, key):
        """Delete an item from DynamoDB table"""
        try:
            table = self.dynamodb.Table(table_name)
            response = table.delete_item(Key=key)
            print(f"Item deleted from {table_name}")
            return response
        except ClientError as e:
            print(f"Error deleting item: {e}")
            return None

# ============================================================================
# Lambda - Serverless Functions
# ============================================================================

class LambdaManager:
    """Manages AWS Lambda functions"""
    
    def __init__(self, region_name='us-east-1'):
        self.lambda_client = boto3.client('lambda', region_name=region_name)
    
    def create_function(self, function_name, runtime, handler, role_arn, zip_file_path):
        """Create a Lambda function"""
        try:
            with open(zip_file_path, 'rb') as f:
                zip_content = f.read()
            
            response = self.lambda_client.create_function(
                FunctionName=function_name,
                Runtime=runtime,
                Role=role_arn,
                Handler=handler,
                Code={'ZipFile': zip_content},
                Description=f'Lambda function {function_name}',
                Timeout=30,
                MemorySize=128
            )
            print(f"Lambda function {function_name} created successfully")
            return response
        except ClientError as e:
            print(f"Error creating Lambda function: {e}")
            return None
    
    def invoke_function(self, function_name, payload):
        """Invoke a Lambda function"""
        try:
            response = self.lambda_client.invoke(
                FunctionName=function_name,
                InvocationType='RequestResponse',
                Payload=json.dumps(payload)
            )
            result = json.loads(response['Payload'].read())
            return result
        except ClientError as e:
            print(f"Error invoking Lambda function: {e}")
            return None
    
    def update_function_code(self, function_name, zip_file_path):
        """Update Lambda function code"""
        try:
            with open(zip_file_path, 'rb') as f:
                zip_content = f.read()
            
            response = self.lambda_client.update_function_code(
                FunctionName=function_name,
                ZipFile=zip_content
            )
            print(f"Lambda function {function_name} code updated")
            return response
        except ClientError as e:
            print(f"Error updating Lambda function code: {e}")
            return None
    
    def list_functions(self):
        """List all Lambda functions"""
        try:
            response = self.lambda_client.list_functions()
            return response.get('Functions', [])
        except ClientError as e:
            print(f"Error listing Lambda functions: {e}")
            return []
    
    def delete_function(self, function_name):
        """Delete a Lambda function"""
        try:
            self.lambda_client.delete_function(FunctionName=function_name)
            print(f"Lambda function {function_name} deleted")
            return True
        except ClientError as e:
            print(f"Error deleting Lambda function: {e}")
            return False

# ============================================================================
# SNS - Simple Notification Service
# ============================================================================

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

# ============================================================================
# SQS - Simple Queue Service
# ============================================================================

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

# ============================================================================
# EC2 - Elastic Compute Cloud
# ============================================================================

class EC2Manager:
    """Manages EC2 instances"""
    
    def __init__(self, region_name='us-east-1'):
        self.ec2_client = boto3.client('ec2', region_name=region_name)
        self.ec2_resource = boto3.resource('ec2', region_name=region_name)
    
    def create_instance(self, image_id, instance_type, key_name, security_group_ids):
        """Launch an EC2 instance"""
        try:
            instances = self.ec2_resource.create_instances(
                ImageId=image_id,
                MinCount=1,
                MaxCount=1,
                InstanceType=instance_type,
                KeyName=key_name,
                SecurityGroupIds=security_group_ids,
                TagSpecifications=[
                    {
                        'ResourceType': 'instance',
                        'Tags': [
                            {'Key': 'Name', 'Value': 'MyAppServer'},
                            {'Key': 'Environment', 'Value': 'Production'}
                        ]
                    }
                ]
            )
            instance = instances[0]
            instance.wait_until_running()
            print(f"Instance {instance.id} launched successfully")
            return instance
        except ClientError as e:
            print(f"Error creating instance: {e}")
            return None
    
    def list_instances(self, filters=None):
        """List EC2 instances"""
        try:
            if filters is None:
                filters = [{'Name': 'instance-state-name', 'Values': ['running']}]
            
            response = self.ec2_client.describe_instances(Filters=filters)
            instances = []
            for reservation in response['Reservations']:
                instances.extend(reservation['Instances'])
            return instances
        except ClientError as e:
            print(f"Error listing instances: {e}")
            return []
    
    def terminate_instance(self, instance_id):
        """Terminate an EC2 instance"""
        try:
            self.ec2_client.terminate_instances(InstanceIds=[instance_id])
            print(f"Instance {instance_id} termination initiated")
            return True
        except ClientError as e:
            print(f"Error terminating instance: {e}")
            return False
    
    def create_security_group(self, group_name, description, vpc_id):
        """Create a security group"""
        try:
            response = self.ec2_client.create_security_group(
                GroupName=group_name,
                Description=description,
                VpcId=vpc_id
            )
            security_group_id = response['GroupId']
            print(f"Security group {group_name} created with ID: {security_group_id}")
            return security_group_id
        except ClientError as e:
            print(f"Error creating security group: {e}")
            return None
    
    def add_security_group_rule(self, group_id, protocol, port, cidr):
        """Add a rule to a security group"""
        try:
            self.ec2_client.authorize_security_group_ingress(
                GroupId=group_id,
                IpProtocol=protocol,
                FromPort=port,
                ToPort=port,
                CidrIp=cidr
            )
            print(f"Rule added to security group {group_id}")
            return True
        except ClientError as e:
            print(f"Error adding security group rule: {e}")
            return False

# ============================================================================
# RDS - Relational Database Service
# ============================================================================

class RDSManager:
    """Manages RDS database instances"""
    
    def __init__(self, region_name='us-east-1'):
        self.rds_client = boto3.client('rds', region_name=region_name)
    
    def create_database_instance(self, db_instance_identifier, db_name, master_username, master_password, db_instance_class):
        """Create an RDS database instance"""
        try:
            response = self.rds_client.create_db_instance(
                DBInstanceIdentifier=db_instance_identifier,
                DBName=db_name,
                DBInstanceClass=db_instance_class,
                Engine='postgres',
                MasterUsername=master_username,
                MasterUserPassword=master_password,
                AllocatedStorage=20,
                VpcSecurityGroupIds=[],
                BackupRetentionPeriod=7,
                MultiAZ=False,
                PubliclyAccessible=False
            )
            print(f"RDS instance {db_instance_identifier} creation initiated")
            return response
        except ClientError as e:
            print(f"Error creating RDS instance: {e}")
            return None
    
    def describe_db_instances(self):
        """Describe all RDS instances"""
        try:
            response = self.rds_client.describe_db_instances()
            return response.get('DBInstances', [])
        except ClientError as e:
            print(f"Error describing RDS instances: {e}")
            return []
    
    def delete_db_instance(self, db_instance_identifier, skip_final_snapshot=True):
        """Delete an RDS database instance"""
        try:
            self.rds_client.delete_db_instance(
                DBInstanceIdentifier=db_instance_identifier,
                SkipFinalSnapshot=skip_final_snapshot
            )
            print(f"RDS instance {db_instance_identifier} deletion initiated")
            return True
        except ClientError as e:
            print(f"Error deleting RDS instance: {e}")
            return False

# ============================================================================
# CloudWatch - Monitoring and Logging
# ============================================================================

class CloudWatchManager:
    """Manages CloudWatch metrics and logs"""
    
    def __init__(self, region_name='us-east-1'):
        self.cloudwatch_client = boto3.client('cloudwatch', region_name=region_name)
        self.logs_client = boto3.client('logs', region_name=region_name)
    
    def put_metric(self, namespace, metric_name, value, unit='Count', dimensions=None):
        """Put a custom metric to CloudWatch"""
        try:
            params = {
                'Namespace': namespace,
                'MetricData': [{
                    'MetricName': metric_name,
                    'Value': value,
                    'Unit': unit
                }]
            }
            if dimensions:
                params['MetricData'][0]['Dimensions'] = dimensions
            
            self.cloudwatch_client.put_metric_data(**params)
            print(f"Metric {metric_name} published to CloudWatch")
            return True
        except ClientError as e:
            print(f"Error putting metric: {e}")
            return False
    
    def get_metric_statistics(self, namespace, metric_name, start_time, end_time, period, statistics):
        """Get metric statistics from CloudWatch"""
        try:
            response = self.cloudwatch_client.get_metric_statistics(
                Namespace=namespace,
                MetricName=metric_name,
                StartTime=start_time,
                EndTime=end_time,
                Period=period,
                Statistics=statistics
            )
            return response.get('Datapoints', [])
        except ClientError as e:
            print(f"Error getting metric statistics: {e}")
            return []
    
    def create_log_group(self, log_group_name):
        """Create a CloudWatch log group"""
        try:
            self.logs_client.create_log_group(logGroupName=log_group_name)
            print(f"Log group {log_group_name} created")
            return True
        except ClientError as e:
            print(f"Error creating log group: {e}")
            return False
    
    def put_log_events(self, log_group_name, log_stream_name, log_events):
        """Put log events to CloudWatch Logs"""
        try:
            self.logs_client.put_log_events(
                logGroupName=log_group_name,
                logStreamName=log_stream_name,
                logEvents=log_events
            )
            print(f"Log events written to {log_group_name}/{log_stream_name}")
            return True
        except ClientError as e:
            print(f"Error putting log events: {e}")
            return False

# ============================================================================
# API Gateway - REST API Management
# ============================================================================

class APIGatewayManager:
    """Manages API Gateway REST APIs"""
    
    def __init__(self, region_name='us-east-1'):
        self.apigateway_client = boto3.client('apigateway', region_name=region_name)
    
    def create_rest_api(self, name, description):
        """Create a REST API"""
        try:
            response = self.apigateway_client.create_rest_api(
                name=name,
                description=description,
                endpointConfiguration={'types': ['REGIONAL']}
            )
            api_id = response['id']
            print(f"REST API {name} created with ID: {api_id}")
            return response
        except ClientError as e:
            print(f"Error creating REST API: {e}")
            return None
    
    def create_resource(self, rest_api_id, parent_id, path_part):
        """Create a resource in the API"""
        try:
            response = self.apigateway_client.create_resource(
                restApiId=rest_api_id,
                parentId=parent_id,
                pathPart=path_part
            )
            return response
        except ClientError as e:
            print(f"Error creating resource: {e}")
            return None
    
    def put_method(self, rest_api_id, resource_id, http_method, authorization_type='NONE'):
        """Put an HTTP method on a resource"""
        try:
            response = self.apigateway_client.put_method(
                restApiId=rest_api_id,
                resourceId=resource_id,
                httpMethod=http_method,
                authorizationType=authorization_type
            )
            return response
        except ClientError as e:
            print(f"Error putting method: {e}")
            return None
    
    def create_deployment(self, rest_api_id, stage_name):
        """Create a deployment for the API"""
        try:
            response = self.apigateway_client.create_deployment(
                restApiId=rest_api_id,
                stageName=stage_name
            )
            print(f"Deployment created for stage {stage_name}")
            return response
        except ClientError as e:
            print(f"Error creating deployment: {e}")
            return None

# ============================================================================
# Step Functions - Workflow Orchestration
# ============================================================================

class StepFunctionsManager:
    """Manages AWS Step Functions state machines"""
    
    def __init__(self, region_name='us-east-1'):
        self.sfn_client = boto3.client('stepfunctions', region_name=region_name)
    
    def create_state_machine(self, name, definition, role_arn):
        """Create a Step Functions state machine"""
        try:
            response = self.sfn_client.create_state_machine(
                name=name,
                definition=json.dumps(definition),
                roleArn=role_arn
            )
            state_machine_arn = response['stateMachineArn']
            print(f"State machine {name} created with ARN: {state_machine_arn}")
            return response
        except ClientError as e:
            print(f"Error creating state machine: {e}")
            return None
    
    def start_execution(self, state_machine_arn, input_data):
        """Start a Step Functions execution"""
        try:
            response = self.sfn_client.start_execution(
                stateMachineArn=state_machine_arn,
                input=json.dumps(input_data)
            )
            execution_arn = response['executionArn']
            print(f"Execution started with ARN: {execution_arn}")
            return response
        except ClientError as e:
            print(f"Error starting execution: {e}")
            return None
    
    def describe_execution(self, execution_arn):
        """Describe a Step Functions execution"""
        try:
            response = self.sfn_client.describe_execution(executionArn=execution_arn)
            return response
        except ClientError as e:
            print(f"Error describing execution: {e}")
            return None

# ============================================================================
# IAM - Identity and Access Management
# ============================================================================

class IAMManager:
    """Manages IAM roles and policies"""
    
    def __init__(self):
        self.iam_client = boto3.client('iam')
    
    def create_role(self, role_name, assume_role_policy_document):
        """Create an IAM role"""
        try:
            response = self.iam_client.create_role(
                RoleName=role_name,
                AssumeRolePolicyDocument=json.dumps(assume_role_policy_document),
                Description=f'Role for {role_name}'
            )
            role_arn = response['Role']['Arn']
            print(f"Role {role_name} created with ARN: {role_arn}")
            return response
        except ClientError as e:
            print(f"Error creating role: {e}")
            return None
    
    def attach_role_policy(self, role_name, policy_arn):
        """Attach a policy to an IAM role"""
        try:
            self.iam_client.attach_role_policy(
                RoleName=role_name,
                PolicyArn=policy_arn
            )
            print(f"Policy {policy_arn} attached to role {role_name}")
            return True
        except ClientError as e:
            print(f"Error attaching policy: {e}")
            return False
    
    def create_policy(self, policy_name, policy_document):
        """Create an IAM policy"""
        try:
            response = self.iam_client.create_policy(
                PolicyName=policy_name,
                PolicyDocument=json.dumps(policy_document)
            )
            policy_arn = response['Policy']['Arn']
            print(f"Policy {policy_name} created with ARN: {policy_arn}")
            return response
        except ClientError as e:
            print(f"Error creating policy: {e}")
            return None

# ============================================================================
# Main Application Example
# ============================================================================

def main():
    """Main application demonstrating AWS services integration"""
    
    # Initialize managers
    s3_manager = S3Manager()
    dynamodb_manager = DynamoDBManager()
    lambda_manager = LambdaManager()
    sns_manager = SNSManager()
    sqs_manager = SQSManager()
    ec2_manager = EC2Manager()
    rds_manager = RDSManager()
    cloudwatch_manager = CloudWatchManager()
    api_gateway_manager = APIGatewayManager()
    step_functions_manager = StepFunctionsManager()
    iam_manager = IAMManager()
    
    # Example workflow
    print("Starting AWS services integration example...")
    
    # 1. Upload data to S3
    s3_manager.upload_file('data.json', 'uploads/data.json')
    
    # 2. Store metadata in DynamoDB
    user_item = {
        'id': str(uuid.uuid4()),
        'name': 'John Doe',
        'email': 'john@example.com',
        'created_at': datetime.now().isoformat()
    }
    dynamodb_manager.put_item('UserData', user_item)
    
    # 3. Publish notification via SNS
    topic_arn = sns_manager.create_topic('user-notifications')
    sns_manager.publish_message(topic_arn, {'event': 'user_created', 'user_id': user_item['id']}, 'New User')
    
    # 4. Send message to SQS queue
    queue_url = sqs_manager.create_queue('processing-queue')
    sqs_manager.send_message(queue_url, json.dumps({'action': 'process_user', 'user_id': user_item['id']}))
    
    # 5. Log metrics to CloudWatch
    cloudwatch_manager.put_metric('MyApp', 'UsersCreated', 1, 'Count')
    
    print("AWS services integration example completed!")

# ============================================================================
# Additional AWS Services Examples
# ============================================================================

# ============================================================================
# Secrets Manager - Secrets Management
# ============================================================================

class SecretsManager:
    """Manages AWS Secrets Manager"""
    
    def __init__(self, region_name='us-east-1'):
        self.secrets_client = boto3.client('secretsmanager', region_name=region_name)
    
    def create_secret(self, secret_name, secret_string, description=None):
        """Create a secret in Secrets Manager"""
        try:
            params = {
                'Name': secret_name,
                'SecretString': secret_string
            }
            if description:
                params['Description'] = description
            
            response = self.secrets_client.create_secret(**params)
            print(f"Secret {secret_name} created successfully")
            return response
        except ClientError as e:
            print(f"Error creating secret: {e}")
            return None
    
    def get_secret_value(self, secret_name):
        """Get a secret value from Secrets Manager"""
        try:
            response = self.secrets_client.get_secret_value(SecretId=secret_name)
            return response['SecretString']
        except ClientError as e:
            print(f"Error getting secret: {e}")
            return None
    
    def update_secret(self, secret_name, secret_string):
        """Update a secret in Secrets Manager"""
        try:
            response = self.secrets_client.update_secret(
                SecretId=secret_name,
                SecretString=secret_string
            )
            print(f"Secret {secret_name} updated successfully")
            return response
        except ClientError as e:
            print(f"Error updating secret: {e}")
            return None
    
    def delete_secret(self, secret_name, recovery_window_days=30):
        """Delete a secret from Secrets Manager"""
        try:
            self.secrets_client.delete_secret(
                SecretId=secret_name,
                RecoveryWindowInDays=recovery_window_days
            )
            print(f"Secret {secret_name} deletion scheduled")
            return True
        except ClientError as e:
            print(f"Error deleting secret: {e}")
            return False

# ============================================================================
# Kinesis - Real-time Data Streaming
# ============================================================================

class KinesisManager:
    """Manages AWS Kinesis streams"""
    
    def __init__(self, region_name='us-east-1'):
        self.kinesis_client = boto3.client('kinesis', region_name=region_name)
    
    def create_stream(self, stream_name, shard_count=1):
        """Create a Kinesis stream"""
        try:
            self.kinesis_client.create_stream(
                StreamName=stream_name,
                ShardCount=shard_count
            )
            print(f"Kinesis stream {stream_name} created")
            return True
        except ClientError as e:
            print(f"Error creating stream: {e}")
            return False
    
    def put_record(self, stream_name, data, partition_key):
        """Put a record into a Kinesis stream"""
        try:
            response = self.kinesis_client.put_record(
                StreamName=stream_name,
                Data=data.encode('utf-8'),
                PartitionKey=partition_key
            )
            print(f"Record put into stream {stream_name}")
            return response
        except ClientError as e:
            print(f"Error putting record: {e}")
            return None
    
    def get_records(self, shard_iterator, limit=10):
        """Get records from a Kinesis stream"""
        try:
            response = self.kinesis_client.get_records(
                ShardIterator=shard_iterator,
                Limit=limit
            )
            return response.get('Records', [])
        except ClientError as e:
            print(f"Error getting records: {e}")
            return []
    
    def get_shard_iterator(self, stream_name, shard_id, shard_iterator_type='TRIM_HORIZON'):
        """Get a shard iterator for reading records"""
        try:
            response = self.kinesis_client.get_shard_iterator(
                StreamName=stream_name,
                ShardId=shard_id,
                ShardIteratorType=shard_iterator_type
            )
            return response['ShardIterator']
        except ClientError as e:
            print(f"Error getting shard iterator: {e}")
            return None

# ============================================================================
# ElastiCache - In-Memory Caching
# ============================================================================

class ElastiCacheManager:
    """Manages AWS ElastiCache clusters"""
    
    def __init__(self, region_name='us-east-1'):
        self.elasticache_client = boto3.client('elasticache', region_name=region_name)
    
    def create_cache_cluster(self, cluster_id, node_type, num_cache_nodes=1, engine='redis'):
        """Create an ElastiCache cluster"""
        try:
            response = self.elasticache_client.create_cache_cluster(
                CacheClusterId=cluster_id,
                NodeType=node_type,
                NumCacheNodes=num_cache_nodes,
                Engine=engine
            )
            print(f"ElastiCache cluster {cluster_id} creation initiated")
            return response
        except ClientError as e:
            print(f"Error creating cache cluster: {e}")
            return None
    
    def describe_cache_clusters(self, cluster_id=None):
        """Describe ElastiCache clusters"""
        try:
            params = {}
            if cluster_id:
                params['CacheClusterId'] = cluster_id
            
            response = self.elasticache_client.describe_cache_clusters(**params)
            return response.get('CacheClusters', [])
        except ClientError as e:
            print(f"Error describing cache clusters: {e}")
            return []
    
    def delete_cache_cluster(self, cluster_id):
        """Delete an ElastiCache cluster"""
        try:
            self.elasticache_client.delete_cache_cluster(CacheClusterId=cluster_id)
            print(f"ElastiCache cluster {cluster_id} deletion initiated")
            return True
        except ClientError as e:
            print(f"Error deleting cache cluster: {e}")
            return False

# ============================================================================
# ECS - Elastic Container Service
# ============================================================================

class ECSManager:
    """Manages AWS ECS clusters and tasks"""
    
    def __init__(self, region_name='us-east-1'):
        self.ecs_client = boto3.client('ecs', region_name=region_name)
    
    def create_cluster(self, cluster_name):
        """Create an ECS cluster"""
        try:
            response = self.ecs_client.create_cluster(clusterName=cluster_name)
            print(f"ECS cluster {cluster_name} created")
            return response
        except ClientError as e:
            print(f"Error creating cluster: {e}")
            return None
    
    def register_task_definition(self, family, container_definitions, cpu='256', memory='512'):
        """Register an ECS task definition"""
        try:
            response = self.ecs_client.register_task_definition(
                family=family,
                containerDefinitions=container_definitions,
                cpu=cpu,
                memory=memory,
                requiresCompatibilities=['FARGATE'],
                networkMode='awsvpc'
            )
            print(f"Task definition {family} registered")
            return response
        except ClientError as e:
            print(f"Error registering task definition: {e}")
            return None
    
    def run_task(self, cluster_name, task_definition, subnets, security_groups):
        """Run an ECS task"""
        try:
            response = self.ecs_client.run_task(
                cluster=cluster_name,
                taskDefinition=task_definition,
                launchType='FARGATE',
                networkConfiguration={
                    'awsvpcConfiguration': {
                        'subnets': subnets,
                        'securityGroups': security_groups,
                        'assignPublicIp': 'ENABLED'
                    }
                }
            )
            print(f"Task started in cluster {cluster_name}")
            return response
        except ClientError as e:
            print(f"Error running task: {e}")
            return None
    
    def list_tasks(self, cluster_name):
        """List tasks in an ECS cluster"""
        try:
            response = self.ecs_client.list_tasks(cluster=cluster_name)
            return response.get('taskArns', [])
        except ClientError as e:
            print(f"Error listing tasks: {e}")
            return []

# ============================================================================
# EKS - Elastic Kubernetes Service
# ============================================================================

class EKSManager:
    """Manages AWS EKS clusters"""
    
    def __init__(self, region_name='us-east-1'):
        self.eks_client = boto3.client('eks', region_name=region_name)
    
    def create_cluster(self, cluster_name, role_arn, vpc_config):
        """Create an EKS cluster"""
        try:
            response = self.eks_client.create_cluster(
                name=cluster_name,
                version='1.27',
                roleArn=role_arn,
                resourcesVpcConfig=vpc_config
            )
            print(f"EKS cluster {cluster_name} creation initiated")
            return response
        except ClientError as e:
            print(f"Error creating EKS cluster: {e}")
            return None
    
    def describe_cluster(self, cluster_name):
        """Describe an EKS cluster"""
        try:
            response = self.eks_client.describe_cluster(name=cluster_name)
            return response.get('cluster')
        except ClientError as e:
            print(f"Error describing cluster: {e}")
            return None
    
    def list_clusters(self):
        """List all EKS clusters"""
        try:
            response = self.eks_client.list_clusters()
            return response.get('clusters', [])
        except ClientError as e:
            print(f"Error listing clusters: {e}")
            return []

# ============================================================================
# SES - Simple Email Service
# ============================================================================

class SESManager:
    """Manages AWS SES email sending"""
    
    def __init__(self, region_name='us-east-1'):
        self.ses_client = boto3.client('ses', region_name=region_name)
    
    def verify_email_address(self, email_address):
        """Verify an email address for SES"""
        try:
            self.ses_client.verify_email_identity(EmailAddress=email_address)
            print(f"Verification email sent to {email_address}")
            return True
        except ClientError as e:
            print(f"Error verifying email: {e}")
            return False
    
    def send_email(self, source, destination, subject, body_text, body_html=None):
        """Send an email via SES"""
        try:
            params = {
                'Source': source,
                'Destination': {'ToAddresses': [destination]},
                'Message': {
                    'Subject': {'Data': subject},
                    'Body': {'Text': {'Data': body_text}}
                }
            }
            if body_html:
                params['Message']['Body']['Html'] = {'Data': body_html}
            
            response = self.ses_client.send_email(**params)
            print(f"Email sent successfully. Message ID: {response['MessageId']}")
            return response
        except ClientError as e:
            print(f"Error sending email: {e}")
            return None
    
    def list_verified_emails(self):
        """List verified email addresses"""
        try:
            response = self.ses_client.list_verified_email_addresses()
            return response.get('VerifiedEmailAddresses', [])
        except ClientError as e:
            print(f"Error listing verified emails: {e}")
            return []

# ============================================================================
# Route 53 - DNS Service
# ============================================================================

class Route53Manager:
    """Manages AWS Route 53 DNS records"""
    
    def __init__(self):
        self.route53_client = boto3.client('route53')
    
    def create_hosted_zone(self, name, caller_reference=None):
        """Create a Route 53 hosted zone"""
        try:
            if not caller_reference:
                caller_reference = str(uuid.uuid4())
            
            response = self.route53_client.create_hosted_zone(
                Name=name,
                CallerReference=caller_reference
            )
            zone_id = response['HostedZone']['Id'].split('/')[-1]
            print(f"Hosted zone {name} created with ID: {zone_id}")
            return response
        except ClientError as e:
            print(f"Error creating hosted zone: {e}")
            return None
    
    def create_record(self, hosted_zone_id, name, record_type, value, ttl=300):
        """Create a DNS record"""
        try:
            response = self.route53_client.change_resource_record_sets(
                HostedZoneId=hosted_zone_id,
                ChangeBatch={
                    'Changes': [{
                        'Action': 'CREATE',
                        'ResourceRecordSet': {
                            'Name': name,
                            'Type': record_type,
                            'TTL': ttl,
                            'ResourceRecords': [{'Value': value}]
                        }
                    }]
                }
            )
            print(f"DNS record {name} created")
            return response
        except ClientError as e:
            print(f"Error creating DNS record: {e}")
            return None
    
    def list_hosted_zones(self):
        """List all hosted zones"""
        try:
            response = self.route53_client.list_hosted_zones()
            return response.get('HostedZones', [])
        except ClientError as e:
            print(f"Error listing hosted zones: {e}")
            return []

# ============================================================================
# Extended Main Application Example
# ============================================================================

def extended_main():
    """Extended example using additional AWS services"""
    
    secrets_manager = SecretsManager()
    kinesis_manager = KinesisManager()
    elasticache_manager = ElastiCacheManager()
    ecs_manager = ECSManager()
    eks_manager = EKSManager()
    ses_manager = SESManager()
    route53_manager = Route53Manager()
    
    print("Starting extended AWS services integration example...")
    
    # Store secrets
    secrets_manager.create_secret('db-password', 'my-secret-password', 'Database password')
    
    # Create Kinesis stream
    kinesis_manager.create_stream('data-stream', shard_count=2)
    
    # Create ElastiCache cluster
    elasticache_manager.create_cache_cluster('my-cache', 'cache.t3.micro')
    
    # Send email via SES
    ses_manager.send_email(
        source='noreply@example.com',
        destination='user@example.com',
        subject='Welcome!',
        body_text='Welcome to our service!',
        body_html='<h1>Welcome!</h1><p>Welcome to our service!</p>'
    )
    
    print("Extended AWS services integration example completed!")

if __name__ == '__main__':
    main()
    extended_main()
