# RIG AI 集成设计

## 目标

用 `rig-core` 替换 `async-openai`，实现录音完成后通过 RIG 发送多模态请求（音频+文本）到 AI，获取结果后写入系统剪贴板并模拟粘贴。

## 架构

```
录音完成 → Rust 读取音频文件 → RIG 构建多模态请求
→ AI 返回文本 → 写入剪贴板 → 模拟 Cmd+V → 文本粘贴到焦点位置
```

- **不回传前端**：结果只通过剪贴板粘贴，前端不参与。
- **成功无通知**：粘贴完成后不打扰用户，内容在剪贴板中保留。
- **失败发通知**：AI 请求失败时通过系统通知告知用户。

## 新增模块

### `src-tauri/src/ai.rs`

封装所有 RIG 交互逻辑：

1. 根据 `ProviderConfig` 创建对应的 RIG provider client
   - `provider_type == "vertex"` → 使用 `rig-vertexai` client
   - `provider_type == "openai-compatible"` → 使用 `rig-core` 的 OpenAI client（自定义 endpoint）
2. 提供 `send_audio_prompt(audio_path, text_prompt, provider_config) -> Result<String, AiError>`
   - 读取音频文件为 bytes
   - 构建 `UserContent::Audio` + `UserContent::Text` 的多模态消息
   - 通过 RIG completion API 发送请求
   - 返回 AI 的文本响应

### `src-tauri/src/clipboard.rs`

提供 `write_and_paste(text: &str)` 函数：

1. 使用 `arboard` crate 写入系统剪贴板
2. 通过 `osascript` 模拟 Cmd+V（macOS）

## 依赖变更

```toml
# 移除
async-openai = "0.20"

# 新增
rig-core = "0.33"        # 核心 AI 抽象层，统一多提供商接口
rig-vertexai = "0.3"     # Vertex AI (Gemini) 提供商集成
arboard = "3"            # 系统剪贴板读写
```

模拟粘贴通过 macOS 的 `osascript` 实现，无需额外依赖。

## 集成到录音流程

在 `lib.rs` 的 `stop_recording` 函数中，`recording:complete` 分支录音保存成功后：

1. 读取 `AppConfig` 中的 `features.transcription` 配置（provider_id + model）
2. 查找对应的 `ProviderConfig`
3. `tokio::spawn` 异步调用 `ai::send_audio_prompt(audio_path, prompt, provider_config)`
4. 成功 → `clipboard::write_and_paste(&response_text)`
5. 失败 → `show_notification("AI 处理失败", error_message)`

## 提供商支持（初期）

| 提供商 | RIG 实现 | 说明 |
|--------|----------|------|
| Vertex AI (Gemini) | `rig-vertexai` | 原生集成，支持多模态 |
| 阿里云 DashScope | `rig-core::providers::openai`（自定义 endpoint） | 通过 OpenAI 兼容接口 |

后续可轻松扩展：只需在配置中添加新提供商，代码中匹配对应的 RIG provider client 即可。

## 错误处理

- AI 请求失败（网络、认证、配额等）→ 系统通知 + eprintln 日志
- 剪贴板写入失败 → 系统通知
- 音频文件读取失败 → 系统通知
- 所有错误不阻塞录音流程，录音结果始终保存

## 设计原则

- **先替换后扩展**：先替换底层库为 RIG，保持现有两家提供商，设计上预留扩展接口
- **Rust 后端主导**：AI 请求、剪贴板操作、粘贴全部在 Rust 侧完成
- **最小打扰**：成功时不打扰用户，只在失败时通知
