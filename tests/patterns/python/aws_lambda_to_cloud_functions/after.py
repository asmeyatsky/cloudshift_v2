import json
import logging
import functions_framework
from google.cloud import storage

logger = logging.getLogger()
logger.setLevel(logging.INFO)

storage_client = storage.Client()


@functions_framework.cloud_event
def handle_gcs_event(cloud_event):
    """Process GCS events and transform uploaded documents."""
    data = cloud_event.data
    bucket_name = data['bucket']
    file_name = data['name']

    logger.info(f"Processing gs://{bucket_name}/{file_name}")

    bucket = storage_client.bucket(bucket_name)
    blob = bucket.blob(file_name)
    body = blob.download_as_text()

    result = process_document(body)

    output_blob = bucket.blob(f"processed/{file_name}")
    output_blob.upload_from_string(
        json.dumps(result),
        content_type='application/json',
    )

    return ('OK', 200)


def process_document(content):
    """Transform document content."""
    lines = content.strip().split('\n')
    return {
        'line_count': len(lines),
        'char_count': len(content),
        'preview': lines[0] if lines else '',
    }
