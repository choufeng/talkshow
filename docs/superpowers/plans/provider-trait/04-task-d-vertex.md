# 子任务 D: Vertex AI Provider 实现

> **依赖**: B (Provider Trait 基础架构) | **阶段**: Phase 2 (可与 C, E 并行) | **复杂度**: 高

## 目标

实现 Vertex AI Provider，使用 Google Cloud Application Default Credentials (ADC) 获取 access token，直接调用 Vertex AI REST API。

## 涉及文件

| 文件 | 操作 |
|------|------|
| `src-tauri/src/providers/vertex.rs` | 修改 (替换 stub) |

## API 参考

Vertex AI Gemini API:
- URL pattern: `https://aiplatform.googleapis.com/v1/projects/{project}/locations/{location}/publishers/google/models/{model}:generateContent`
- Auth: `Authorization: Bearer {access_token}`
- 获取 token: `gcloud auth application-default print-access-token` 或环境变量

### 请求格式 (音频转写 & 文本补全共用)
```json
{
  "contents": [{
    "role": "user",
    "parts": [
      {"inline_data": {"mime_type": "audio/wav", "data": "<base64>"}},
      {"text": "prompt"}
    ]
  }],
  "generationConfig": {}
}
```

### 响应格式
```json
{
  "candidates": [{
    "content": {
      "parts": [{"text": "response text"}]
    }
  }]
}
```

## 步骤

- [ ] **Step 1: 替换 `providers/vertex.rs` 完整实现**

```rust
use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use async_trait::async_trait;
use base64::Engine;
use std::sync::{Arc, Mutex};

const VERTEX_BASE_URL: &str = "https://aiplatform.googleapis.com/v1";

type TokenCache = Arc<Mutex<Option<(String, std::time::Instant)>>>;

pub struct VertexAIProvider {
    token_cache: TokenCache,
}

impl VertexAIProvider {
    pub fn new() -> Self {
        Self {
            token_cache: Arc::new(Mutex::new(None)),
        }
    }

    fn get_project_and_location() -> Result<(String, String), ProviderError> {
        let project = std::env::var("GOOGLE_CLOUD_PROJECT").map_err(|_| {
            ProviderError::RequestError("GOOGLE_CLOUD_PROJECT not set".to_string())
        })?;
        let location = std::env::var("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|_| "global".to_string());
        Ok((project, location))
    }

    async fn get_access_token(&self) -> Result<String, ProviderError> {
        {
            let guard = self.token_cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((token, expires_at)) = guard.as_ref() {
                if expires_at > &std::time::Instant::now() {
                    return Ok(token.clone());
                }
            }
        }

        let output = std::process::Command::new("gcloud")
            .args(["auth", "application-default", "print-access-token"])
            .output()
            .map_err(|e| ProviderError::RequestError(format!("Failed to run gcloud: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ProviderError::RequestError(format!(
                "gcloud auth failed: {}",
                stderr
            )));
        }

        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(3500);

        {
            let mut guard = self.token_cache.lock().unwrap_or_else(|e| e.into_inner());
            *guard = Some((token.clone(), expires_at));
        }

        Ok(token)
    }

    fn build_url(project: &str, location: &str, model: &str) -> String {
        format!(
            "{}/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
            VERTEX_BASE_URL, project, location, model
        )
    }
}

#[async_trait]
impl Provider for VertexAIProvider {
    async fn transcribe(
        &self,
        logger: &Logger,
        audio_bytes: &[u8],
        media_type: &str,
        prompt: &str,
        model: &str,
    ) -> Result<String, ProviderError> {
        let token = self.get_access_token().await?;
        let (project, location) = Self::get_project_and_location()?;
        let url = Self::build_url(&project, &location, model);

        let audio_b64 = base64::engine::general_purpose::STANDARD.encode(audio_bytes);

        logger.info(
            "vertex",
            "准备发送音频请求",
            Some(serde_json::json!({
                "model": model,
                "media_type": media_type,
                "audio_size_b64": audio_b64.len(),
            })),
        );

        let body = serde_json::json!({
            "contents": [{
                "role": "user",
                "parts": [
                    {"inline_data": {"mime_type": media_type, "data": audio_b64}},
                    {"text": prompt}
                ]
            }],
            "generationConfig": {}
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                logger.error(
                    "vertex",
                    "音频请求失败",
                    Some(serde_json::json!({ "error": e.to_string() })),
                );
                ProviderError::RequestError(e.to_string())
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let err = format!("HTTP {} - {}", status, body);
            logger.error(
                "vertex",
                "音频请求失败",
                Some(serde_json::json!({ "error": &err })),
            );
            return Err(ProviderError::RequestError(err));
        }

        let resp: serde_json::Value = response.json().await.map_err(|e| {
            ProviderError::RequestError(format!("Failed to parse response: {}", e))
        })?;

        let text = extract_text_from_vertex_response(&resp);

        logger.info(
            "vertex",
            "音频请求成功",
            Some(serde_json::json!({ "response_length": text.len() })),
        );

        Ok(text)
    }

    async fn complete_text(
        &self,
        logger: &Logger,
        prompt: &str,
        model: &str,
        _thinking: ThinkingMode,
    ) -> Result<String, ProviderError> {
        let token = self.get_access_token().await?;
        let (project, location) = Self::get_project_and_location()?;
        let url = Self::build_url(&project, &location, model);

        logger.info(
            "vertex",
            "准备发送文本请求",
            Some(serde_json::json!({ "model": model })),
        );

        let body = serde_json::json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": prompt}]
            }],
            "generationConfig": {}
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                logger.error(
                    "vertex",
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
                "vertex",
                "文本请求失败",
                Some(serde_json::json!({ "error": &err })),
            );
            return Err(ProviderError::RequestError(err));
        }

        let resp: serde_json::Value = response.json().await.map_err(|e| {
            ProviderError::RequestError(format!("Failed to parse response: {}", e))
        })?;

        let text = extract_text_from_vertex_response(&resp);

        logger.info(
            "vertex",
            "文本请求成功",
            Some(serde_json::json!({ "response_length": text.len() })),
        );

        Ok(text)
    }

    fn needs_api_key(&self) -> bool {
        false
    }

    fn default_models() -> Vec<ModelConfig> {
        vec![ModelConfig {
            name: "gemini-2.0-flash".to_string(),
            capabilities: vec!["transcription".to_string(), "chat".to_string()],
            verified: None,
        }]
    }
}

fn extract_text_from_vertex_response(resp: &serde_json::Value) -> String {
    resp.get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array())
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}
```

- [ ] **Step 2: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | grep -E "(vertex|error)"
```

预期：vertex.rs 无编译错误。确认 `base64` crate 已在 Cargo.toml 中。

## 提交

```bash
git add src-tauri/src/providers/vertex.rs
git commit -m "feat: implement Vertex AI provider with ADC auth and REST API"
```
