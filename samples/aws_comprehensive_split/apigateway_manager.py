"""Split from aws_comprehensive_example.py."""
import boto3
from botocore.exceptions import ClientError

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
