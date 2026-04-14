use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use async_trait::async_trait;
use reqwest::multipart;

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

pub struct OpenAIProvider {
    api_key: Option<String>,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: Option<String>, endpoint: Option<String>) -> Self {
        Self {
            api_key,
            base_url: endpoint
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(DEFAULT_BASE_URL)
                .trim_end_matches('/')
                .to_string(),
        }
    }

    fn get_api_key(&self) -> Result<&str, ProviderError> {
        self.api_key
            .as_deref()
            .filter(|k| !k.is_empty())
            .ok_or_else(|| ProviderError::MissingApiKey("openai".to_string()))
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    async fn transcribe(
        &self,
        logger: &Logger,
        audio_bytes: &[u8],
        media_type: &str,
        prompt: &str,
        model: &str,
    ) -> Result<String, ProviderError> {
        let api_key = self.get_api_key()?;
        let extension = media_type.split('/').next_back().unwrap_or("wav");

        logger.info(
            "openai",
            "准备发送转写请求",
            Some(serde_json::json!({
                "model": model,
                "media_type": media_type,
                "audio_size_bytes": audio_bytes.len(),
                "base_url": self.base_url,
            })),
        );

        let file_part = multipart::Part::bytes(audio_bytes.to_vec())
            .file_name(format!("audio.{}", extension))
            .mime_str(media_type)
            .map_err(|e| ProviderError::RequestError(e.to_string()))?;

        let form = multipart::Form::new()
            .part("file", file_part)
            .text("model", model.to_string())
            .text("prompt", prompt.to_string());

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/audio/transcriptions", self.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                logger.error(
                    "openai",
                    "转写请求失败",
                    Some(serde_json::json!({ "error": e.to_string() })),
                );
                ProviderError::RequestError(e.to_string())
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let err = format!("HTTP {} - {}", status, body);
            logger.error(
                "openai",
                "转写请求失败",
                Some(serde_json::json!({ "error": &err })),
            );
            return Err(ProviderError::RequestError(err));
        }

        #[derive(serde::Deserialize)]
        struct TranscriptionResponse {
            text: String,
        }

        let resp: TranscriptionResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::RequestError(format!("Failed to parse response: {}", e)))?;

        logger.info(
            "openai",
            "转写请求成功",
            Some(serde_json::json!({ "response_length": resp.text.len() })),
        );

        Ok(resp.text)
    }

    async fn complete_text(
        &self,
        logger: &Logger,
        prompt: &str,
        model: &str,
        thinking: ThinkingMode,
    ) -> Result<String, ProviderError> {
        let api_key = self.get_api_key()?;

        logger.info(
            "openai",
            "准备发送文本请求",
            Some(serde_json::json!({ "model": model, "base_url": self.base_url })),
        );

        let mut body = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
        });

        match thinking {
            ThinkingMode::Enabled => {
                body["reasoning_effort"] = serde_json::json!("high");
            }
            ThinkingMode::Disabled => {
                body["reasoning_effort"] = serde_json::json!("none");
            }
            ThinkingMode::Default => {}
        }

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                logger.error(
                    "openai",
                    "文本请求失败",
                    Some(serde_json::json!({ "error": e.to_string() })),
                );
                ProviderError::RequestError(e.to_string())
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let err = format!("HTTP {} - {}", status, body);
            logger.error(
                "openai",
                "文本请求失败",
                Some(serde_json::json!({ "error": &err })),
            );
            return Err(ProviderError::RequestError(err));
        }

        #[derive(serde::Deserialize)]
        struct ChatResponse {
            choices: Vec<Choice>,
        }
        #[derive(serde::Deserialize)]
        struct Choice {
            message: Message,
        }
        #[derive(serde::Deserialize)]
        struct Message {
            content: String,
        }

        let resp: ChatResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::RequestError(format!("Failed to parse response: {}", e)))?;

        let text = resp
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        logger.info(
            "openai",
            "文本请求成功",
            Some(serde_json::json!({ "response_length": text.len() })),
        );

        Ok(text)
    }

    fn needs_api_key(&self) -> bool {
        true
    }

    fn default_models() -> Vec<ModelConfig> {
        vec![ModelConfig {
            name: "gpt-4o".to_string(),
            capabilities: vec!["chat".to_string()],
            verified: None,
        }]
    }
}
