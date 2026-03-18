"""
GCP **Natural Language API** + **Vision API** — analogues for Azure Cognitive
Services (Text Analytics sentiment / language, Computer Vision Read OCR).

Azure maps:
  Subscription key + regional endpoint  →  **Application Default Credentials**
  Text Analytics sentiment            →  ``analyze_sentiment`` (score → positive/negative/neutral)
  Text Analytics languages            →  response ``language`` (BCP-47; optional name map below)
  Vision Read (async 202 + poll)      →  Vision **document_text_detection** is typically **sync**;
  this sample returns **full extracted text** in one call.

Requires: ``pip install google-cloud-language google-cloud-vision google-api-core``
"""
from __future__ import annotations

from google.api_core import exceptions as gcp_exceptions
from google.cloud import language_v1
from google.cloud import vision


_LANG_NAMES: dict[str, str] = {
    "en": "English",
    "es": "Spanish",
    "fr": "French",
    "de": "German",
    "it": "Italian",
    "pt": "Portuguese",
    "zh": "Chinese",
    "ja": "Japanese",
    "ko": "Korean",
}


class CognitiveServicesGCPManager:
    """Same rough shape as ``CognitiveServicesManager`` (sentiment, language, image text)."""

    def __init__(self) -> None:
        self._lang = language_v1.LanguageServiceClient()
        self._vision = vision.ImageAnnotatorClient()

    def _document(self, text: str) -> language_v1.Document:
        return language_v1.Document(
            content=text, type_=language_v1.Document.Type.PLAIN_TEXT
        )

    def analyze_text_sentiment(self, text: str) -> str | None:
        """Return ``positive`` / ``negative`` / ``neutral`` from NL sentiment score."""
        try:
            resp = self._lang.analyze_sentiment(
                request={"document": self._document(text)}
            )
            s = resp.document_sentiment.score
            if s > 0.25:
                label = "positive"
            elif s < -0.25:
                label = "negative"
            else:
                label = "neutral"
            return label
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error analyzing sentiment: {e}")
            return None
        except Exception as e:
            print(f"Error analyzing sentiment: {e}")
            return None

    def detect_language(self, text: str) -> str | None:
        """Human-readable language name when known; else BCP-47 code (Azure returns name)."""
        try:
            resp = self._lang.analyze_sentiment(
                request={"document": self._document(text)}
            )
            code = (resp.language or "").strip() or None
            if not code:
                return None
            return _LANG_NAMES.get(code.split("-")[0].lower(), code)
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error detecting language: {e}")
            return None
        except Exception as e:
            print(f"Error detecting language: {e}")
            return None

    def recognize_text_from_image(self, image_url: str) -> str | None:
        """OCR: returns **full document text** (Azure Read returns a poll URL instead)."""
        try:
            img = vision.Image()
            img.source.image_uri = image_url
            resp = self._vision.document_text_detection(image=img)
            if resp.error.message:
                print(f"Error recognizing text: {resp.error.message}")
                return None
            if resp.full_text_annotation and resp.full_text_annotation.text:
                return resp.full_text_annotation.text.strip() or None
            return None
        except gcp_exceptions.GoogleAPICallError as e:
            print(f"Error recognizing text: {e}")
            return None
        except Exception as e:
            print(f"Error recognizing text: {e}")
            return None
