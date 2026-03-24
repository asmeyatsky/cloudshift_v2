from azure.search.documents import SearchClient
from azure.core.credentials import AzureKeyCredential

endpoint = "https://mysearch.search.windows.net"
client = SearchClient(endpoint, "products", AzureKeyCredential(admin_key))


def search_products(q: str):
    return list(client.search(search_text=q, top=20))
