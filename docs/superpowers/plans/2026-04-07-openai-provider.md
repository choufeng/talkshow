# OpenAI Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an OpenAI Provider to TalkShow that supports audio transcription and text completion via the OpenAI API, with a configurable base URL for third-party compatible services.

**Architecture:** New `providers/openai.rs` implementing the existing `Provider` trait, with `ProviderConfig` extended by an `endpoint` field. The frontend renders an endpoint editor for providers that need one.

**Tech Stack:** Rust (reqwest, async-trait, serde), TypeScript/Svelte 5 (stores, EditableField component)

---

### Task 1: Add `endpoint` field to `ProviderConfig` (Rust)

**Files:**
- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: Add `endpoint` field to `ProviderConfig` struct**

In `src-tauri/src/config.rs`, add `endpoint` to `ProviderConfig`:

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub models: Vec<ModelConfig>,
}
```

- [ ] **Step 2: Add OpenAI entry to `builtin_providers()`**

Add the OpenAI provider to the `builtin_providers()` function, before the `dashscope` entry:

```rust
ProviderConfig {
    id: "openai".to_string(),
    name: "OpenAI".to_string(),
    api_key: Some(String::new()),
    endpoint: Some("https://api.openai.com/v1".to_string()),
    models: vec![ModelConfig {
        name: "gpt-4o-transcribe".to_string(),
        capabilities: vec!["transcription".to_string(), "chat".to_string()],
        verified: None,
    }],
},
```

- [ ] **Step 3: Run Rust type check to verify**

Run: `cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: Compiles with errors about `providers/mod.rs` not handling the `"openai"` case yet (that's fine, will be resolved in Task 2). If there are errors about the `endpoint` field itself, fix them.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "feat: add endpoint field to ProviderConfig and OpenAI builtin provider"
```

---

### Task 2: Implement `providers/openai.rs`

**Files:**
- Create: `src-tauri/src/providers/openai.rs`
- Modify: `src-tauri/src/providers/mod.rs`

- [ ] **Step 1: Create `openai.rs` with the full provider implementation**

Create `src-tauri/src/providers/openai.rs`:

```rust
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

        let resp: TranscriptionResponse = response.json().await.map_err(|e| {
            ProviderError::RequestError(format!("Failed to parse response: {}", e))
        })?;

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
            name: "gpt-4o-transcribe".to_string(),
            capabilities: vec!["transcription".to_string(), "chat".to_string()],
            verified: None,
        }]
    }
}
```

- [ ] **Step 2: Register `openai` module in `providers/mod.rs`**

Add at the top of `src-tauri/src/providers/mod.rs`:

```rust
pub mod openai;
```

Add the `"openai"` branch to the `create_provider` match:

```rust
"openai" => Ok(Box::new(openai::OpenAIProvider::new(
    config.api_key.clone(),
    config.endpoint.clone(),
))),
```

- [ ] **Step 3: Run `cargo check` to verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/providers/openai.rs src-tauri/src/providers/mod.rs
git commit -m "feat: implement OpenAI provider with transcription and text completion"
```

---

### Task 3: Update frontend TypeScript types and constants

**Files:**
- Modify: `src/lib/stores/config.ts`

- [ ] **Step 1: Add `endpoint` to `ProviderConfig` interface**

In `src/lib/stores/config.ts`, add `endpoint` to the `ProviderConfig` interface:

```typescript
export interface ProviderConfig {
  id: string;
  name: string;
  api_key?: string;
  endpoint?: string;
  models: ModelConfig[];
}
```

- [ ] **Step 2: Add OpenAI to `BUILTIN_PROVIDERS`**

Add this entry to the `BUILTIN_PROVIDERS` array (before the `vertex` entry):

```typescript
{
    id: 'openai',
    name: 'OpenAI',
    api_key: '',
    endpoint: 'https://api.openai.com/v1',
    models: [{ name: 'gpt-4o-transcribe', capabilities: ['transcription', 'chat'] }]
},
```

- [ ] **Step 3: Update `PROVIDERS_REQUIRING_KEY`**

Change the constant to include `'openai'`:

```typescript
export const PROVIDERS_REQUIRING_KEY = ['dashscope', 'openai'];
```

- [ ] **Step 4: Add `PROVIDERS_WITH_ENDPOINT` constant**

Add after the `PROVIDERS_REQUIRING_KEY` line:

```typescript
export const PROVIDERS_WITH_ENDPOINT = ['openai'];
```

- [ ] **Step 5: Run TypeScript type check**

Run: `npm run check 2>&1 | tail -10`
Expected: No errors related to the config types

- [ ] **Step 6: Commit**

```bash
git add src/lib/stores/config.ts
git commit -m "feat: add OpenAI provider to frontend config types and constants"
```

---

### Task 4: Add endpoint editor to models page

**Files:**
- Modify: `src/routes/models/+page.svelte`

- [ ] **Step 1: Import `PROVIDERS_WITH_ENDPOINT` from config store**

In the `<script>` tag import line (line 3), add `PROVIDERS_WITH_ENDPOINT` to the import:

Change:
```
import { config, isBuiltinProvider, BUILTIN_PROVIDERS, MODEL_CAPABILITIES, TRANSLATE_LANGUAGES, PROVIDERS_REQUIRING_KEY } from '$lib/stores/config';
```
To:
```
import { config, isBuiltinProvider, BUILTIN_PROVIDERS, MODEL_CAPABILITIES, TRANSLATE_LANGUAGES, PROVIDERS_REQUIRING_KEY, PROVIDERS_WITH_ENDPOINT } from '$lib/stores/config';
```

- [ ] **Step 2: Add `handleEndpointChange` function**

Add this function after the `handleApiKeyChange` function (around line 157):

```typescript
async function handleEndpointChange(providerId: string, value: string) {
    try {
        await config.save(updateNestedPath($config, ['ai', 'providers'], (providers) =>
            (providers as ProviderConfig[]).map((p) =>
                p.id === providerId ? { ...p, endpoint: value } : p
            )
        ));
    } catch (e) {
        console.error('Failed to save endpoint:', e);
        await config.load();
    }
}
```

- [ ] **Step 3: Add endpoint editor block in the provider card template**

In the provider card template (inside `{#each $config.ai.providers || [] as provider (provider.id)}`), after the `{#if needsApiKey(provider)}` block that renders the API Key input (after the closing `{/if}` around line 452), add:

```svelte
{#if PROVIDERS_WITH_ENDPOINT.includes(provider.id)}
    <div class="mb-3">
        <span class="block text-body text-foreground-alt mb-1">Endpoint</span>
        <EditableField
            value={provider.endpoint || ''}
            aria-label="Endpoint"
            placeholder="https://api.openai.com/v1"
            mode="text"
            onChange={(val: string) => handleEndpointChange(provider.id, val)}
        />
    </div>
{/if}
```

- [ ] **Step 4: Run TypeScript type check**

Run: `npm run check 2>&1 | tail -10`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add src/routes/models/+page.svelte
git commit -m "feat: add endpoint editor for OpenAI provider on models page"
```

---

### Task 5: Build and verify

**Files:** None (verification only)

- [ ] **Step 1: Run full Rust build**

Run: `cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: Build succeeds

- [ ] **Step 2: Run frontend type check**

Run: `npm run check`
Expected: No errors

- [ ] **Step 3: Run existing tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20`
Expected: All existing tests pass (the OpenAI provider does not break existing functionality because `#[serde(default)]` handles the new `endpoint` field gracefully for existing configs)

- [ ] **Step 4: Final commit (if any fixups needed)**

If any fixups were needed during verification, commit them. Otherwise, no commit needed.
