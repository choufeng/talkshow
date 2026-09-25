use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use async_trait::async_trait;
use reqwest::multipart;

const DASHSCOPE_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

/// qwen3-asr 系列走 chat/completions 协议（实测 base64 可用），不走 /audio/transcriptions
fn is_qwen_asr_model(model: &str) -> bool {
    model.starts_with("qwen3-asr") || model.starts_with("qwen-audio")
}

fn build_asr_body(model: &str, media_type: &str, audio_bytes: &[u8]) -> serde_json::Value {
    use base64::Engine;
    let format = media_type.split('/').next_back().unwrap_or("wav");
    let b64 = base64::engine::general_purpose::STANDARD.encode(audio_bytes);
    serde_json::json!({
        "model": model,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "input_audio",
                "input_audio": {
                    "data": format!("data:{};base64,{}", media_type, b64),
                    "format": format,
                }
            }]
        }]
    })
}

pub struct DashScopeProvider {
    pub api_key: Option<String>,
}

impl DashScopeProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }

    fn get_api_key(&self) -> Result<&str, ProviderError> {
        self.api_key
            .as_deref()
            .filter(|k| !k.is_empty())
            .ok_or_else(|| ProviderError::MissingApiKey("dashscope".to_string()))
    }
}

#[async_trait]
impl Provider for DashScopeProvider {
    async fn transcribe(
        &self,
        logger: &Logger,
        audio_bytes: &[u8],
        media_type: &str,
        prompt: &str,
        model: &str,
    ) -> Result<String, ProviderError> {
        let api_key = self.get_api_key()?;

        if is_qwen_asr_model(model) {
            return self
                .transcribe_via_chat(logger, api_key, audio_bytes, media_type, model)
                .await;
        }

        let extension = media_type.split('/').next_back().unwrap_or("wav");

        logger.info(
            "dashscope",
            "准备发送转写请求",
            Some(serde_json::json!({
                "model": model,
                "media_type": media_type,
                "audio_size_bytes": audio_bytes.len(),
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
            .post(format!("{}/audio/transcriptions", DASHSCOPE_BASE_URL))
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                logger.error(
                    "dashscope",
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
                "dashscope",
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
            "dashscope",
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
            "dashscope",
            "准备发送文本请求",
            Some(serde_json::json!({ "model": model })),
        );

        let mut body = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
        });

        match thinking {
            ThinkingMode::Enabled => {
                body["enable_thinking"] = serde_json::json!(true);
            }
            ThinkingMode::Disabled => {
                body["enable_thinking"] = serde_json::json!(false);
            }
            ThinkingMode::Default => {}
        }

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/chat/completions", DASHSCOPE_BASE_URL))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                logger.error(
                    "dashscope",
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
                "dashscope",
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
            "dashscope",
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
            name: "qwen3.5-flash".to_string(),
            capabilities: vec!["chat".to_string()],
            verified: None,
        }]
    }
}

impl DashScopeProvider {
    /// qwen3-asr 系列：chat/completions + input_audio base64（官方文档仅写公网 URL，实测 base64 data URI 可用）
    async fn transcribe_via_chat(
        &self,
        logger: &Logger,
        api_key: &str,
        audio_bytes: &[u8],
        media_type: &str,
        model: &str,
    ) -> Result<String, ProviderError> {
        logger.info(
            "dashscope",
            "准备发送 ASR chat 转写请求",
            Some(serde_json::json!({
                "model": model,
                "media_type": media_type,
                "audio_size_bytes": audio_bytes.len(),
            })),
        );

        let body = build_asr_body(model, media_type, audio_bytes);
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/chat/completions", DASHSCOPE_BASE_URL))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::RequestError(format!(
                "HTTP {} - {}",
                status, text
            )));
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
            "dashscope",
            "ASR chat 转写请求成功",
            Some(serde_json::json!({ "response_length": text.len() })),
        );

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asr_model_routing() {
        assert!(is_qwen_asr_model("qwen3-asr-flash"));
        assert!(is_qwen_asr_model("qwen3-asr-flash-us"));
        assert!(is_qwen_asr_model("qwen-audio-3.0-asr-flash"));
        assert!(!is_qwen_asr_model("qwen3.5-flash"));
        assert!(!is_qwen_asr_model("fun-asr-flash")); // fun-asr 走原生协议，暂不支持
    }

    #[test]
    fn asr_body_shape() {
        let body = build_asr_body("qwen3-asr-flash", "audio/wav", b"RIFF");
        assert_eq!(body["model"], "qwen3-asr-flash");
        let audio = &body["messages"][0]["content"][0]["input_audio"];
        assert_eq!(audio["format"], "wav");
        assert_eq!(
            audio["data"].as_str().unwrap(),
            "data:audio/wav;base64,UklGRg=="
        );
    }
}
