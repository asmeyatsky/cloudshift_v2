import json
from google.cloud import secretmanager
from google.api_core import exceptions


def get_secret(secret_name, project_id='my-project'):
    """Retrieve a secret from GCP Secret Manager."""
    client = secretmanager.SecretManagerServiceClient()
    name = f"projects/{project_id}/secrets/{secret_name}/versions/latest"

    try:
        response = client.access_secret_version(request={"name": name})
    except exceptions.NotFound:
        raise ValueError(f"Secret {secret_name} not found")

    payload = response.payload.data.decode("utf-8")
    try:
        return json.loads(payload)
    except json.JSONDecodeError:
        return response.payload.data


def create_secret(secret_name, secret_value, project_id='my-project'):
    """Create a new secret in GCP Secret Manager."""
    client = secretmanager.SecretManagerServiceClient()
    parent = f"projects/{project_id}"

    secret = client.create_secret(
        request={
            "parent": parent,
            "secret_id": secret_name,
            "secret": {"replication": {"automatic": {}}},
        }
    )

    client.add_secret_version(
        request={
            "parent": secret.name,
            "payload": {"data": json.dumps(secret_value).encode("utf-8")},
        }
    )


def list_secrets(project_id='my-project'):
    """List all secrets in GCP Secret Manager."""
    client = secretmanager.SecretManagerServiceClient()
    parent = f"projects/{project_id}"
    secrets = []
    for secret in client.list_secrets(request={"parent": parent}):
        # Extract secret name from full resource path
        secrets.append(secret.name.split('/')[-1])
    return secrets
