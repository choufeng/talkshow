use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use async_trait::async_trait;

const DEFAULT_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";

pub struct ZhipuProvider {
    api_key: Option<String>,
    base_url: String,
}

impl ZhipuProvider {
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
            .ok_or_else(|| ProviderError::MissingApiKey("zhipu".to_string()))
    }
}

#[async_trait]
impl Provider for ZhipuProvider {
    async fn transcribe(
        &self,
        _logger: &Logger,
        _audio_bytes: &[u8],
        _media_type: &str,
        _prompt: &str,
        _model: &str,
    ) -> Result<String, ProviderError> {
        Err(ProviderError::UnsupportedOperation(
            "智谱 AI 不支持语音转写".to_string(),
        ))
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
            "zhipu",
            "准备发送文本请求",
            Some(serde_json::json!({ "model": model, "base_url": self.base_url })),
        );

        let mut body = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
        });

        match thinking {
            ThinkingMode::Enabled => {
                body["thinking"] = serde_json::json!({ "type": "enabled" });
            }
            ThinkingMode::Disabled => {
                body["thinking"] = serde_json::json!({ "type": "disabled" });
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
                    "zhipu",
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
                "zhipu",
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
            "zhipu",
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
            name: "glm-4.7-flash".to_string(),
            capabilities: vec!["chat".to_string()],
            verified: None,
        }]
    }
}
