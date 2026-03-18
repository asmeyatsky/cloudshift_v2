"""Split from aws_comprehensive_example.py."""
import boto3
import json
from botocore.exceptions import ClientError

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
