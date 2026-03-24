from google.cloud import discoveryengine_v1 as discoveryengine

SERVING_CONFIG = "projects/my-project/locations/global/collections/default/dataStores/products/servingConfigs/default"
client = discoveryengine.SearchServiceClient()


def search_products(q: str):
    response = client.search(request={"serving_config": SERVING_CONFIG, "query": q, "page_size": 20})
    return list(response.results)
