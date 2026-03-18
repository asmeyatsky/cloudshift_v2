"""AWS Secrets Manager — split sample (class renamed to avoid confusion)."""
"""Split from aws_comprehensive_example.py."""
import boto3
from botocore.exceptions import ClientError

class SecretsManagerSample:
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
