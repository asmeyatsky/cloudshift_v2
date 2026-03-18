"""Split from aws_comprehensive_example.py."""
import boto3
import json
from botocore.exceptions import ClientError

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
