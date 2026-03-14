from google.cloud import storage

storage_client = storage.Client()

def upload_document(bucket_name, key, content):
    bucket = storage_client.bucket(bucket_name)
    blob = bucket.blob(key)
    blob.upload_from_string(content)

def download_document(bucket_name, key):
    bucket = storage_client.bucket(bucket_name)
    blob = bucket.blob(key)
    return blob.download_as_bytes()

def list_documents(bucket_name, prefix):
    bucket = storage_client.bucket(bucket_name)
    blobs = bucket.list_blobs(prefix=prefix)
    return [blob.name for blob in blobs]
