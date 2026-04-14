# Replace rig-vertexai with Direct google-cloud-aiplatform-v1 Usage

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the `rig-vertexai` dependency and use `google-cloud-aiplatform-v1` directly, enabling full `ThinkingConfig` support (thinkingBudget for Gemini 2.5, thinkingLevel for Gemini 3+) to fix slow response times caused by default thinking.

**Architecture:** Replace all `rig_vertexai::Client` usage with a new `VertexClient` wrapper that directly calls `google_cloud_aiplatform_v1::client::PredictionService`. The new wrapper handles client initialization (ADC auth, project/location from env), model path construction, content building, generation config (including ThinkingConfig), and response parsing. All existing call sites remain unchanged — only the internal types and Vertex path implementation change.

**Tech Stack:** Rust, `google-cloud-aiplatform-v1` v1.9.0 (already in dependency tree), `google-cloud-auth` (already transitive dep via rig-vertexai)

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `src-tauri/src/ai.rs` | **Major rewrite** | New `VertexClient` wrapper, `ThinkingMode` redesign, direct `google-cloud-aiplatform-v1` API calls |
| `src-tauri/Cargo.toml` | **Modify** | Remove `rig-vertexai`, add `google-cloud-aiplatform-v1` and `google-cloud-auth` explicitly |
| `src-tauri/src/lib.rs` | **Modify** | Update `VertexClientState` type from `rig_vertexai::Client` to new `VertexClient` |
| `src-tauri/src/real_llm_client.rs` | **Modify** | Update `VertexClientCache` type alias |
| `src-tauri/src/skills.rs` | **No change** | Already uses `crate::ai::ThinkingMode::Disabled` |
| `src-tauri/src/translation.rs` | **No change** | Already uses `crate::ai::ThinkingMode::Disabled` |

---

### Task 1: Update Cargo.toml — Remove rig-vertexai, Add Explicit Dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml:29-30`

- [ ] **Step 1: Remove rig-vertexai and add explicit deps**

Remove `rig-vertexai = "0.3"`. Add `google-cloud-aiplatform-v1` and `google-cloud-auth`:

```toml
rig-core = "0.33"
google-cloud-aiplatform-v1 = "1"
google-cloud-auth = "0.18"
```

Note: `google-cloud-aiplatform-v1` is already in the dependency tree (pulled by rig-vertexai), we just need to declare it directly. The `google-cloud-auth` crate is also already a transitive dependency.

- [ ] **Step 2: Verify Cargo.toml looks correct**

Run: `cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | head -20`

This will fail at compile time because we haven't updated the Rust code yet — that's expected. We just want to confirm the deps resolve.

---

### Task 2: Redesign ThinkingMode Enum

**Files:**
- Modify: `src-tauri/src/ai.rs:24-30`

- [ ] **Step 1: Replace ThinkingMode enum**

Replace the current enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ThinkingMode {
    Default,
    Enabled,
    Disabled,
}
```

With the new enum that carries actual parameters:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ThinkingMode {
    Default,
    Disabled,
    Budget(i32),
    Level(google_cloud_aiplatform_v1::model::generation_config::thinking_config::ThinkingLevel),
}
```

- [ ] **Step 2: Update send_text_via_openai_compatible match arms**

In `send_text_via_openai_compatible` (ai.rs:408-418), update the match:

```rust
let request = match thinking {
    ThinkingMode::Default => model.completion_request(message).build(),
    ThinkingMode::Disabled => model
        .completion_request(message)
        .additional_params(serde_json::json!({"enable_thinking": false}))
        .build(),
    ThinkingMode::Budget(_) | ThinkingMode::Level(_) => model
        .completion_request(message)
        .additional_params(serde_json::json!({"enable_thinking": true}))
        .build(),
};
```

Note: The openai-compatible path doesn't natively support thinkingBudget/thinkingLevel, but we pass enable_thinking as a best-effort signal.

---

### Task 3: Create VertexClient Wrapper and Replace Vertex Cache Type

**Files:**
- Modify: `src-tauri/src/ai.rs:32-53`

- [ ] **Step 1: Replace VertexClientCache type and get_or_create_vertex_client**

Remove the old `VertexClientCache` type alias and `get_or_create_vertex_client` function. Add the new `VertexClient` struct and its builder:

```rust
use google_cloud_aiplatform_v1 as vertexai;
use google_cloud_auth::credentials::Credentials;
use tokio::sync::OnceCell;

type VertexClientCache = Arc<Mutex<Option<VertexClient>>>;

struct VertexClient {
    project: String,
    location: String,
    credentials: Credentials,
    inner: OnceCell<vertexai::client::PredictionService>,
}

impl VertexClient {
    fn builder() -> VertexClientBuilder {
        VertexClientBuilder::default()
    }

    fn new() -> Result<Self, String> {
        Self::builder().build()
    }

    async fn get_inner(&self) -> Result<&vertexai::client::PredictionService, AiError> {
        let credentials = self.credentials.clone();
        self.inner
            .get_or_init(|| async {
                vertexai::client::PredictionService::builder()
                    .with_credentials(credentials)
                    .build()
                    .await
                    .map_err(|e| format!("Failed to build Vertex AI client: {}", e))
            })
            .await
            .as_ref()
            .map_err(|e| AiError::RequestError(e.clone()))
    }

    fn model_path(&self, model_name: &str) -> String {
        format!(
            "projects/{}/locations/{}/publishers/google/models/{}",
            self.project, self.location, model_name
        )
    }
}

#[derive(Default)]
struct VertexClientBuilder {
    project: Option<String>,
    location: Option<String>,
    credentials: Option<Credentials>,
}

impl VertexClientBuilder {
    fn build(self) -> Result<VertexClient, String> {
        let project = self
            .project
            .or_else(|| std::env::var("GOOGLE_CLOUD_PROJECT").ok())
            .ok_or_else(|| {
                "GOOGLE_CLOUD_PROJECT env var required or set via builder".to_string()
            })?;

        let location = self
            .location
            .or_else(|| std::env::var("GOOGLE_CLOUD_LOCATION").ok())
            .unwrap_or_else(|| "global".to_string());

        let credentials = if let Some(creds) = self.credentials {
            creds
        } else {
            google_cloud_auth::credentials::Builder::default()
                .build()
                .map_err(|e| format!("Failed to build credentials: {}", e))?
        };

        Ok(VertexClient {
            project,
            location,
            credentials,
            inner: OnceCell::new(),
        })
    }
}

fn get_or_create_vertex_client(
    logger: &Logger,
    cache: &VertexClientCache,
) -> Result<VertexClient, AiError> {
    let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ref client) = *guard {
        return Ok(client.clone());
    }
    let client = VertexClient::new().map_err(|e| {
        logger.error(
            "ai",
            "Vertex AI client 初始化失败",
            Some(serde_json::json!({ "error": e })),
        );
        AiError::RequestError(format!("Vertex AI client init failed: {}", e))
    })?;
    *guard = Some(client.clone());
    logger.info("ai", "Vertex AI client 已创建并缓存", None);
    Ok(client)
}
```

Note: `VertexClient` needs to derive `Clone`. Since `OnceCell` is not `Clone`, we wrap the inner client in `Arc`:

Actually, let's use `Arc<OnceCell<...>>` to make it cloneable:

```rust
struct VertexClient {
    project: String,
    location: String,
    credentials: Credentials,
    inner: Arc<OnceCell<vertexai::client::PredictionService>>,
}

impl Clone for VertexClient {
    fn clone(&self) -> Self {
        Self {
            project: self.project.clone(),
            location: self.location.clone(),
            credentials: self.credentials.clone(),
            inner: self.inner.clone(),
        }
    }
}
```

---

### Task 4: Rewrite send_via_vertex (Audio)

**Files:**
- Modify: `src-tauri/src/ai.rs:126-193`

- [ ] **Step 1: Replace send_via_vertex with direct vertexai API call**

The new implementation constructs `vertexai::model::Content` and `vertexai::model::Part` directly instead of going through rig's type system:

```rust
async fn send_via_vertex(
    logger: &Logger,
    client: &VertexClient,
    audio_b64: &str,
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
) -> Result<String, AiError> {
    logger.info(
        "ai",
        "准备发送 Vertex AI 音频请求",
        Some(serde_json::json!({
            "model": model_name,
            "media_type": media_type,
            "audio_size_b64": audio_b64.len(),
            "prompt": text_prompt,
        })),
    );

    let audio_blob = vertexai::model::Blob::new()
        .set_mime_type(media_type)
        .set_data(bytes::Bytes::from(base64::engine::general_purpose::STANDARD.decode(audio_b64).map_err(|e| AiError::RequestError(format!("Base64 decode failed: {}", e)))?));

    let audio_part = vertexai::model::Part::new().set_inline_data(audio_blob);
    let text_part = vertexai::model::Part::new().set_text(text_prompt);

    let user_content = vertexai::model::Content::new()
        .set_role("user")
        .set_parts([audio_part, text_part]);

    let generation_config = vertexai::model::GenerationConfig::new()
        .set_candidate_count(1)
        .set_thinking_config(
            vertexai::model::generation_config::ThinkingConfig::new()
                .set_thinking_budget(0),
        );

    let model_path = client.model_path(model_name);
    let prediction_client = client.get_inner().await.map_err(|e| {
        logger.error("ai", "Vertex AI client 获取失败", Some(serde_json::json!({ "error": e.to_string() })));
        e
    })?;

    logger.info(
        "ai",
        "Vertex AI 请求发送中",
        Some(serde_json::json!({ "model": model_name })),
    );

    let response = prediction_client
        .generate_content()
        .set_model(&model_path)
        .set_contents([user_content])
        .set_generation_config(generation_config)
        .send()
        .await
        .map_err(|e| {
            logger.error(
                "ai",
                "Vertex AI 请求失败",
                Some(serde_json::json!({ "error": e.to_string(), "model": model_name })),
            );
            AiError::RequestError(e.to_string())
        })?;

    let text = extract_text_from_response(&response);

    logger.info(
        "ai",
        "Vertex AI 请求成功",
        Some(serde_json::json!({ "response_length": text.len() })),
    );

    Ok(text)
}
```

---

### Task 5: Add Response Helper and Rewrite send_text_via_vertex

**Files:**
- Modify: `src-tauri/src/ai.rs` (add helper function, rewrite send_text_via_vertex at lines 307-362)

- [ ] **Step 1: Add extract_text_from_response helper**

Add this helper function before `send_via_vertex`:

```rust
fn extract_text_from_response(response: &vertexai::model::GenerateContentResponse) -> String {
    response
        .candidates
        .first()
        .and_then(|c| c.content.as_ref())
        .map(|c| {
            c.parts
                .iter()
                .filter_map(|p| p.text().cloned())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}
```

- [ ] **Step 2: Rewrite send_text_via_vertex with ThinkingConfig support**

```rust
async fn send_text_via_vertex(
    logger: &Logger,
    client: &VertexClient,
    text_prompt: &str,
    model_name: &str,
    thinking: ThinkingMode,
) -> Result<String, AiError> {
    logger.info(
        "ai",
        "准备发送 Vertex AI 文本请求",
        Some(serde_json::json!({
            "model": model_name,
            "thinking": format!("{:?}", thinking),
        })),
    );

    let text_part = vertexai::model::Part::new().set_text(text_prompt);
    let user_content = vertexai::model::Content::new()
        .set_role("user")
        .set_parts([text_part]);

    let mut generation_config = vertexai::model::GenerationConfig::new().set_candidate_count(1);

    match thinking {
        ThinkingMode::Default => {}
        ThinkingMode::Disabled => {
            generation_config = generation_config.set_thinking_config(
                vertexai::model::generation_config::ThinkingConfig::new()
                    .set_thinking_budget(0),
            );
        }
        ThinkingMode::Budget(budget) => {
            generation_config = generation_config.set_thinking_config(
                vertexai::model::generation_config::ThinkingConfig::new()
                    .set_thinking_budget(budget),
            );
        }
        ThinkingMode::Level(level) => {
            generation_config = generation_config.set_thinking_config(
                vertexai::model::generation_config::ThinkingConfig::new()
                    .set_thinking_level(level),
            );
        }
    }

    let model_path = client.model_path(model_name);
    let prediction_client = client.get_inner().await.map_err(|e| {
        logger.error("ai", "Vertex AI client 获取失败", Some(serde_json::json!({ "error": e.to_string() })));
        e
    })?;

    logger.info(
        "ai",
        "Vertex AI 文本请求发送中",
        Some(serde_json::json!({ "model": model_name })),
    );

    let response = prediction_client
        .generate_content()
        .set_model(&model_path)
        .set_contents([user_content])
        .set_generation_config(generation_config)
        .send()
        .await
        .map_err(|e| {
            logger.error(
                "ai",
                "Vertex AI 文本请求失败",
                Some(serde_json::json!({ "error": e.to_string(), "model": model_name })),
            );
            AiError::RequestError(e.to_string())
        })?;

    let text = extract_text_from_response(&response);

    logger.info(
        "ai",
        "Vertex AI 文本请求成功",
        Some(serde_json::json!({ "response_length": text.len() })),
    );

    Ok(text)
}
```

---

### Task 6: Clean Up Imports in ai.rs

**Files:**
- Modify: `src-tauri/src/ai.rs:1-14`

- [ ] **Step 1: Update imports**

Remove rig-vertexai-specific imports. Add vertexai imports. The final import block should be:

```rust
use base64::Engine;
use google_cloud_aiplatform_v1 as vertexai;
use rig::OneOrMany;
use rig::completion::message::Message;
use rig::completion::message::{AudioMediaType, UserContent};
use rig::providers::openai;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::OnceCell;

use crate::config::ProviderConfig;
use crate::logger::Logger;
```

Remove these unused imports:
- `use rig::client::CompletionClient;` (only used for rig's completion_model, which we're replacing)
- `use rig::client::transcription::TranscriptionClient;` (only used for OpenAI compatible path — check if still needed)
- `use rig::completion::CompletionModel;` (same)
- `use rig::completion::message::MimeType;` (check usage)

Actually, keep all `rig::*` imports that are used by the openai-compatible path functions. Only remove what's exclusively used by the old vertex path.

**Final imports:**

```rust
use base64::Engine;
use google_cloud_aiplatform_v1 as vertexai;
use google_cloud_auth::credentials::Credentials;
use rig::OneOrMany;
use rig::client::transcription::TranscriptionClient;
use rig::completion::CompletionModel;
use rig::completion::message::{AudioMediaType, UserContent};
use rig::providers::openai;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::OnceCell;

use crate::config::ProviderConfig;
use crate::logger::Logger;
```

Note: Remove `use rig::completion::message::Message;` — we no longer use rig's Message type in the vertex path, but we still use it in the openai-compatible text path. Check `send_text_via_openai_compatible` — yes it still uses `Message::User`. So keep it.

Actually, let's be more precise. After the rewrite:
- `send_via_vertex` and `send_text_via_vertex` → use vertexai types directly, no rig types
- `send_via_openai_compatible` → uses `openai::Client`, `TranscriptionModel` (rig trait)
- `send_text_via_openai_compatible` → uses `openai::CompletionsClient`, `CompletionModel` (rig trait), `UserContent`, `Message`, `OneOrMany`
- `send_audio_prompt`, `send_text_prompt`, `send_audio_prompt_from_bytes` → dispatch functions, no type changes

So we need:
- `rig::OneOrMany` — yes, for openai paths
- `rig::completion::message::Message` — yes, for openai text path
- `rig::completion::message::{AudioMediaType, UserContent}` — yes, for openai paths
- `rig::providers::openai` — yes
- `rig::client::transcription::TranscriptionClient` — yes, for `.transcription_model()`
- `rig::completion::CompletionModel` — yes, for `.completion_model()` / `.completion()`

We do NOT need:
- `rig::client::CompletionClient` — only needed for `client.completion_model(model)` in old vertex path

---

### Task 7: Update VertexClientState in lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs:53-55`

- [ ] **Step 1: Update VertexClientState type**

Change from:
```rust
struct VertexClientState {
    client: Arc<Mutex<Option<rig_vertexai::Client>>>,
}
```

To:
```rust
struct VertexClientState {
    client: Arc<Mutex<Option<crate::ai::VertexClient>>>,
}
```

No other changes needed in lib.rs — the usage pattern (`&h.state::<VertexClientState>().client`) remains the same.

---

### Task 8: Update real_llm_client.rs Type Alias

**Files:**
- Modify: `src-tauri/src/real_llm_client.rs:8`

- [ ] **Step 1: Update VertexClientCache type alias**

Change from:
```rust
type VertexClientCache = Arc<Mutex<Option<rig_vertexai::Client>>>;
```

To:
```rust
type VertexClientCache = Arc<Mutex<Option<crate::ai::VertexClient>>>;
```

---

### Task 9: Verify Compilation and Run Tests

**Files:**
- All modified files

- [ ] **Step 1: Run cargo check**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: Clean compilation with no errors

- [ ] **Step 2: Run cargo build**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: Successful build

- [ ] **Step 3: Run existing tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: All tests pass

---

### Task 10: Commit

- [ ] **Step 1: Stage and commit all changes**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/ai.rs src-tauri/src/lib.rs src-tauri/src/real_llm_client.rs
git commit -m "feat: replace rig-vertexai with direct google-cloud-aiplatform-v1

Remove rig-vertexai dependency (unmaintained, no public repo, drops
ThinkingConfig). Use google-cloud-aiplatform-v1 directly for full
ThinkingConfig support (thinkingBudget for Gemini 2.5, thinkingLevel
for Gemini 3+). All Vertex AI paths now explicitly disable thinking
when ThinkingMode::Disabled is set, fixing slow default-think behavior."
```
