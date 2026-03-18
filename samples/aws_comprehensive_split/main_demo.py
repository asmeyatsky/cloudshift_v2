"""Orchestrator demo — wire split managers together.
Run from repo root:  PYTHONPATH=samples/aws_comprehensive_split python samples/aws_comprehensive_split/main_demo.py
(Requires AWS credentials; will call real APIs if run.)
"""
import json
import uuid
from datetime import datetime

from s3_manager import S3Manager
from dynamodb_manager import DynamoDBManager
from lambda_manager import LambdaManager
from sns_manager import SNSManager
from sqs_manager import SQSManager
from ec2_manager import EC2Manager
from rds_manager import RDSManager
from cloudwatch_manager import CloudWatchManager
from apigateway_manager import APIGatewayManager
from stepfunctions_manager import StepFunctionsManager
from iam_manager import IAMManager
from secrets_manager_sample import SecretsManagerSample
from kinesis_manager import KinesisManager
from elasticache_manager import ElastiCacheManager
from ecs_manager import ECSManager
from eks_manager import EKSManager
from ses_manager import SESManager
from route53_manager import Route53Manager

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


def extended_main():
    """Extended example using additional AWS services"""
    
    secrets_manager = SecretsManagerSample()
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
