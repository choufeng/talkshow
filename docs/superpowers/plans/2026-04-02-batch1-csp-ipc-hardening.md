# Batch 1: P0 安全配置加固

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 2 个 Critical 级别安全问题 — 设置 CSP 策略 + 加固 IPC 命令（API Key 脱敏 + 输入校验）。

**Architecture:** 纯后端配置层修改，不涉及新依赖、新文件。修改 `tauri.conf.json`、`config.rs`、`lib.rs` 三个文件。

**Tech Stack:** Rust / Tauri 2

---

## File Structure

| 文件 | 操作 |
|------|------|
| `src-tauri/tauri.conf.json` | 修改 — 设置 CSP |
| `src-tauri/src/config.rs` | 修改 — 添加 `mask_api_keys`、`validate_config` 函数及测试 |
| `src-tauri/src/lib.rs` | 修改 — `get_config` 和 `save_config_cmd` 调用新函数 |

---

### Task 1: 设置 CSP 安全策略

**Files:**
- Modify: `src-tauri/tauri.conf.json:22-24`

**Context:** 当前 `"csp": null` 完全禁用了内容安全策略，是整个项目最关键的安全防线缺失。应用中存在以下网络请求来源：
- AI API 调用 — 用户自定义 endpoint（OpenAI compatible）
- Vertex AI — Google Cloud API
- DashScope — `dashscope.aliyuncs.com`
- HuggingFace 模型下载 — `huggingface.co`
- Tauri IPC — `tauri://localhost` / `ipc://localhost` / `https://tauri.localhost`

由于 endpoint 是用户自定义的，`connect-src` 不能静态限制所有外部域名。

`src/app.html` 中有一个内联脚本（FOUC 预防），需要 `'unsafe-inline'` 兼容。

- [ ] **Step 1: 替换 CSP 配置**

将 `src-tauri/tauri.conf.json` 中的：

```json
"security": {
  "csp": null
}
```

替换为：

```json
"security": {
  "csp": "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: avatar:; connect-src https: http://localhost:* tauri: ipc: https://tauri.localhost"
}
```

- [ ] **Step 2: 手动验证应用正常工作**

Run: `npm run tauri dev`
验证：
- 应用能正常启动
- AI 模型连接测试能通过
- HuggingFace 模型下载能进行
- 日志页面正常显示
- 设置页面正常操作

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "fix(security): set CSP policy to replace null (C-1)"
```

---

### Task 2: `get_config` 返回时对 API Key 脱敏

**Files:**
- Modify: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs:613-617`

**Context:** `get_config` 命令将完整的 `AppConfig`（含所有 provider 的 `api_key`）返回给前端，任何 WebView 中的脚本都可读取。需要脱敏后再返回。

- [ ] **Step 1: 在 `config.rs` 的 tests section 之前添加 `mask_api_keys` 函数**

```rust
pub fn mask_api_keys(mut config: AppConfig) -> AppConfig {
    for provider in &mut config.ai.providers {
        if let Some(ref key) = provider.api_key {
            if !key.is_empty() {
                if key.len() <= 8 {
                    provider.api_key = Some("*".repeat(key.len()));
                } else {
                    provider.api_key = Some(format!("{}...{}", &key[..3], &key[key.len()-4..]));
                }
            }
        }
    }
    config
}
```

- [ ] **Step 2: 在 `config.rs` 的 tests section 中添加测试**

```rust
#[test]
fn test_mask_api_keys_masks_long_keys() {
    let mut config = AppConfig::default();
    config.ai.providers.push(ProviderConfig {
        id: "test".to_string(),
        provider_type: "openai-compatible".to_string(),
        name: "Test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: Some("sk-abcdefghijklmnop".to_string()),
        models: vec![],
    });
    let masked = mask_api_keys(config);
    assert_eq!(masked.ai.providers[0].api_key, Some("sk-...mnop".to_string()));
}

#[test]
fn test_mask_api_keys_masks_short_keys() {
    let mut config = AppConfig::default();
    config.ai.providers.push(ProviderConfig {
        id: "test".to_string(),
        provider_type: "openai-compatible".to_string(),
        name: "Test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: Some("short".to_string()),
        models: vec![],
    });
    let masked = mask_api_keys(config);
    assert_eq!(masked.ai.providers[0].api_key, Some("*****".to_string()));
}

#[test]
fn test_mask_api_keys_preserves_empty_string() {
    let mut config = AppConfig::default();
    config.ai.providers.push(ProviderConfig {
        id: "test".to_string(),
        provider_type: "openai-compatible".to_string(),
        name: "Test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: Some("".to_string()),
        models: vec![],
    });
    let masked = mask_api_keys(config);
    assert_eq!(masked.ai.providers[0].api_key, Some("".to_string()));
}
```

- [ ] **Step 3: 运行测试**

Run: `npm run test:rust`
Expected: 所有新测试 PASS

- [ ] **Step 4: 修改 `lib.rs:613-617` 中的 `get_config`**

将：
```rust
#[tauri::command]
fn get_config(app_handle: tauri::AppHandle) -> config::AppConfig {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    config::load_config(&app_data_dir)
}
```

改为：
```rust
#[tauri::command]
fn get_config(app_handle: tauri::AppHandle) -> config::AppConfig {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let config = config::load_config(&app_data_dir);
    config::mask_api_keys(config)
}
```

- [ ] **Step 5: 运行全部测试确认无回归**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "fix(security): mask API keys in get_config response (C-2 partial)"
```

---

### Task 3: `save_config_cmd` 添加输入校验

**Files:**
- Modify: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs:684-688`

**Context:** `save_config_cmd` 接受前端传入的任意 `AppConfig` 对象，零校验直接持久化到磁盘。需要白名单校验关键字段。

- [ ] **Step 1: 在 `config.rs` 中 `mask_api_keys` 函数之后添加 `validate_config`**

```rust
pub fn validate_config(config: &AppConfig) -> Result<(), String> {
    for provider in &config.ai.providers {
        match provider.provider_type.as_str() {
            "vertex" | "openai-compatible" | "sensevoice" => {}
            _ => return Err(format!("Invalid provider type '{}' for provider '{}'", provider.provider_type, provider.id)),
        }

        if provider.provider_type == "openai-compatible" && !provider.endpoint.is_empty() {
            if !provider.endpoint.starts_with("https://") && !provider.endpoint.starts_with("http://") {
                return Err(format!("Endpoint must start with http:// or https:// for provider '{}'", provider.id));
            }
        }

        if provider.id.trim().is_empty() {
            return Err("Provider ID cannot be empty".to_string());
        }

        if provider.name.trim().is_empty() {
            return Err(format!("Provider name cannot be empty for '{}'", provider.id));
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

- [ ] **Step 2: 在 tests section 添加测试**

```rust
#[test]
fn test_validate_config_rejects_invalid_provider_type() {
    let mut config = AppConfig::default();
    config.ai.providers.push(ProviderConfig {
        id: "bad".to_string(),
        provider_type: "malicious-type".to_string(),
        name: "Bad".to_string(),
        endpoint: "https://evil.com".to_string(),
        api_key: None,
        models: vec![],
    });
    assert!(validate_config(&config).is_err());
}

#[test]
fn test_validate_config_rejects_non_https_endpoint() {
    let mut config = AppConfig::default();
    config.ai.providers.push(ProviderConfig {
        id: "bad".to_string(),
        provider_type: "openai-compatible".to_string(),
        name: "Bad".to_string(),
        endpoint: "ftp://evil.com".to_string(),
        api_key: None,
        models: vec![],
    });
    assert!(validate_config(&config).is_err());
}

#[test]
fn test_validate_config_allows_empty_endpoint_for_vertex() {
    let mut config = AppConfig::default();
    config.ai.providers.push(ProviderConfig {
        id: "vertex".to_string(),
        provider_type: "vertex".to_string(),
        name: "Vertex AI".to_string(),
        endpoint: "".to_string(),
        api_key: None,
        models: vec![],
    });
    assert!(validate_config(&config).is_ok());
}

#[test]
fn test_validate_config_rejects_empty_id() {
    let mut config = AppConfig::default();
    config.ai.providers.push(ProviderConfig {
        id: "".to_string(),
        provider_type: "openai-compatible".to_string(),
        name: "Test".to_string(),
        endpoint: "https://api.example.com".to_string(),
        api_key: None,
        models: vec![],
    });
    assert!(validate_config(&config).is_err());
}

#[test]
fn test_validate_config_rejects_too_long_shortcut() {
    let mut config = AppConfig::default();
    config.shortcut = "A".repeat(101);
    assert!(validate_config(&config).is_err());
}
```

- [ ] **Step 3: 运行测试**

Run: `npm run test:rust`
Expected: 所有新测试 PASS

- [ ] **Step 4: 修改 `lib.rs:684-688` 中的 `save_config_cmd`**

将：
```rust
#[tauri::command]
fn save_config_cmd(app_handle: tauri::AppHandle, config: config::AppConfig) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    config::save_config(&app_data_dir, &config)
}
```

改为：
```rust
#[tauri::command]
fn save_config_cmd(app_handle: tauri::AppHandle, config: config::AppConfig) -> Result<(), String> {
    config::validate_config(&config)?;
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    config::save_config(&app_data_dir, &config)
}
```

- [ ] **Step 5: 运行全部测试**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "fix(security): add input validation to save_config_cmd (C-2 complete)"
```
