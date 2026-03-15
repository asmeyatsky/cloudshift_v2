import boto3
import json

bedrock = boto3.client('bedrock-runtime')
rekognition = boto3.client('rekognition')
comprehend = boto3.client('comprehend')

def generate_text(prompt: str, model_id: str = 'anthropic.claude-v2') -> str:
    """Generate text using Amazon Bedrock."""
    response = bedrock.invoke_model(
        modelId=model_id,
        body=json.dumps({
            'prompt': f'\n\nHuman: {prompt}\n\nAssistant:',
            'max_tokens_to_sample': 1000
        }),
        contentType='application/json'
    )
    result = json.loads(response['body'].read())
    return result['completion']

def detect_labels_in_image(bucket: str, key: str) -> list:
    """Detect labels in an image using Rekognition."""
    response = rekognition.detect_labels(
        Image={'S3Object': {'Bucket': bucket, 'Name': key}},
        MaxLabels=10
    )
    return [{'name': label['Name'], 'confidence': label['Confidence']} for label in response['Labels']]

def analyze_text_sentiment(text: str) -> dict:
    """Analyze sentiment using Comprehend."""
    response = comprehend.detect_sentiment(
        Text=text,
        LanguageCode='en'
    )
    return {
        'sentiment': response['Sentiment'],
        'scores': response['SentimentScore']
    }
