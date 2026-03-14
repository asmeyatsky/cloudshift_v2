import json
import boto3
from botocore.exceptions import ClientError


def get_secret(secret_name, region_name='us-east-1'):
    """Retrieve a secret from AWS Secrets Manager."""
    session = boto3.session.Session()
    client = session.client(
        service_name='secretsmanager',
        region_name=region_name,
    )

    try:
        response = client.get_secret_value(SecretId=secret_name)
    except ClientError as e:
        if e.response['Error']['Code'] == 'ResourceNotFoundException':
            raise ValueError(f"Secret {secret_name} not found")
        raise

    if 'SecretString' in response:
        return json.loads(response['SecretString'])
    return response['SecretBinary']


def create_secret(secret_name, secret_value, region_name='us-east-1'):
    """Create a new secret in AWS Secrets Manager."""
    client = boto3.client('secretsmanager', region_name=region_name)
    client.create_secret(
        Name=secret_name,
        SecretString=json.dumps(secret_value),
    )


def list_secrets(region_name='us-east-1'):
    """List all secrets in AWS Secrets Manager."""
    client = boto3.client('secretsmanager', region_name=region_name)
    paginator = client.get_paginator('list_secrets')
    secrets = []
    for page in paginator.paginate():
        for secret in page['SecretList']:
            secrets.append(secret['Name'])
    return secrets
