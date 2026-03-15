import boto3
import json

secrets_client = boto3.client('secretsmanager')
sts_client = boto3.client('sts')

def get_database_credentials(secret_name: str = 'prod/database') -> dict:
    """Retrieve database credentials from Secrets Manager."""
    response = secrets_client.get_secret_value(SecretId=secret_name)
    return json.loads(response['SecretString'])

def get_api_key(secret_name: str) -> str:
    """Get a single API key from Secrets Manager."""
    response = secrets_client.get_secret_value(SecretId=secret_name)
    return response['SecretString']

def assume_cross_account_role(role_arn: str, session_name: str = 'cloudshift') -> dict:
    """Assume a role in another AWS account."""
    response = sts_client.assume_role(
        RoleArn=role_arn,
        RoleSessionName=session_name,
        DurationSeconds=3600
    )
    credentials = response['Credentials']
    return {
        'access_key': credentials['AccessKeyId'],
        'secret_key': credentials['SecretAccessKey'],
        'session_token': credentials['SessionToken']
    }
