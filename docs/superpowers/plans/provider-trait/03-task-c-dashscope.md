# 子任务 C: DashScope Provider 实现

> **依赖**: B (Provider Trait 基础架构) | **阶段**: Phase 2 (可与 D, E 并行) | **复杂度**: 高

## 目标

实现 DashScope (阿里云通义) Provider 的 `transcribe()` 和 `complete_text()` 方法，使用 `reqwest` 直接调用 DashScope OpenAI 兼容 API，替换现有的 rig-core 调用。

## 涉及文件

| 文件 | 操作 |
|------|------|
| `src-tauri/src/providers/dashscope.rs` | 修改 (替换 stub) |

## API 参考

DashScope 提供 OpenAI 兼容接口：

| 功能 | URL | 方法 |
|------|-----|------|
| 音频转写 | `https://dashscope.aliyuncs.com/compatible-mode/v1/audio/transcriptions` | POST (multipart) |
| 文本补全 | `https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions` | POST (JSON) |

认证: `Authorization: Bearer {api_key}`

### 转写请求
- Content-Type: `multipart/form-data`
- Fields: `file` (binary), `model` (string), `prompt` (string)
- Response: `{ "text": "..." }`

### 文本补全请求
- Content-Type: `application/json`
- Body: `{ "model": "...", "messages": [{"role": "user", "content": "..."}], "enable_thinking": true/false }`
- Response: `{ "choices": [{"message": {"content": "..."}}] }`

## 步骤

- [ ] **Step 1: 替换 `providers/dashscope.rs` 完整实现**

```rust
use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use async_trait::async_trait;
use reqwest::multipart;

const DASHSCOPE_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

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

        let resp: TranscriptionResponse = response.json().await.map_err(|e| {
            ProviderError::RequestError(format!("Failed to parse response: {}", e))
        })?;

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

        let resp: ChatResponse = response.json().await.map_err(|e| {
            ProviderError::RequestError(format!("Failed to parse response: {}", e))
        })?;

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
            name: "qwen2-audio-instruct".to_string(),
            capabilities: vec!["transcription".to_string()],
            verified: None,
        }]
    }
}
```

- [ ] **Step 2: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | grep -E "(dashscope|error)"
```

预期：dashscope.rs 无编译错误。

## 提交

```bash
git add src-tauri/src/providers/dashscope.rs
git commit -m "feat: implement DashScope provider with reqwest HTTP client"
```
