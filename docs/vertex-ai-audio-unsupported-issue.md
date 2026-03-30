# Vertex AI 音频请求不支持问题

## 状态：待修复

## 问题描述

使用 Vertex AI provider 发送音频转录请求时，返回错误：

```
ProviderError: Unsupported user content type: Audio(Audio { data: Base64(...) })
```

Vertex AI（Gemini 模型）本身是支持多模态输入（包括音频）的，问题出在 `rig-vertexai` crate 的实现上。

## 根因分析

### 调用链路

```
用户录音完成
  → ai.rs::send_audio_prompt()
    → ai.rs::send_via_vertex()
      → UserContent::audio(base64_data, media_type)    // rig-core 构建 Audio 类型
      → model.completion_request(message).build()
      → model.completion(request)                       // rig-vertexai 处理
        → types/message.rs: RigMessage → vertexai::Content 转换
          → UserContent::Audio 走到 _ => Err(...) 分支  ← 报错位置
```

### 关键代码位置

1. **调用入口** — `src-tauri/src/ai.rs:82-142` (`send_via_vertex` 函数)
   - 第 96 行：`AudioMediaType::from_mime_type(media_type)` 转换 MIME 类型
   - 第 106 行：`UserContent::audio(audio_b64.to_string(), audio_mt)` 构建音频内容
   - 第 120-127 行：通过 `completion_request` → `completion()` 发送请求

2. **报错位置** — `rig-vertexai-0.3.2/src/types/message.rs:56-59`
   ```rust
   _ => Err(CompletionError::ProviderError(format!(
       "Unsupported user content type: {:?}",
       user_content
   ))),
   ```
   `UserContent::Audio` 没有被显式处理，直接走到了 `_` 兜底分支返回错误。

3. **底层 API 实际支持** — `google-cloud-aiplatform-v1-1.9.0/src/model.rs`
   - `Part` 结构体有 `Data::InlineData(Box<Blob>)` 变体（第 4633 行）
   - `Blob` 结构体有 `mime_type: String` 和 `data: Bytes` 字段（第 4686-4693 行）
   - `Part::set_inline_data()` 方法存在（第 3934 行）
   - 只是 `rig-vertexai` 没有实现 `UserContent::Audio → Part::inline_data(Blob)` 的转换

### rig-core 的 Audio 数据结构

`rig-core-0.33.0/src/completion/message.rs`:

```rust
pub struct Audio {
    pub data: DocumentSourceKind,      // Base64(String) | Raw(Vec<u8>) | Url(String) | ...
    pub media_type: Option<AudioMediaType>,  // WAV | MP3 | FLAC | OGG | ...
    pub additional_params: Option<serde_json::Value>,
}
```

理论上，`Audio { data: Base64(b64_str), media_type: Some(WAV) }` 可以被转换为：
```rust
Part::new().set_inline_data(
    Blob::new()
        .set_mime_type("audio/wav")
        .set_data(Bytes::from(decode(b64_str)))
)
```

## 影响范围

- **受影响功能**：Vertex AI provider 的音频转录（核心功能）
- **不受影响功能**：
  - Vertex AI 的纯文本请求（`send_text_via_vertex` 正常工作）
  - OpenAI-Compatible provider 的音频转录（使用不同的 API 路径）
- **当前规避措施**：连通性测试已改为文本测试（`lib.rs:328`）

## 修复方案

### 方案 A：绕过 rig-vertexai，直接调用 REST API（推荐）

用 `reqwest` 直接调用 Vertex AI REST API：

```
POST https://{location}-aiplatform.googleapis.com/v1/projects/{project}/locations/{location}/publishers/google/models/{model}:generateContent
```

请求体：
```json
{
  "contents": [{
    "role": "user",
    "parts": [
      { "inline_data": { "mime_type": "audio/wav", "data": "<base64>" } },
      { "text": "请将这段音频转录为文字" }
    ]
  }]
}
```

认证方式：使用 ADC（Application Default Credentials）获取 Bearer token。

**优点**：不依赖 `rig-vertexai` 的消息转换，完全可控，`inlineData` 是 Gemini 原生支持的字段。
**缺点**：需要自行处理 HTTP 请求和认证，增加约 50-80 行代码。

### 方案 B：Patch rig-vertexai crate

使用 Cargo `[patch]` 指向本地修改版或 fork，在 `types/message.rs` 中添加：

```rust
UserContent::Audio(audio) => {
    let mime = audio.media_type
        .map(|mt| mt.to_mime_type().to_string())
        .unwrap_or("audio/wav".to_string());
    let bytes = match audio.data {
        DocumentSourceKind::Base64(b64) => {
            base64::engine::general_purpose::STANDARD.decode(&b64)?
        }
        DocumentSourceKind::Raw(bytes) => bytes,
        _ => return Err(CompletionError::ProviderError("...".into())),
    };
    Ok(vertexai::model::Part::new().set_inline_data(
        vertexai::model::Blob::new()
            .set_mime_type(mime)
            .set_data(bytes::Bytes::from(bytes))
    ))
}
```

**优点**：修改最小，符合 rig 框架设计。
**缺点**：需要维护 fork，上游更新可能冲突。

### 方案 C：换用 Google Generative AI SDK

不使用 Vertex AI（企业级 gRPC 端点），改用 Google AI 的 REST API：

```
POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}
```

**优点**：更简单，用 API Key 认证而非 ADC。
**缺点**：功能受限（无企业级特性），需要 API Key 而非 ADC 认证。

## 涉及文件

| 文件 | 说明 |
|------|------|
| `src-tauri/src/ai.rs:82-142` | `send_via_vertex()` — 需要修改的调用入口 |
| `src-tauri/Cargo.toml` | 可能需要添加 `reqwest` 依赖（方案 A）或 `[patch]`（方案 B） |
| `src-tauri/src/lib.rs:328` | 连通性测试规避逻辑，修复后可恢复音频测试 |
| `rig-vertexai-0.3.2/src/types/message.rs:56` | 根因位置（外部 crate） |

## 相关依赖版本

| Crate | 版本 |
|-------|------|
| `rig-vertexai` | 0.3.2 |
| `rig-core` | 0.33.0 |
| `google-cloud-aiplatform-v1` | 1.9.0 |
