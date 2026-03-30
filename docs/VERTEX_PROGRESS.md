# Vertex AI 修复进度

## 背景

项目 TalkShow 是一个 Tauri v2 桌面应用（前端 SvelteKit 5 + 后端 Rust），通过录音实现语音转文字。

当前后端 AI 请求模块 `src-tauri/src/ai.rs` 支持 3 个入口函数，按 `provider_type` 字符串 match 分派到 Vertex AI 或 OpenAI-Compatible 的具体实现。

使用的 AI SDK：
- `rig-vertexai` 0.3.2 — Vertex AI 客户端（通过 gRPC 调用，**不使用 HTTP endpoint**）
- `rig-core` 0.33 — rig 框架
- `openai` crate（rig 的 openai provider）— OpenAI 兼容接口

## 发现的问题

### 问题 1：Vertex 的 endpoint 配置无实际作用
- `rig-vertexai` 的 `Client` 构建不接收 endpoint URL，它通过 gRPC 的 `PredictionService` 自动连接
- 实际需要的环境变量：`GOOGLE_CLOUD_PROJECT`（必需）、`GOOGLE_CLOUD_LOCATION`（可选，默认 `"global"`）
- 认证依赖 ADC（`gcloud auth application-default login`）
- config.json 中 Vertex 的 endpoint 字段完全无用

### 问题 2：rig-vertexai 不支持 Audio 内容类型
- `rig-vertexai` 源码 `types/message.rs:56` 对 `UserContent::Audio` 直接返回 `Unsupported user content type: Audio` 错误
- 这意味着 Vertex AI 的连通性测试不能用音频，只能用文本

### 问题 3：内置 provider 的 type 可被用户随意修改
- 前端 `PROVIDER_TYPES` 包含 `stubbed`（未实现），用户可把 Vertex 改成 stubbed
- 已在上一轮提交中修复：后端/前端都会强制校正内置 provider 的 type 和 endpoint

## 已完成的提交

### `06ffaf9` — fix: force-correct builtin provider type/endpoint on config load, remove stubbed option
- `config.rs`: `merge_builtin_providers` 强制校正内置 provider 的 `provider_type` 和 `endpoint`
- `config.ts`: `mergeBuiltinProviders` 同步校正逻辑
- `+page.svelte`: `PROVIDER_TYPES` 移除 `stubbed` 选项

## 当前未提交的修改（工作区中）

### 1. `src-tauri/src/config.rs`
- Vertex builtin 的 endpoint 从 `"https://aiplatform.googleapis.com/v1"` 改为 `String::new()`（空字符串）
- dashscope 保持不变

### 2. `src-tauri/src/lib.rs` — test_model_connectivity
- Vertex 类型的连通性测试改为使用文本测试（`send_text_prompt("Hi")`）而非音频测试
- 原逻辑：`if is_transcription { send_audio } else { send_text }`
- 新逻辑：`if vertex { send_text } else if is_transcription { send_audio } else { send_text }`
- Rust 编译已通过（cargo check 0 errors, 1 warning: unused `warn` method）

## 待完成的任务

### Task 3: 添加 `get_vertex_env_info` Tauri 命令
**文件**: `src-tauri/src/lib.rs`

需要在 `test_model_connectivity` 函数之后、`show_notification` 之前添加：

```rust
#[derive(serde::Serialize, Clone)]
struct VertexEnvInfo {
    project: String,
    location: String,
}

#[tauri::command]
fn get_vertex_env_info() -> VertexEnvInfo {
    let project = std::env::var("GOOGLE_CLOUD_PROJECT").unwrap_or_default();
    let location = std::env::var("GOOGLE_CLOUD_LOCATION")
        .unwrap_or_else(|_| "global".to_string());
    VertexEnvInfo { project, location }
}
```

然后在 `invoke_handler` 中注册：
```rust
.invoke_handler(tauri::generate_handler![
    get_config,
    update_shortcut,
    save_config_cmd,
    test_model_connectivity,
    get_vertex_env_info,  // 新增
    logger::get_log_sessions,
    logger::get_log_content
])
```

### Task 4: 前端 — Vertex 卡片隐藏 endpoint，显示环境变量信息
**文件**: `src/routes/models/+page.svelte`

需要做的修改：

1. 添加 `vertexEnvInfo` 状态和加载逻辑：
```typescript
let vertexEnvInfo = $state<{ project: string; location: string } | null>(null);

onMount(async () => {
  config.load();
  try {
    vertexEnvInfo = await invoke<{ project: string; location: string }>('get_vertex_env_info');
  } catch { vertexEnvInfo = null; }
});
```

2. 在 provider 卡片中，当 `provider.type === 'vertex'` 时：
   - **隐藏** endpoint 输入框
   - **显示**环境变量信息（只读）：
     - `GOOGLE_CLOUD_PROJECT`: 显示值或 "未设置"
     - `GOOGLE_CLOUD_LOCATION`: 显示值或 "global"（默认）
     - `ADC 认证`: 提示运行 `gcloud auth application-default login`

3. 非 Vertex 的 provider 保持显示 endpoint 输入框

### Task 5: 前端 config.ts 同步更新
**文件**: `src/lib/stores/config.ts`

- `BUILTIN_PROVIDERS` 中 Vertex 的 endpoint 改为空字符串 `''`

### Task 6: 最终验证
- `cargo check` 通过
- `npm run check` 通过（0 errors）
- 手动测试 Vertex 连通性（预期：文本测试通过）

## 关键文件清单

| 文件 | 职责 |
|------|------|
| `src-tauri/src/ai.rs` | AI API 调用：Vertex/OpenAI-Compatible 的音频和文本请求（本次未改） |
| `src-tauri/src/config.rs` | 配置模型定义、持久化、内置 provider 合并（已改 endpoint） |
| `src-tauri/src/lib.rs` | Tauri 命令注册、快捷键、录音调度（已改测试逻辑，待加命令） |
| `src/lib/stores/config.ts` | 前端配置 TypeScript 类型、Tauri invoke 封装（待同步） |
| `src/routes/models/+page.svelte` | 模型管理页面 UI（待改 Vertex 显示） |

## rig-vertexai 关键源码位置（已查阅）

- `~/.cargo/registry/src/.../rig-vertexai-0.3.2/src/client.rs` — Client 构建，读 `GOOGLE_CLOUD_PROJECT` 和 `GOOGLE_CLOUD_LOCATION`，默认 location 为 `"global"`
- `~/.cargo/registry/src/.../rig-vertexai-0.3.2/src/types/message.rs:56` — `UserContent::Audio` 走 `_ => Err(...)` 分支，不支持音频
- `~/.cargo/registry/src/.../rig-vertexai-0.3.2/src/completion.rs` — completion API，通过 gRPC `PredictionService.generate_content`
