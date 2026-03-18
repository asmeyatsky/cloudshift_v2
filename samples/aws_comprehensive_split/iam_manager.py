"""Split from aws_comprehensive_example.py."""
import boto3
import json
from botocore.exceptions import ClientError

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
