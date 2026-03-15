//! HTTP client for Claude API -- infrastructure adapter for LLM-assisted migration.
//!
//! This module is only compiled when the `llm-fallback` feature is enabled.
//! It implements the `LlmFallbackPort` trait using the Anthropic Messages API.

use crate::domain::ports::{LlmFallbackContext, LlmFallbackError, LlmFallbackPort};
use crate::domain::value_objects::{Language, SourceCloud};

/// Claude API client for LLM-assisted migration fallback.
pub struct ClaudeClient {
    api_key: String,
    model: String,
    base_url: String,
}

impl ClaudeClient {
    /// Create a new Claude API client.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "claude-sonnet-4-20250514".to_string(),
            base_url: "https://api.anthropic.com/v1/messages".to_string(),
        }
    }

    /// Create with a custom model.
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

impl LlmFallbackPort for ClaudeClient {
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
            "model": self.model,
            "max_tokens": 8192,
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        });

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(&self.base_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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

        // Extract text from Claude's response
        let text = body["content"][0]["text"]
            .as_str()
            .ok_or_else(|| LlmFallbackError::ParseError("No text in response".into()))?;

        // Extract code from markdown code block if present
        let code = extract_code_block(text).unwrap_or(text);

        Ok(code.to_string())
    }
}

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
