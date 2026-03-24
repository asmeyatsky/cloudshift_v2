from azure.cosmos import CosmosClient

client = CosmosClient(url, credential=key)
db = client.get_database_client("app")
container = db.get_container_client("users")


def get_user(user_id):
    return container.read_item(item=user_id, partition_key=user_id)


def upsert_user(doc):
    container.upsert_item(doc)
