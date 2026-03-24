from google.cloud import secretmanager

client = secretmanager.SecretManagerServiceClient()


def get_param(name: str):
    secret_name = f"projects/my-project/secrets/{name}/versions/latest"
    response = client.access_secret_version(name=secret_name)
    return response.payload.data.decode("utf-8")


def put_param(name: str, value: str):
    parent = f"projects/my-project/secrets/{name}"
    client.add_secret_version(parent=parent, payload={"data": value.encode("utf-8")})
