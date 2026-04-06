# Provider Trait Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace rig-core/rig-vertexai dependencies with independent Provider trait implementations, remove Add Provider UI, and hardcode provider endpoints.

**Architecture:** Define a `Provider` trait with `transcribe()` and `complete_text()` async methods. Each service (DashScope, VertexAI, SenseVoice) gets its own struct implementing this trait. The `ProviderConfig` struct is simplified by removing `provider_type` and `endpoint` fields. Frontend removes Add Provider and Endpoint editing.

**Tech Stack:** Rust (reqwest, async-trait, serde), TypeScript/Svelte (Tauri IPC)

---

## File Structure

### New Files
- `src-tauri/src/providers/mod.rs` — Provider trait, ProviderContext, create_provider factory
- `src-tauri/src/providers/dashscope.rs` — DashScope (通义) HTTP implementation
- `src-tauri/src/providers/vertex.rs` — Vertex AI HTTP implementation (ADC auth)
- `src-tauri/src/providers/sensevoice.rs` — SenseVoice local inference wrapper

### Modified Files
- `src-tauri/Cargo.toml` — Remove rig-core, rig-vertexai
- `src-tauri/src/ai.rs` — Simplify to thin wrapper calling Provider trait
- `src-tauri/src/config.rs` — Remove provider_type, endpoint from ProviderConfig; update validation/merge/migration
- `src-tauri/src/lib.rs` — Replace VertexClientState with ProviderContext; update all call sites
- `src-tauri/src/llm_client.rs` — Update LlmClient trait (remove endpoint param)
- `src-tauri/src/real_llm_client.rs` — Update to use Provider trait
- `src-tauri/src/skills.rs` — Use Provider trait instead of rig
- `src-tauri/src/translation.rs` — Use Provider trait instead of rig
- `src/lib/stores/config.ts` — Remove type/endpoint from ProviderConfig
- `src/routes/models/+page.svelte` — Remove Add Provider, Endpoint, simplify
- `src/lib/components/onboarding/steps/ProviderConfigStep.svelte` — Remove Add Provider dialog, simplify

---

### Task 1: Create Provider Trait and Module Structure

**Files:**
- Create: `src-tauri/src/providers/mod.rs`

- [ ] **Step 1: Create providers module with trait definition**

```rust
// src-tauri/src/providers/mod.rs
pub mod dashscope;
pub mod sensevoice;
pub mod vertex;

use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::sensevoice::SenseVoiceEngine;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingMode {
    Default,
    Enabled,
    Disabled,
}

#[derive(Debug)]
pub enum ProviderError {
    ProviderNotFound(String),
    MissingApiKey(String),
    FileReadError(String),
    RequestError(String),
    UnsupportedOperation(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::ProviderNotFound(id) => write!(f, "Provider not found: {}", id),
            ProviderError::MissingApiKey(id) => {
                write!(f, "Missing API key for provider: {}", id)
            }
            ProviderError::FileReadError(e) => write!(f, "Failed to read audio file: {}", e),
            ProviderError::RequestError(e) => write!(f, "AI request failed: {}", e),
            ProviderError::UnsupportedOperation(op) => {
                write!(f, "Unsupported operation: {}", op)
            }
        }
    }
}

impl std::error::Error for ProviderError {}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn transcribe(
        &self,
        logger: &Logger,
        audio_bytes: &[u8],
        media_type: &str,
        prompt: &str,
        model: &str,
    ) -> Result<String, ProviderError>;

    async fn complete_text(
        &self,
        logger: &Logger,
        prompt: &str,
        model: &str,
        thinking: ThinkingMode,
    ) -> Result<String, ProviderError>;

    fn needs_api_key(&self) -> bool;
    fn default_models() -> Vec<ModelConfig>;
}

pub struct ProviderContext {
    pub sensevoice_engine: Arc<Mutex<Option<SenseVoiceEngine>>>,
}

impl ProviderContext {
    pub fn new() -> Self {
        ProviderContext {
            sensevoice_engine: Arc::new(Mutex::new(None)),
        }
    }
}

use crate::config::ProviderConfig;

pub fn create_provider(
    config: &ProviderConfig,
    ctx: &ProviderContext,
) -> Result<Box<dyn Provider>, ProviderError> {
    match config.id.as_str() {
        "dashscope" => Ok(Box::new(dashscope::DashScopeProvider::new(
            config.api_key.clone(),
        ))),
        "vertex" => Ok(Box::new(vertex::VertexAIProvider::new())),
        "sensevoice" => Ok(Box::new(sensevoice::SenseVoiceProvider::new(
            ctx.sensevoice_engine.clone(),
        ))),
        _ => Err(ProviderError::ProviderNotFound(config.id.clone())),
    }
}

pub const PROVIDERS_REQUIRING_KEY: &[&str] = &["dashscope"];

pub fn provider_needs_api_key(id: &str) -> bool {
    PROVIDERS_REQUIRING_KEY.contains(&id)
}

pub const BUILTIN_PROVIDER_IDS: &[&str] = &["dashscope", "vertex", "sensevoice"];

pub fn is_builtin_provider(id: &str) -> bool {
    BUILTIN_PROVIDER_IDS.contains(&id)
}
```

- [ ] **Step 2: Create stub provider files**

```rust
// src-tauri/src/providers/dashscope.rs
use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use async_trait::async_trait;

pub struct DashScopeProvider {
    api_key: Option<String>,
}

impl DashScopeProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl Provider for DashScopeProvider {
    async fn transcribe(
        &self,
        _logger: &Logger,
        _audio_bytes: &[u8],
        _media_type: &str,
        _prompt: &str,
        _model: &str,
    ) -> Result<String, ProviderError> {
        todo!()
    }

    async fn complete_text(
        &self,
        _logger: &Logger,
        _prompt: &str,
        _model: &str,
        _thinking: ThinkingMode,
    ) -> Result<String, ProviderError> {
        todo!()
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

```rust
// src-tauri/src/providers/vertex.rs
use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use async_trait::async_trait;

pub struct VertexAIProvider {}

impl VertexAIProvider {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Provider for VertexAIProvider {
    async fn transcribe(
        &self,
        _logger: &Logger,
        _audio_bytes: &[u8],
        _media_type: &str,
        _prompt: &str,
        _model: &str,
    ) -> Result<String, ProviderError> {
        todo!()
    }

    async fn complete_text(
        &self,
        _logger: &Logger,
        _prompt: &str,
        _model: &str,
        _thinking: ThinkingMode,
    ) -> Result<String, ProviderError> {
        todo!()
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
```

```rust
// src-tauri/src/providers/sensevoice.rs
use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use crate::sensevoice::SenseVoiceEngine;

pub struct SenseVoiceProvider {
    engine: Arc<Mutex<Option<SenseVoiceEngine>>>,
}

impl SenseVoiceProvider {
    pub fn new(engine: Arc<Mutex<Option<SenseVoiceEngine>>>) -> Self {
        Self { engine }
    }
}

#[async_trait]
impl Provider for SenseVoiceProvider {
    async fn transcribe(
        &self,
        _logger: &Logger,
        _audio_bytes: &[u8],
        _media_type: &str,
        _prompt: &str,
        _model: &str,
    ) -> Result<String, ProviderError> {
        todo!()
    }

    async fn complete_text(
        &self,
        _logger: &Logger,
        _prompt: &str,
        _model: &str,
        _thinking: ThinkingMode,
    ) -> Result<String, ProviderError> {
        Err(ProviderError::UnsupportedOperation(
            "SenseVoice does not support text completion".to_string(),
        ))
    }

    fn needs_api_key(&self) -> bool {
        false
    }

    fn default_models() -> Vec<ModelConfig> {
        vec![ModelConfig {
            name: "SenseVoice-Small".to_string(),
            capabilities: vec!["transcription".to_string()],
            verified: None,
        }]
    }
}
```

- [ ] **Step 3: Register module in lib.rs**

Add `mod providers;` to `src-tauri/src/lib.rs` at the top alongside existing mod declarations.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/providers/
git commit -m "feat: add Provider trait and module structure with stubs"
```

---

### Task 2: Implement DashScope Provider

**Files:**
- Modify: `src-tauri/src/providers/dashscope.rs`

- [ ] **Step 1: Implement DashScope transcription (multipart/form-data)**

The DashScope OpenAI-compatible API uses multipart form for audio transcription:
- URL: `https://dashscope.aliyuncs.com/compatible-mode/v1/audio/transcriptions`
- Auth: `Authorization: Bearer {api_key}`
- Body: multipart with file + model + prompt fields

```rust
// src-tauri/src/providers/dashscope.rs
use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use async_trait::async_trait;
use reqwest::multipart;

const DASHSCOPE_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

pub struct DashScopeProvider {
    api_key: Option<String>,
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

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/providers/dashscope.rs
git commit -m "feat: implement DashScope provider with reqwest"
```

---

### Task 3: Implement Vertex AI Provider

**Files:**
- Modify: `src-tauri/src/providers/vertex.rs`

- [ ] **Step 1: Implement Vertex AI provider using ADC auth and REST API**

Vertex AI uses Google Cloud Application Default Credentials. We get an access token via `gcloud auth application-default print-access-token` or the environment, then call the Vertex AI REST API directly.

```rust
// src-tauri/src/providers/vertex.rs
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
        let project = std::env::var("GOOGLE_CLOUD_PROJECT")
            .map_err(|_| ProviderError::RequestError("GOOGLE_CLOUD_PROJECT not set".to_string()))?;
        let location = std::env::var("GOOGLE_CLOUD_LOCATION")
            .unwrap_or_else(|_| "global".to_string());
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
            .args([
                "auth",
                "application-default",
                "print-access-token",
            ])
            .output()
            .map_err(|e| {
                ProviderError::RequestError(format!("Failed to run gcloud: {}", e))
            })?;

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

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/providers/vertex.rs
git commit -m "feat: implement Vertex AI provider with ADC auth"
```

---

### Task 4: Implement SenseVoice Provider Wrapper

**Files:**
- Modify: `src-tauri/src/providers/sensevoice.rs`

- [ ] **Step 1: Implement SenseVoice provider wrapping SenseVoiceEngine**

SenseVoice is a local inference engine. It needs the audio file path (not bytes), so we write audio bytes to a temp file, then call `SenseVoiceEngine::transcribe()`. The engine is shared via `Arc<Mutex<Option<SenseVoiceEngine>>>`.

```rust
// src-tauri/src/providers/sensevoice.rs
use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use crate::sensevoice::SenseVoiceEngine;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

pub struct SenseVoiceProvider {
    pub engine: Arc<Mutex<Option<SenseVoiceEngine>>>,
    pub language: Arc<Mutex<i32>>,
}

impl SenseVoiceProvider {
    pub fn new(engine: Arc<Mutex<Option<SenseVoiceEngine>>>) -> Self {
        Self {
            engine,
            language: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_language(self, language: Arc<Mutex<i32>>) -> Self {
        Self { language, ..self }
    }
}

#[async_trait]
impl Provider for SenseVoiceProvider {
    async fn transcribe(
        &self,
        _logger: &Logger,
        audio_bytes: &[u8],
        _media_type: &str,
        _prompt: &str,
        _model: &str,
    ) -> Result<String, ProviderError> {
        let tmp_dir = std::env::temp_dir().join("talkshow_provider");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let tmp_path = tmp_dir.join(format!("audio_{}.wav", uuid()));

        std::fs::write(&tmp_path, audio_bytes)
            .map_err(|e| ProviderError::FileReadError(e.to_string()))?;

        let lang = self
            .language
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .to_owned();

        let mut guard = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_mut() {
            Some(engine) => engine
                .transcribe(&tmp_path, lang)
                .map_err(|e| ProviderError::RequestError(e.to_string())),
            None => Err(ProviderError::RequestError(
                "SenseVoice engine not initialized".to_string(),
            )),
        }
    }

    async fn complete_text(
        &self,
        _logger: &Logger,
        _prompt: &str,
        _model: &str,
        _thinking: ThinkingMode,
    ) -> Result<String, ProviderError> {
        Err(ProviderError::UnsupportedOperation(
            "SenseVoice does not support text completion".to_string(),
        ))
    }

    fn needs_api_key(&self) -> bool {
        false
    }

    fn default_models() -> Vec<ModelConfig> {
        vec![ModelConfig {
            name: "SenseVoice-Small".to_string(),
            capabilities: vec!["transcription".to_string()],
            verified: None,
        }]
    }
}

fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/providers/sensevoice.rs
git commit -m "feat: implement SenseVoice provider wrapping local engine"
```

---

### Task 5: Simplify ProviderConfig and Update Config Module

**Files:**
- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: Remove provider_type and endpoint from ProviderConfig**

Change `ProviderConfig` struct (line 98-108 in config.rs) from:

```rust
pub struct ProviderConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub name: String,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub models: Vec<ModelConfig>,
}
```

to:

```rust
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_key: Option<String>,
    pub models: Vec<ModelConfig>,
}
```

- [ ] **Step 2: Update builtin_providers() to use new struct**

Replace the `builtin_providers()` function (lines 11-49) to remove provider_type and endpoint:

```rust
fn builtin_providers() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            id: "dashscope".to_string(),
            name: "阿里云".to_string(),
            api_key: Some(String::new()),
            models: vec![ModelConfig {
                name: "qwen2-audio-instruct".to_string(),
                capabilities: vec!["transcription".to_string()],
                verified: None,
            }],
        },
        ProviderConfig {
            id: "vertex".to_string(),
            name: "Vertex AI".to_string(),
            api_key: None,
            models: vec![ModelConfig {
                name: "gemini-2.0-flash".to_string(),
                capabilities: vec!["transcription".to_string(), "chat".to_string()],
                verified: None,
            }],
        },
        ProviderConfig {
            id: "sensevoice".to_string(),
            name: "SenseVoice (本地)".to_string(),
            api_key: None,
            models: vec![ModelConfig {
                name: "SenseVoice-Small".to_string(),
                capabilities: vec!["transcription".to_string()],
                verified: None,
            }],
        },
    ]
}
```

- [ ] **Step 3: Simplify merge_builtin_providers()**

Replace `merge_builtin_providers()` (lines 52-79). Remove the logic that corrects provider_type and endpoint — only correct `name` and fill missing providers:

```rust
fn merge_builtin_providers(mut providers: Vec<ProviderConfig>) -> Vec<ProviderConfig> {
    let builtins = builtin_providers();
    let builtin_map: std::collections::HashMap<String, ProviderConfig> =
        builtins.into_iter().map(|p| (p.id.clone(), p)).collect();

    let builtin_ids: std::collections::HashSet<String> = builtin_map.keys().cloned().collect();
    let user_ids: std::collections::HashSet<String> =
        providers.iter().map(|p| p.id.clone()).collect();

    let missing: Vec<ProviderConfig> = builtin_map
        .values()
        .filter(|p| !user_ids.contains(&p.id))
        .cloned()
        .collect();

    for provider in &mut providers {
        if let Some(builtin) = builtin_map.get(&provider.id)
            && builtin_ids.contains(&provider.id)
        {
            provider.name = builtin.name.clone();
        }
    }

    let mut result = missing;
    result.append(&mut providers);
    result
}
```

- [ ] **Step 4: Simplify validate_config()**

Replace `validate_config()` (lines 432-478). Remove provider_type validation and endpoint URL validation:

```rust
pub fn validate_config(config: &AppConfig) -> Result<(), String> {
    for provider in &config.ai.providers {
        if provider.id.trim().is_empty() {
            return Err("Provider ID cannot be empty".to_string());
        }
        if provider.name.trim().is_empty() {
            return Err(format!(
                "Provider name cannot be empty for '{}'",
                provider.id
            ));
        }
    }

    if config.shortcut.len() > 100 {
        return Err("Shortcut string too long".to_string());
    }
    if config.recording_shortcut.len() > 100 {
        return Err("Recording shortcut string too long".to_string());
    }
    if config.translate_shortcut.len() > 100 {
        return Err("Translate shortcut string too long".to_string());
    }

    Ok(())
}
```

- [ ] **Step 5: Update tests that reference provider_type and endpoint**

Update all test ProviderConfig constructions to remove provider_type and endpoint fields. The affected tests are:
- `test_merge_builtin_providers_adds_missing`
- `test_merge_builtin_providers_corrects_existing` (update assertion to check name instead of endpoint)
- `test_validate_config_rejects_invalid_provider_type` — remove entirely
- `test_validate_config_rejects_non_https_endpoint` — remove entirely
- `test_validate_config_allows_empty_endpoint_for_vertex` — update to remove provider_type/endpoint

- [ ] **Step 6: Verify tests compile and pass**

Run: `cd src-tauri && cargo test --lib config`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "refactor: simplify ProviderConfig, remove type and endpoint"
```

---

### Task 6: Update Cargo.toml — Remove rig Dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Remove rig-core and rig-vertexai lines**

Remove these two lines from `src-tauri/Cargo.toml`:
```
rig-core = "0.33"
rig-vertexai = "0.3"
```

- [ ] **Step 2: Verify it compiles (will fail until ai.rs is updated — that's expected)**

Run: `cd src-tauri && cargo check 2>&1 | head -20`
Expected: Compilation errors in ai.rs, real_llm_client.rs, skills.rs, translation.rs, lib.rs — all referencing rig/rig_vertexai

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore: remove rig-core and rig-vertexai dependencies"
```

---

### Task 7: Rewrite ai.rs to Use Provider Trait

**Files:**
- Modify: `src-tauri/src/ai.rs`

- [ ] **Step 1: Replace entire ai.rs with thin wrapper**

```rust
// src-tauri/src/ai.rs
use crate::config::ProviderConfig;
use crate::logger::Logger;
use crate::providers::{self, ProviderContext, ProviderError, ThinkingMode};
use std::path::Path;

pub use providers::{ProviderError as AiError, ThinkingMode};

pub async fn send_audio_prompt(
    logger: &Logger,
    audio_path: &Path,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
    ctx: &ProviderContext,
) -> Result<String, AiError> {
    let audio_data =
        std::fs::read(audio_path).map_err(|e| AiError::FileReadError(e.to_string()))?;

    let extension = audio_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("wav");
    let media_type = match extension {
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "mp3" => "audio/mp3",
        "ogg" => "audio/ogg",
        _ => "audio/wav",
    };

    send_audio_prompt_from_bytes(logger, &audio_data, media_type, text_prompt, model_name, provider, ctx).await
}

pub async fn send_audio_prompt_from_bytes(
    logger: &Logger,
    audio_bytes: &[u8],
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
    ctx: &ProviderContext,
) -> Result<String, AiError> {
    let p = providers::create_provider(provider, ctx)?;
    p.transcribe(logger, audio_bytes, media_type, text_prompt, model_name)
        .await
}

pub async fn send_text_prompt(
    logger: &Logger,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
    ctx: &ProviderContext,
    thinking: ThinkingMode,
) -> Result<String, AiError> {
    let p = providers::create_provider(provider, ctx)?;
    p.complete_text(logger, text_prompt, model_name, thinking)
        .await
}
```

Note: `AiError` is re-exported from `providers::ProviderError`. The `VertexClientCache` type alias is removed — replaced by `ProviderContext`.

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/ai.rs
git commit -m "refactor: rewrite ai.rs as thin Provider trait wrapper"
```

---

### Task 8: Update lib.rs — Replace VertexClientState with ProviderContext

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Update imports and state structs**

Remove `VertexClientState` struct (line 53-55). Add import for `providers::ProviderContext`.

Change:
```rust
struct VertexClientState {
    client: crate::ai::VertexClientCache,
}
```

To: (remove entirely)

- [ ] **Step 2: Update SenseVoice provider creation in transcribe path**

In the recording completion handler (around line 248), replace the sensevoice-specific code block with Provider trait call. Replace:

```rust
let text_result = if provider.provider_type == "sensevoice" {
    // ... 30 lines of sensevoice-specific code ...
} else {
    // ... ai::send_audio_prompt with vertex_cache ...
};
```

With:

```rust
let provider_ctx = h.state::<ProviderContext>();
let text_result = if provider.id == "sensevoice" {
    let sv_provider = providers::sensevoice::SenseVoiceProvider::new(provider_ctx.sensevoice_engine.clone());
    let lang = h.state::<SenseVoiceState>().language.lock().unwrap_or_else(|e| e.into_inner()).to_owned();
    let sv_provider = sv_provider.with_language(lang);
    sv_provider.transcribe(&logger, &audio_data, media_type, "请将这段音频转录为文字，只输出转录结果，不要添加任何解释。", &model_name).await.map_err(|e| e.to_string())
} else {
    let prompt = "请将这段音频转录为文字，只输出转录结果，不要添加任何解释。";
    ai::send_audio_prompt(
        &logger,
        &audio_path,
        prompt,
        &model_name,
        &provider,
        &provider_ctx,
    )
    .await
    .map_err(|e| e.to_string())
};
```

Wait — for sensevoice the audio is a file path, not bytes. We need to read the file:

```rust
let text_result = if provider.id == "sensevoice" {
    let audio_data = std::fs::read(&audio_path).map_err(|e| format!("Failed to read audio: {}", e))?;
    let extension = audio_path.extension().and_then(|e| e.to_str()).unwrap_or("wav");
    let media_type = match extension {
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "mp3" => "audio/mp3",
        "ogg" => "audio/ogg",
        _ => "audio/wav",
    };
    let sv_provider = providers::sensevoice::SenseVoiceProvider::new(
        provider_ctx.sensevoice_engine.clone()
    );
    let lang = h.state::<SenseVoiceState>().language.lock().unwrap_or_else(|e| e.into_inner()).to_owned();
    let sv_provider = sv_provider.with_language(lang);
    sv_provider.transcribe(&logger, &audio_data, media_type, "", &model_name).await.map_err(|e| e.to_string())
} else {
    let prompt = "请将这段音频转录为文字，只输出转录结果，不要添加任何解释。";
    ai::send_audio_prompt(
        &logger,
        &audio_path,
        prompt,
        &model_name,
        &provider,
        &provider_ctx,
    )
    .await
    .map_err(|e| e.to_string())
};
```

- [ ] **Step 3: Replace all `VertexClientState` references with `ProviderContext`**

Search and replace all occurrences of:
- `h.state::<VertexClientState>().client` → `h.state::<ProviderContext>().as_ref()` (or pass `&ProviderContext`)
- `VertexClientState { client: Arc::new(Mutex::new(None)) }` → `ProviderContext::new()`

Specific locations:
1. `skills::process_with_skills` call (line ~312-318): Replace `&h.state::<VertexClientState>().client` with `&h.state::<ProviderContext>()`
2. `translation::translate_text` call (line ~346-355): Same replacement
3. `test_model_connectivity` command (line ~824-866): Replace `vertex_cache` with `ProviderContext`
4. Setup block (line ~1556-1559): Replace `VertexClientState` initialization with `ProviderContext::new()`

- [ ] **Step 4: Update skills.rs and translation.rs call sites in lib.rs**

Both `process_with_skills` and `translate_text` currently take `&VertexClientCache`. They need to take `&ProviderContext` instead. Update the call signatures in the next task, but here update the call sites in lib.rs to pass `&ProviderContext`.

- [ ] **Step 5: Update the test_model_connectivity command**

Replace the `provider.provider_type == "sensevoice"` check with `provider.id == "sensevoice"`, and pass `ProviderContext` instead of `VertexClientCache`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "refactor: replace VertexClientState with ProviderContext in lib.rs"
```

---

### Task 9: Update skills.rs, translation.rs, real_llm_client.rs, llm_client.rs

**Files:**
- Modify: `src-tauri/src/skills.rs`
- Modify: `src-tauri/src/translation.rs`
- Modify: `src-tauri/src/real_llm_client.rs`
- Modify: `src-tauri/src/llm_client.rs`

- [ ] **Step 1: Update skills.rs — replace VertexClientCache with ProviderContext**

Change the `process_with_skills` function signature from:
```rust
pub async fn process_with_skills(
    ...
    vertex_cache: &VertexClientCache,
    ...
) -> Result<String, String>
```

To:
```rust
pub async fn process_with_skills(
    logger: &Logger,
    skills_config: &SkillsConfig,
    transcription_config: &crate::config::TranscriptionConfig,
    providers: &[ProviderConfig],
    transcription: &str,
    ctx: &crate::providers::ProviderContext,
    selected_text: Option<&str>,
) -> Result<String, String>
```

Remove the `type VertexClientCache` line. Update the inner call to `crate::ai::send_text_prompt` to pass `ctx` instead of `vertex_cache`.

Remove the `endpoint` field usage in `process_with_skills_client` — the `LlmClient` trait will be updated in step 3.

- [ ] **Step 2: Update translation.rs — replace VertexClientCache with ProviderContext**

Same pattern as skills.rs. Change `translate_text` signature to take `&crate::providers::ProviderContext` instead of `&VertexClientCache`.

- [ ] **Step 3: Update llm_client.rs — remove endpoint parameter**

```rust
// src-tauri/src/llm_client.rs
use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
#[allow(dead_code)]
pub trait LlmClient: Send + Sync {
    async fn send_text(
        &self,
        prompt: &str,
        model_name: &str,
        provider_id: &str,
    ) -> Result<String, String>;

    async fn send_audio(
        &self,
        audio_bytes: &[u8],
        media_type: &str,
        text_prompt: &str,
        model_name: &str,
        provider_id: &str,
    ) -> Result<String, String>;
}
```

- [ ] **Step 4: Update real_llm_client.rs**

```rust
// src-tauri/src/real_llm_client.rs
use crate::ai::{ThinkingMode, send_audio_prompt_from_bytes, send_text_prompt};
use crate::config::ProviderConfig;
use crate::llm_client::LlmClient;
use crate::logger::Logger;
use crate::providers::ProviderContext;

#[allow(dead_code)]
pub struct RealLlmClient<'a> {
    logger: &'a Logger,
    ctx: &'a ProviderContext,
}

impl<'a> RealLlmClient<'a> {
    #[allow(dead_code)]
    pub fn new(logger: &'a Logger, ctx: &'a ProviderContext) -> Self {
        Self { logger, ctx }
    }
}

#[async_trait::async_trait]
impl LlmClient for RealLlmClient<'_> {
    async fn send_text(
        &self,
        prompt: &str,
        model_name: &str,
        provider_id: &str,
    ) -> Result<String, String> {
        let provider = ProviderConfig {
            id: provider_id.to_string(),
            name: provider_id.to_string(),
            api_key: None,
            models: vec![],
        };
        send_text_prompt(
            self.logger,
            prompt,
            model_name,
            &provider,
            self.ctx,
            ThinkingMode::Disabled,
        )
        .await
        .map_err(|e| e.to_string())
    }

    async fn send_audio(
        &self,
        audio_bytes: &[u8],
        media_type: &str,
        text_prompt: &str,
        model_name: &str,
        provider_id: &str,
    ) -> Result<String, String> {
        let provider = ProviderConfig {
            id: provider_id.to_string(),
            name: provider_id.to_string(),
            api_key: None,
            models: vec![],
        };
        send_audio_prompt_from_bytes(
            self.logger,
            audio_bytes,
            media_type,
            text_prompt,
            model_name,
            &provider,
            self.ctx,
        )
        .await
        .map_err(|e| e.to_string())
    }
}
```

- [ ] **Step 5: Update test mock calls in skills.rs**

Update `MockLlmClient` calls in skills.rs tests from `.expect_send_text().returning(|_, _, _, _| ...)` to `.expect_send_text().returning(|_, _, _| ...)` (remove endpoint param).

Update `process_with_skills_client` to match new `LlmClient` trait signature — remove endpoint parameter from `client.send_text()` call.

- [ ] **Step 6: Update test mock calls in translation.rs**

Same — update mock expectations to remove endpoint param.

- [ ] **Step 7: Verify all tests compile and pass**

Run: `cd src-tauri && cargo test`
Expected: All tests pass

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/skills.rs src-tauri/src/translation.rs src-tauri/src/real_llm_client.rs src-tauri/src/llm_client.rs
git commit -m "refactor: update skills/translation/llm_client to use ProviderContext"
```

---

### Task 10: Update Frontend — Simplify ProviderConfig Type and Stores

**Files:**
- Modify: `src/lib/stores/config.ts`

- [ ] **Step 1: Update ProviderConfig interface**

Change:
```typescript
export interface ProviderConfig {
  id: string;
  type: string;
  name: string;
  endpoint: string;
  api_key?: string;
  models: ModelConfig[];
}
```

To:
```typescript
export interface ProviderConfig {
  id: string;
  name: string;
  api_key?: string;
  models: ModelConfig[];
}
```

- [ ] **Step 2: Update BUILTIN_PROVIDERS**

Remove `type` and `endpoint` from all entries:

```typescript
export const BUILTIN_PROVIDERS: ProviderConfig[] = [
  {
    id: 'vertex',
    name: 'Vertex AI',
    models: [{ name: 'gemini-2.0-flash', capabilities: ['transcription', 'chat'] }]
  },
  {
    id: 'dashscope',
    name: '阿里云',
    api_key: '',
    models: [{ name: 'qwen2-audio-instruct', capabilities: ['transcription'] }]
  },
  {
    id: 'sensevoice',
    name: 'SenseVoice (本地)',
    models: [{ name: 'SenseVoice-Small', capabilities: ['transcription'] }]
  }
];
```

- [ ] **Step 3: Update mergeBuiltinProviders**

Remove type/endpoint correction:

```typescript
function mergeBuiltinProviders(providers: ProviderConfig[]): ProviderConfig[] {
  const userIds = new Set(providers.map((p) => p.id));
  const missing = BUILTIN_PROVIDERS.filter((p) => !userIds.has(p.id));
  const corrected = providers.map((p) => {
    const builtin = BUILTIN_PROVIDERS.find((b) => b.id === p.id);
    if (builtin) {
      return { ...p, name: builtin.name };
    }
    return p;
  });
  return [...missing, ...corrected];
}
```

- [ ] **Step 4: Add helper constants**

```typescript
export const PROVIDERS_REQUIRING_KEY = ['dashscope'];
export function needsApiKey(providerId: string): boolean {
  return PROVIDERS_REQUIRING_KEY.includes(providerId);
}
```

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/config.ts
git commit -m "refactor: simplify ProviderConfig type in frontend stores"
```

---

### Task 11: Update Models Page — Remove Add Provider and Endpoint

**Files:**
- Modify: `src/routes/models/+page.svelte`

- [ ] **Step 1: Remove Add Provider dialog and related code**

Remove:
- `addProviderDialog` state and `newProvider` state (lines 17-28)
- `formErrors` state (line 29)
- `PROVIDER_TYPES` constant (lines 75-78)
- `handleNameInput`, `handleTypeChange`, `validateEndpointUrl`, `validateForm`, `handleAddProvider` functions (lines 272-340)
- `handleProviderFieldChange` function (lines 163-173)
- The "添加 Provider" button (lines 649-655)
- The entire Add Provider `<Dialog>` (lines 658-731)
- Import of `Plus` from lucide-svelte
- Import of `generateSlug` from utils

- [ ] **Step 2: Remove Endpoint display**

Remove lines 570-581 (the `{:else if provider.type !== 'sensevoice'}` block showing Endpoint EditableField).

Also remove the `{:else if}` condition, keeping only the `vertex` info block for `provider.id === 'vertex'`.

- [ ] **Step 3: Update needsApiKey to use id-based check**

Replace:
```typescript
function needsApiKey(provider: ProviderConfig): boolean {
  return provider.type === 'openai-compatible';
}
```

With:
```typescript
function needsApiKey(provider: ProviderConfig): boolean {
  return PROVIDERS_REQUIRING_KEY.includes(provider.id);
}
```

Import `PROVIDERS_REQUIRING_KEY` from config store.

- [ ] **Step 4: Update all provider.type references to provider.id**

Search and replace in this file:
- `provider.type === 'sensevoice'` → `provider.id === 'sensevoice'`
- `provider.type === 'vertex'` → `provider.id === 'vertex'`
- `provider.type !== 'sensevoice'` → `provider.id !== 'sensevoice'`

- [ ] **Step 5: Remove the delete provider button for custom (non-builtin) providers**

Since all providers are now builtin, remove the `{:else}` branch showing the delete (✕) button (lines 491-497). Every provider card only shows the reset button.

- [ ] **Step 6: Update reset dialog description**

Change "确定要重置为默认设置吗？自定义的 Endpoint、API Key 和 Models 将被覆盖。" to "确定要重置为默认设置吗？API Key 和自定义模型将被覆盖。"

- [ ] **Step 7: Commit**

```bash
git add src/routes/models/+page.svelte
git commit -m "refactor: remove Add Provider and Endpoint from models page"
```

---

### Task 12: Simplify Onboarding Provider Config Step

**Files:**
- Modify: `src/lib/components/onboarding/steps/ProviderConfigStep.svelte`

- [ ] **Step 1: Remove Add Provider dialog and related code**

Remove:
- `newProvider` state (line 23)
- `formErrors` state (line 24)
- `addProviderDialogOpen` state (line 25)
- `PROVIDER_TYPES` constant (lines 32-35)
- `handleNameInput`, `validateForm`, `handleAddProvider`, `openAddProviderDialog`, `closeAddProviderDialog` functions (lines 162-230)
- The "添加自定义 Provider" button (lines 458-464)
- The entire Add Provider `<Dialog>` (lines 480-561)
- Import of `Plus` from lucide-svelte
- Import of `generateSlug` from utils
- `handleProviderFieldChange` function (lines 154-160)

- [ ] **Step 2: Update provider.type references to provider.id**

Replace:
- `provider.type === 'vertex'` → `provider.id === 'vertex'`
- `provider.type === 'openai-compatible'` → `provider.id === 'dashscope'`
- `provider.type === 'sensevoice'` → `provider.id === 'sensevoice'`
- `provider.type !== 'sensevoice'` → `provider.id !== 'sensevoice'`
- `isSensevoice` → `provider.id === 'sensevoice'`
- `hasProviderWithApiKey` → use `needsApiKey(provider.id)` check

- [ ] **Step 3: Remove the "手动配置其他 Provider" link text**

Change the link from "手动配置其他 Provider" to "手动配置 API Key". The manual-config scene now only shows API Key input for providers that need it (dashscope), plus the Vertex info and SenseVoice status — no adding new providers.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/onboarding/steps/ProviderConfigStep.svelte
git commit -m "refactor: simplify onboarding provider config, remove Add Provider"
```

---

### Task 13: Final Integration Build and Test

**Files:**
- All modified files

- [ ] **Step 1: Run Rust compilation**

Run: `cd src-tauri && cargo build`
Expected: Clean build with no errors

- [ ] **Step 2: Run Rust tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass

- [ ] **Step 3: Run frontend build**

Run: `npm run build`
Expected: Clean build with no errors

- [ ] **Step 4: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix: resolve integration issues from provider trait migration"
```
