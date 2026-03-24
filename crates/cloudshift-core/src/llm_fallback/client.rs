//! HTTP client for Gemini API -- infrastructure adapter for LLM-assisted migration.
//!
//! This module is only compiled when the `llm-fallback` feature is enabled.
//! It implements the `LlmFallbackPort` trait using the Google Gemini API.

use crate::domain::ports::{LlmFallbackContext, LlmFallbackError, LlmFallbackPort};
use crate::domain::value_objects::{Language, SourceCloud};

/// Gemini API client for LLM-assisted migration fallback.
pub struct GeminiClient {
    api_key: String,
    model: String,
}

/// Default Gemini model for migration fallback.
const DEFAULT_LLM_MODEL: &str = "gemini-2.5-flash";

impl GeminiClient {
    /// Create a new Gemini API client.
    /// Model can be overridden via `CLOUDSHIFT_LLM_MODEL` env var.
    pub fn new(api_key: String) -> Self {
        let model =
            std::env::var("CLOUDSHIFT_LLM_MODEL").unwrap_or_else(|_| DEFAULT_LLM_MODEL.to_string());
        Self { api_key, model }
    }

    /// Create with a custom model (overrides env default).
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    fn endpoint(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        )
    }
}

impl LlmFallbackPort for GeminiClient {
    fn complete_migration(
        &self,
        source: &str,
        language: Language,
        source_cloud: SourceCloud,
        context: &LlmFallbackContext,
    ) -> Result<String, LlmFallbackError> {
        let remaining = super::detector::detect_remaining_cloud_refs(source, language);

        let prompt = super::prompt::build_fallback_prompt(
            source,
            &context.original_source,
            language,
            source_cloud,
            &remaining,
            &context.applied_patterns,
        );

        let request_body = serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }],
            "generationConfig": {
                "maxOutputTokens": 8192,
                "temperature": 0.1
            }
        });

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(self.endpoint())
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .map_err(|e| LlmFallbackError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(LlmFallbackError::ApiError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let body: serde_json::Value = response
            .json()
            .map_err(|e| LlmFallbackError::ParseError(e.to_string()))?;

        // Extract text from Gemini's response
        let text = body["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| LlmFallbackError::ParseError("No text in Gemini response".into()))?;

        // Extract code from markdown code block if present
        let code = extract_code_block(text).unwrap_or(text);

        Ok(code.to_string())
    }
}

// Keep backward-compatible type alias
pub type ClaudeClient = GeminiClient;

/// Extract code from a markdown code block.
fn extract_code_block(text: &str) -> Option<&str> {
    let start_markers = [
        "```python\n",
        "```typescript\n",
        "```javascript\n",
        "```java\n",
        "```go\n",
        "```hcl\n",
        "```\n",
    ];
    for marker in &start_markers {
        if let Some(start) = text.find(marker) {
            let code_start = start + marker.len();
            if let Some(end) = text[code_start..].find("```") {
                return Some(&text[code_start..code_start + end]);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_python_code_block() {
        let text = "Here is the code:\n```python\nfrom google.cloud import storage\n```\nDone.";
        let code = extract_code_block(text).unwrap();
        assert_eq!(code, "from google.cloud import storage\n");
    }

    #[test]
    fn extract_generic_code_block() {
        let text = "```\nsome code\n```";
        let code = extract_code_block(text).unwrap();
        assert_eq!(code, "some code\n");
    }

    #[test]
    fn no_code_block_returns_none() {
        let text = "just plain text with no code blocks";
        assert!(extract_code_block(text).is_none());
    }
}
