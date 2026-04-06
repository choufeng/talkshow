# 子任务 A: ProviderConfig 简化

> **依赖**: 无 | **阶段**: Phase 1 (第一个执行) | **复杂度**: 中

## 目标

从 `ProviderConfig` 中移除 `provider_type` 和 `endpoint` 字段。Provider 的身份和端点由 `id` 字段硬编码决定，不再允许用户自定义。

## 涉及文件

| 文件 | 操作 |
|------|------|
| `src-tauri/src/config.rs` | 修改 |

## 前置知识

当前 `ProviderConfig` 结构:
```rust
pub struct ProviderConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub provider_type: String,   // "vertex" | "openai-compatible" | "sensevoice"
    pub name: String,
    pub endpoint: String,        // 如 "https://dashscope.aliyuncs.com/..."
    pub api_key: Option<String>,
    pub models: Vec<ModelConfig>,
}
```

迁移后:
```rust
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_key: Option<String>,
    pub models: Vec<ModelConfig>,
}
```

## 步骤

- [ ] **Step 1: 修改 ProviderConfig 结构体**

在 `config.rs` 中，删除 `provider_type` 和 `endpoint` 字段。移除 `#[serde(rename = "type")]`。

- [ ] **Step 2: 更新 `builtin_providers()` 函数**

移除所有 `provider_type` 和 `endpoint` 字段：

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

- [ ] **Step 3: 简化 `merge_builtin_providers()`**

移除 `provider_type` 和 `endpoint` 的修正逻辑，只保留 `name` 修正和缺失 provider 补充。

- [ ] **Step 4: 简化 `validate_config()`**

移除 `provider_type` 验证（只允许三种类型的检查）和 `endpoint` URL 格式验证。只保留：
- Provider ID 不能为空
- Provider name 不能为空
- Shortcut 长度限制

- [ ] **Step 5: 更新所有测试**

1. 所有测试中创建 `ProviderConfig` 的地方，移除 `provider_type` 和 `endpoint` 字段
2. **删除** `test_validate_config_rejects_invalid_provider_type` 测试
3. **删除** `test_validate_config_rejects_non_https_endpoint` 测试
4. **更新** `test_validate_config_allows_empty_endpoint_for_vertex` → 移除 endpoint 相关断言

- [ ] **Step 6: 处理配置迁移 (serde 兼容)**

为已保存的用户配置文件添加向后兼容：旧配置中有 `type` 和 `endpoint` 字段，新结构不包含。用 `#[serde(default)]` 或直接忽略多余字段。

当前 `ProviderConfig` 有 `#[serde(rename = "type")]`，需要在结构体上方加 `#[serde(deny_unknown_fields)]` 的反向 — 即默认忽略未知字段。Rust serde 默认行为就是忽略未知字段，所以不需要额外处理。

但 `endpoint` 字段在新结构中不存在，需要确认旧配置文件反序列化不会失败。用 `#[serde(default)]` 在移除的字段位置不需要（已移除），serde 默认会忽略 JSON 中的多余字段。

- [ ] **Step 7: 更新 `config.rs` 中其他引用 `provider_type` 和 `endpoint` 的代码**

搜索 `config.rs` 中所有 `.provider_type` 和 `.endpoint` 的引用并移除/替换。

- [ ] **Step 8: 验证**

```bash
cd src-tauri && cargo test --lib config
```

预期：所有测试通过，无编译错误。

> **注意**: 此步骤完成后，`ai.rs`、`lib.rs` 等文件中仍有 `.provider_type` 和 `.endpoint` 的引用。这些会在子任务 F 中处理。如果 `cargo check` 因此报错，这是预期行为。

## 提交

```bash
git checkout -b refactor/provider-config-simplify
git add src-tauri/src/config.rs
git commit -m "refactor: simplify ProviderConfig, remove type and endpoint fields"
```
