# 模型连通性测试 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在模型配置页面上为每个模型提供连通性和可用性测试，结果持久化到配置文件中。

**Architecture:** 新增 Tauri 命令 `test_model_connectivity`，根据模型 capabilities 决定发送文本还是音频测试请求，统一以 API 响应判断结果。前端在模型标签上显示验证状态（idle/testing/ok/error），测试完成后结果通过 config.json 持久化。

**Tech Stack:** Rust (rig-core, rig-vertexai, tokio), Svelte 5, Tauri 2 IPC, Tailwind CSS

---

### Task 1: 后端 — 新增 ModelVerified 结构体和 config 类型更新

**Files:**
- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: 在 config.rs 中新增 ModelVerified 结构体并更新 ModelConfig**

在 `ModelConfig` 结构体之前添加 `ModelVerified`，然后给 `ModelConfig` 加上 `verified` 字段：

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ModelVerified {
    pub status: String,
    pub tested_at: String,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ModelConfig {
    pub name: String,
    pub capabilities: Vec<String>,
    pub verified: Option<ModelVerified>,
}
```

- [ ] **Step 2: 验证编译通过**

Run: `cargo check` (in `src-tauri/`)
Expected: 编译成功

---

### Task 2: 后端 — 新增 ai.rs 文本和音频字节测试函数

**Files:**
- Modify: `src-tauri/src/ai.rs`

- [ ] **Step 1: 新增 send_text_prompt 函数**

在 `send_audio_prompt` 函数之后添加纯文本测试函数，复用现有的 provider 路由逻辑：

```rust
pub async fn send_text_prompt(
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
) -> Result<String, AiError> {
    match provider.provider_type.as_str() {
        "vertex" => send_text_via_vertex(text_prompt, model_name).await,
        "openai-compatible" => {
            let api_key = provider
                .api_key
                .as_deref()
                .ok_or_else(|| AiError::MissingApiKey(provider.id.clone()))?;
            send_text_via_openai_compatible(text_prompt, model_name, api_key, &provider.endpoint)
                .await
        }
        _ => Err(AiError::ProviderNotFound(format!(
            "Unknown provider type: {}",
            provider.provider_type
        ))),
    }
}

async fn send_text_via_vertex(
    text_prompt: &str,
    model_name: &str,
) -> Result<String, AiError> {
    let client = rig_vertexai::Client::builder()
        .build()
        .map_err(|e| AiError::RequestError(format!("Vertex AI client init failed: {}", e)))?;

    let model = client.completion_model(model_name);
    let prompt_content = OneOrMany::one(UserContent::text(text_prompt.to_string()));
    let message = Message::User { content: prompt_content };
    let request = model.completion_request(message).build();
    let response = model
        .completion(request)
        .await
        .map_err(|e| AiError::RequestError(e.to_string()))?;

    let text = response
        .choice
        .into_iter()
        .filter_map(|c| match c {
            rig::completion::message::AssistantContent::Text(t) => Some(t.text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    Ok(text)
}

async fn send_text_via_openai_compatible(
    text_prompt: &str,
    model_name: &str,
    api_key: &str,
    base_url: &str,
) -> Result<String, AiError> {
    let client = openai::CompletionsClient::builder()
        .api_key(api_key)
        .base_url(base_url)
        .build()
        .map_err(|e| AiError::RequestError(format!("Client init failed: {}", e)))?;

    let model = client.completion_model(model_name);
    let prompt_content = OneOrMany::one(UserContent::text(text_prompt.to_string()));
    let message = Message::User { content: prompt_content };
    let request = model.completion_request(message).build();
    let response = model
        .completion(request)
        .await
        .map_err(|e| AiError::RequestError(e.to_string()))?;

    let text = response
        .choice
        .into_iter()
        .filter_map(|c| match c {
            rig::completion::message::AssistantContent::Text(t) => Some(t.text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    Ok(text)
}
```

- [ ] **Step 2: 新增 send_audio_prompt_from_bytes 函数**

```rust
pub async fn send_audio_prompt_from_bytes(
    audio_bytes: &[u8],
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
) -> Result<String, AiError> {
    let audio_b64 = base64::engine::general_purpose::STANDARD.encode(audio_bytes);

    match provider.provider_type.as_str() {
        "vertex" => send_via_vertex(&audio_b64, media_type, text_prompt, model_name).await,
        "openai-compatible" => {
            let api_key = provider
                .api_key
                .as_deref()
                .ok_or_else(|| AiError::MissingApiKey(provider.id.clone()))?;
            send_via_openai_compatible(
                &audio_b64,
                media_type,
                text_prompt,
                model_name,
                api_key,
                &provider.endpoint,
            )
            .await
        }
        _ => Err(AiError::ProviderNotFound(format!(
            "Unknown provider type: {}",
            provider.provider_type
        ))),
    }
}
```

- [ ] **Step 3: 验证编译通过**

Run: `cargo check` (in `src-tauri/`)
Expected: 编译成功

---

### Task 3: 后端 — 生成测试音频文件

**Files:**
- Create: `src-tauri/assets/test.wav`

- [ ] **Step 1: 用 macOS say 命令生成测试音频**

```bash
say -o src-tauri/assets/test.wav "测试"
```

如果 `assets/` 目录不存在先创建：
```bash
mkdir -p src-tauri/assets
say -o src-tauri/assets/test.wav "测试"
```

验证文件存在且大小合理（应该几 KB 到几十 KB）：
```bash
ls -la src-tauri/assets/test.wav
```

---

### Task 4: 后端 — 新增 test_model_connectivity Tauri 命令

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 新增 TestResult 结构体和 test_model_connectivity 命令**

在 `lib.rs` 中 `save_config_cmd` 函数之后添加：

```rust
use std::time::Instant;

#[derive(serde::Serialize, Clone)]
struct TestResult {
    status: String,
    latency_ms: Option<u64>,
    message: String,
}

#[tauri::command]
async fn test_model_connectivity(
    app_handle: tauri::AppHandle,
    provider_id: String,
    model_name: String,
) -> Result<TestResult, String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);

    let provider = app_config
        .ai
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?
        .clone();

    let model = provider
        .models
        .iter()
        .find(|m| m.name == model_name)
        .ok_or_else(|| format!("Model not found: {}", model_name))?
        .clone();

    let is_transcription = model.capabilities.iter().any(|c| c == "transcription");

    let test_audio: &[u8] = include_bytes!("../assets/test.wav");

    let start = Instant::now();
    let result = if is_transcription {
        ai::send_audio_prompt_from_bytes(
            test_audio,
            "audio/wav",
            "请将这段音频转录为文字",
            &model_name,
            &provider,
        )
        .await
    } else {
        ai::send_text_prompt("Hi", &model_name, &provider).await
    };
    let latency = start.elapsed().as_millis() as u64;

    let (status, message) = match result {
        Ok(text) => ("ok".to_string(), text.chars().take(50).collect()),
        Err(e) => ("error".to_string(), e.to_string()),
    };

    let verified = config::ModelVerified {
        status: status.clone(),
        tested_at: chrono::Utc::now().to_rfc3339(),
        latency_ms: Some(latency),
        message: if status == "error" {
            Some(message.clone())
        } else {
            None
        },
    };

    if let Some(p) = app_config.ai.providers.iter_mut().find(|p| p.id == provider_id) {
        if let Some(m) = p.models.iter_mut().find(|m| m.name == model_name) {
            m.verified = Some(verified);
        }
    }
    config::save_config(&app_data_dir, &app_config)?;

    Ok(TestResult {
        status,
        latency_ms: Some(latency),
        message,
    })
}
```

注意：需要添加 `chrono` 依赖到 `Cargo.toml`，用于生成 ISO 8601 时间戳。

- [ ] **Step 2: 在 Cargo.toml 添加 chrono 依赖**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 中添加：

```toml
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 3: 注册新命令到 invoke_handler**

在 `lib.rs` 的 `invoke_handler` 宏中添加 `test_model_connectivity`：

```rust
.invoke_handler(tauri::generate_handler![
    get_config,
    update_shortcut,
    save_config_cmd,
    test_model_connectivity
])
```

- [ ] **Step 4: 验证编译通过**

Run: `cargo check` (in `src-tauri/`)
Expected: 编译成功

---

### Task 5: 前端 — 更新 config.ts 类型定义

**Files:**
- Modify: `src/lib/stores/config.ts`

- [ ] **Step 1: 新增 ModelVerified 接口并更新 ModelConfig**

```typescript
export interface ModelVerified {
  status: 'ok' | 'error';
  tested_at: string;
  latency_ms?: number;
  message?: string;
}

export interface ModelConfig {
  name: string;
  capabilities: string[];
  verified?: ModelVerified;
}
```

- [ ] **Step 2: 验证 TypeScript 无错误**

Run: `npx svelte-check --tsconfig ./tsconfig.json` (in project root)
Expected: 无类型错误

---

### Task 6: 前端 — 更新模型配置页面 UI

**Files:**
- Modify: `src/routes/models/+page.svelte`

- [ ] **Step 1: 添加 import 和测试状态**

在 script 顶部添加 `invoke` 导入：

```typescript
import { invoke } from '@tauri-apps/api/core';
```

在现有状态变量后添加测试相关状态：

```typescript
import type { ModelVerified } from '$lib/stores/config';

let testingModels = $state<Set<string>>(new Set());
```

- [ ] **Step 2: 添加测试函数**

在 `handleDialogOpenChange` 函数之后添加：

```typescript
function formatTestDate(isoStr: string): string {
  try {
    return new Date(isoStr).toLocaleDateString(undefined, { month: '2-digit', day: '2-digit' });
  } catch {
    return '';
  }
}

function getTestKey(providerId: string, modelName: string): string {
  return `${providerId}::${modelName}`;
}

function isTesting(providerId: string, modelName: string): boolean {
  return testingModels.has(getTestKey(providerId, modelName));
}

async function testModel(providerId: string, modelName: string) {
  const key = getTestKey(providerId, modelName);
  testingModels = new Set([...testingModels, key]);
  try {
    const result = await invoke<{ status: string; latency_ms?: number; message: string }>('test_model_connectivity', {
      providerId,
      modelName,
    });
    await config.load();
    return result;
  } catch (e) {
    await config.load();
    throw e;
  } finally {
    const next = new Set(testingModels);
    next.delete(key);
    testingModels = next;
  }
}

async function testAllModels(provider: ProviderConfig) {
  for (const model of provider.models) {
    await testModel(provider.id, model.name);
  }
}
```

- [ ] **Step 3: 更新模型标签模板**

将现有的模型标签区域（`{#each provider.models || [] as model (model.name)}` 内的 `<span>`）替换为带验证状态的版本：

```svelte
{#each provider.models || [] as model (model.name)}
  {@const verified = model.verified}
  {@const testing = isTesting(provider.id, model.name)}
  <span
    class="inline-flex items-center gap-1 rounded px-2 py-0.5 text-[10px] text-accent-foreground cursor-pointer select-none
      {verified?.status === 'ok' ? 'bg-green-500/20 border border-green-500/30' : ''}
      {verified?.status === 'error' ? 'bg-red-500/20 border border-red-500/30' : ''}
      {!verified && !testing ? 'bg-accent' : ''}
      {testing ? 'bg-accent' : ''}"
    title={verified ? `${verified.status === 'ok' ? '验证通过' : '验证失败'}${verified.latency_ms ? ' · ' + verified.latency_ms + 'ms' : ''}${verified.message ? ' · ' + verified.message : ''}` : '点击测试'}
    onclick={() => testModel(provider.id, model.name)}
  >
    {model.name}
    {#if model.capabilities?.includes('transcription')}
      <span class="text-[8px] opacity-70">T</span>
    {/if}
    {#if testing}
      <span class="animate-spin text-[9px]">⟳</span>
    {:else if verified?.status === 'ok'}
      <span class="text-green-500 text-[9px]">✓</span>
      <span class="text-[8px] text-green-500/70">{formatTestDate(verified.tested_at)}</span>
    {:else if verified?.status === 'error'}
      <span class="text-red-500 text-[9px]">✕</span>
      <span class="text-[8px] text-red-500/70">{formatTestDate(verified.tested_at)}</span>
    {/if}
    <button
      class="opacity-60 hover:opacity-100 transition-opacity"
      onclick|stopPropagation={() => handleRemoveModel(provider.id, model.name)}
    >
      ✕
    </button>
  </span>
{/each}
```

注意：Svelte 5 中使用 `onclick|stopPropagation` 语法来阻止删除按钮的点击冒泡到标签的测试点击。如果不支持此语法，则用 `onclick={(e) => { e.stopPropagation(); handleRemoveModel(provider.id, model.name); }}`。

- [ ] **Step 4: 添加"测试全部模型"按钮**

在"添加模型"按钮之后添加测试全部按钮：

```svelte
<button
  class="text-xs text-accent-foreground hover:underline"
  onclick={() => openAddModelDialog(provider.id)}
>
  + 添加模型
</button>
<span class="text-border mx-1">|</span>
<button
  class="text-xs text-accent-foreground hover:underline inline-flex items-center gap-0.5"
  onclick={() => testAllModels(provider)}
  disabled={provider.models.length === 0 || [...testingModels].some(k => k.startsWith(provider.id + '::'))}
>
  ⟳ 测试全部
</button>
```

- [ ] **Step 5: 验证前端编译**

Run: `npx svelte-check --tsconfig ./tsconfig.json` (in project root)
Expected: 无错误

---

### Task 7: 端到端验证

- [ ] **Step 1: 启动开发环境**

Run: `cargo tauri dev` (in project root)

- [ ] **Step 2: 验证页面正常加载**

- 打开应用，导航到模型配置页面
- 确认页面无 JS 错误（打开 DevTools 检查 Console）
- 确认现有 Provider 和 Model 正常显示

- [ ] **Step 3: 验证测试功能**

- 点击单个模型标签，确认出现旋转图标
- 确认测试完成后显示 ✓ 或 ✕ 状态
- 确认 tooltip 显示详细信息（延迟、错误消息）
- 点击"测试全部"按钮，确认顺序执行

- [ ] **Step 4: 验证持久化**

- 测试完成后关闭应用
- 重新打开应用，导航到模型配置页
- 确认上次测试结果仍然显示（✓/✕ + 日期）
- 检查 config.json 文件确认 `verified` 字段已写入

- [ ] **Step 5: 提交所有变更**

```bash
git add -A
git commit -m "feat: add model connectivity test with persistent verification status"
```
