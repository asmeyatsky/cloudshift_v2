"""Azure Cognitive Services — Text Analytics / Vision."""


class CognitiveServicesManager:
    """Manages Azure Cognitive Services"""

    def __init__(self, endpoint, key):
        self.endpoint = endpoint
        self.key = key
        self.headers = {
            'Ocp-Apim-Subscription-Key': key,
            'Content-Type': 'application/json'
        }

    def analyze_text_sentiment(self, text):
        """Analyze text sentiment using Text Analytics"""
        import requests

        try:
            url = f"{self.endpoint}/text/analytics/v3.1/sentiment"
            documents = [{'id': '1', 'language': 'en', 'text': text}]
            response = requests.post(url, headers=self.headers, json={'documents': documents})

            if response.status_code == 200:
                result = response.json()
                return result['documents'][0]['sentiment']
            else:
                print(f"Error analyzing sentiment: {response.status_code}")
                return None
        except Exception as e:
            print(f"Error analyzing sentiment: {e}")
            return None

    def detect_language(self, text):
        """Detect language of text"""
        import requests

        try:
            url = f"{self.endpoint}/text/analytics/v3.1/languages"
            documents = [{'id': '1', 'text': text}]
            response = requests.post(url, headers=self.headers, json={'documents': documents})

            if response.status_code == 200:
                result = response.json()
                return result['documents'][0]['detectedLanguage']['name']
            else:
                print(f"Error detecting language: {response.status_code}")
                return None
        except Exception as e:
            print(f"Error detecting language: {e}")
            return None

    def recognize_text_from_image(self, image_url):
        """Recognize text from image using Computer Vision"""
        import requests

        try:
            url = f"{self.endpoint}/vision/v3.2/read/analyze"
            response = requests.post(
                url,
                headers={'Ocp-Apim-Subscription-Key': self.key},
                json={'url': image_url}
            )

            if response.status_code == 202:
                operation_url = response.headers['Operation-Location']
                return operation_url
            else:
                print(f"Error recognizing text: {response.status_code}")
                return None
        except Exception as e:
            print(f"Error recognizing text: {e}")
            return None
