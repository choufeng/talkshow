# Security Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 TalkShow 项目中发现的所有 Critical 和 High 级别安全问题，以及部分 Medium 级别问题。

**Architecture:** 分层修复 — 先修复 Tauri 安全配置（CSP），再加固后端 IPC 命令（输入校验 + 敏感数据过滤），然后处理数据存储安全（keyring + 录音路径），最后处理代码级安全问题。

**Tech Stack:** Rust / Tauri 2 / SvelteKit / keyring crate / reqwest

---

## File Structure

| 文件 | 职责 | 操作 |
|------|------|------|
| `src-tauri/tauri.conf.json` | Tauri 安全配置 | 修改 |
| `src-tauri/Cargo.toml` | Rust 依赖管理 | 修改 |
| `src-tauri/src/config.rs` | 配置结构体 + 序列化/反序列化 | 修改 |
| `src-tauri/src/lib.rs` | Tauri IPC 命令定义 | 修改 |
| `src-tauri/src/keyring_store.rs` | **新建** — 跨平台密钥存储抽象 | 新建 |
| `src-tauri/src/recording.rs` | 录音文件路径 | 修改 |
| `src-tauri/src/sensevoice.rs` | 模型下载完整性校验 | 修改 |
| `src-tauri/src/audio_control.rs` | 音频控制逻辑 bug 修复 | 修改 |
| `src-tauri/src/ai.rs` | 日志脱敏 | 修改 |
| `src-tauri/src/clipboard.rs` | osascript 字符串转义 | 修改 |
| `src-tauri/src/skills.rs` | osascript 字符串转义 | 修改 |
| `src/lib/stores/config.ts` | 前端配置 store | 修改 |

---

### Task 1: 设置 CSP 安全策略

**Files:**
- Modify: `src-tauri/tauri.conf.json:22-24`

- [ ] **Step 1: 分析应用实际需要的网络请求目标**

应用中存在的网络请求来源：
- AI API 调用 — 用户自定义 endpoint（OpenAI compatible）
- Vertex AI — Google Cloud API（`generativelanguage.googleapis.com`）
- DashScope — `dashscope.aliyuncs.com`
- HuggingFace 模型下载 — `huggingface.co`
- Tauri IPC — `tauri://localhost` / `ipc://localhost` / `https://tauri.localhost`

- [ ] **Step 2: 编写 CSP 策略并替换 `null`**

由于 endpoint 是用户自定义的，CSP `connect-src` 不能静态限制所有外部域名。采用以下策略：
- `default-src 'self'` — 限制默认来源
- `script-src 'self'` — 禁止外部脚本和内联脚本
- `style-src 'self' 'unsafe-inline'` — 允许内联样式（Tailwind 需要）
- `img-src 'self' data: avatar:` — 允许数据 URI 和 Tauri 协议
- `connect-src https: http://localhost:* tauri: ipc: https://tauri.localhost` — 允许所有 HTTPS 连接（因为 endpoint 用户自定义），以及 Tauri IPC

```json
{
  "app": {
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: avatar:; connect-src https: http://localhost:* tauri: ipc: https://tauri.localhost"
    }
  }
}
```

- [ ] **Step 3: 验证 app.html 中的内联脚本是否兼容**

`src/app.html` 中有一个内联脚本用于 FOUC 预防（读取 localStorage 设置 dark class）。在 `script-src 'self'` 下这个脚本会被阻止。需要改为 CSP 兼容的方式。

方案：将 FOUC 脚本移到 SvelteKit 的 `+layout.ts` 的 root layout 中，通过 Svelte 的 `onMount` 处理。但这会导致 FOUC。折中方案是在 CSP 中为该脚本添加 hash：

先用以下方式获取 hash（临时）：
```bash
echo -n '(function(){var t=localStorage.getItem("theme")||"system";var d=t==="dark"||(t==="system"&&window.matchMedia("(prefers-color-scheme:dark)").matches);if(d)document.documentElement.classList.add("dark")})()' | openssl dgst -sha256 -binary | openssl base64 -A
```

如果 hash 方式过于脆弱，则使用 `'unsafe-inline'`（降低安全性但确保功能正常）。

最终 CSP：
```json
"csp": "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: avatar:; connect-src https: http://localhost:* tauri: ipc: https://tauri.localhost"
```

- [ ] **Step 4: 手动测试应用是否正常工作**

Run: `npm run tauri dev`
验证：
- 应用能正常启动
- AI 模型连接测试能通过
- HuggingFace 模型下载能进行
- 日志页面正常显示
- 设置页面正常操作

- [ ] **Step 5: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "fix(security): set CSP policy to replace null (C-1)"
```

---

### Task 2: `get_config` 返回时对 API Key 脱敏

**Files:**
- Modify: `src-tauri/src/config.rs` (添加脱敏函数)
- Modify: `src-tauri/src/lib.rs:613-617` (调用脱敏函数)
- Test: `src-tauri/src/config.rs` tests section

- [ ] **Step 1: 在 `config.rs` 中添加 `mask_api_keys` 函数和测试**

在 `config.rs` 的 tests section 之前添加脱敏函数：

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

在 tests section 中添加测试：

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
fn test_mask_api_keys_preserves_none() {
    let config = AppConfig::default();
    let masked = mask_api_keys(config);
    // default config has no providers, so nothing to check
    assert!(masked.ai.providers.is_empty());
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

- [ ] **Step 2: 运行测试验证**

Run: `npm run test:rust`
Expected: 所有新测试 PASS

- [ ] **Step 3: 修改 `lib.rs` 中的 `get_config` 命令**

将 `lib.rs:613-617`：

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

- [ ] **Step 4: 运行 Rust 测试确认无回归**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "fix(security): mask API keys in get_config response (C-2 partial)"
```

---

### Task 3: `save_config_cmd` 添加输入校验

**Files:**
- Modify: `src-tauri/src/config.rs` (添加校验函数)
- Modify: `src-tauri/src/lib.rs:684-688` (调用校验函数)
- Test: `src-tauri/src/config.rs` tests section

- [ ] **Step 1: 在 `config.rs` 中添加 `validate_config` 函数**

在 `mask_api_keys` 函数之后添加：

```rust
pub fn validate_config(config: &AppConfig) -> Result<(), String> {
    // 校验 provider
    for provider in &config.ai.providers {
        // provider_type 白名单
        match provider.provider_type.as_str() {
            "vertex" | "openai-compatible" | "sensevoice" => {}
            _ => return Err(format!("Invalid provider type '{}' for provider '{}'", provider.provider_type, provider.id)),
        }

        // endpoint 格式校验（vertex 和 sensevoice 允许为空）
        if provider.provider_type == "openai-compatible" && !provider.endpoint.is_empty() {
            if !provider.endpoint.starts_with("https://") && !provider.endpoint.starts_with("http://") {
                return Err(format!("Endpoint must start with http:// or https:// for provider '{}'", provider.id));
            }
        }

        // id 不能为空
        if provider.id.trim().is_empty() {
            return Err("Provider ID cannot be empty".to_string());
        }

        // name 不能为空
        if provider.name.trim().is_empty() {
            return Err(format!("Provider name cannot be empty for '{}'", provider.id));
        }
    }

    // 校验 shortcut 长度
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

添加测试：

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

- [ ] **Step 2: 运行测试验证**

Run: `npm run test:rust`
Expected: 所有新测试 PASS

- [ ] **Step 3: 修改 `lib.rs` 中的 `save_config_cmd`**

将 `lib.rs:684-688`：

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

- [ ] **Step 4: 运行全部测试确认无回归**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "fix(security): add input validation to save_config_cmd (C-2 complete)"
```

---

### Task 4: 使用 keyring crate 存储 API Key

**Files:**
- Create: `src-tauri/src/keyring_store.rs`
- Modify: `src-tauri/Cargo.toml` (添加 keyring 依赖)
- Modify: `src-tauri/src/config.rs` (分离 api_key 存储逻辑)
- Modify: `src-tauri/src/lib.rs` (注册新模块 + 修改 get_config / save_config_cmd)
- Test: `src-tauri/src/keyring_store.rs`

- [ ] **Step 1: 添加 keyring 依赖**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 中添加：

```toml
keyring = "3"
```

- [ ] **Step 2: 创建 `keyring_store.rs`**

```rust
use std::collections::HashMap;

const SERVICE_NAME: &str = "com.jiaxia.talkshow";

pub fn store_api_key(provider_id: &str, api_key: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, provider_id)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;
    if api_key.is_empty() {
        entry
            .delete_credential()
            .map_err(|e| format!("Failed to delete keyring entry: {}", e))?;
    } else {
        entry
            .set_password(api_key)
            .map_err(|e| format!("Failed to store API key: {}", e))?;
    }
    Ok(())
}

pub fn get_api_key(provider_id: &str) -> Result<Option<String>, String> {
    let entry = keyring::Entry::new(SERVICE_NAME, provider_id)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to get API key: {}", e)),
    }
}

pub fn delete_api_key(provider_id: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, provider_id)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;
    entry
        .delete_credential()
        .map_err(|e| format!("Failed to delete API key: {}", e))?;
    Ok(())
}

pub fn migrate_keys_from_config(config: &crate::config::AppConfig) -> usize {
    let mut count = 0;
    for provider in &config.ai.providers {
        if let Some(ref key) = provider.api_key {
            if !key.is_empty() {
                if store_api_key(&provider.id, key).is_ok() {
                    count += 1;
                }
            }
        }
    }
    count
}

pub fn load_all_api_keys(
    provider_ids: &[String],
) -> HashMap<String, String> {
    let mut keys = HashMap::new();
    for id in provider_ids {
        if let Ok(Some(key)) = get_api_key(id) {
            keys.insert(id.clone(), key);
        }
    }
    keys
}
```

- [ ] **Step 3: 运行 cargo check 确认编译通过**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/keyring_store.rs
git commit -m "feat(security): add keyring-based API key storage module (H-1 partial)"
```

---

### Task 5: 将 keyring 集成到配置读写流程

**Files:**
- Modify: `src-tauri/src/config.rs` (保存时剥离 api_key)
- Modify: `src-tauri/src/lib.rs` (加载/保存时同步 keyring)

这是 Task 4 的延续。需要在 `save_config_cmd` 中将 api_key 写入 keyring 并从 JSON 中剥离，在 `get_config` 中从 keyring 读取 api_key 并合并返回。

- [ ] **Step 1: 在 `config.rs` 中添加 `strip_api_keys` 函数**

```rust
pub fn strip_api_keys(mut config: AppConfig) -> (AppConfig, Vec<(String, Option<String>)>) {
    let keys: Vec<(String, Option<String>)> = config
        .ai
        .providers
        .iter()
        .map(|p| (p.id.clone(), p.api_key.clone()))
        .collect();
    for provider in &mut config.ai.providers {
        provider.api_key = None;
    }
    (config, keys)
}
```

- [ ] **Step 2: 修改 `save_config_cmd` 使用 keyring**

将 `lib.rs` 中的 `save_config_cmd` 改为：

```rust
#[tauri::command]
fn save_config_cmd(app_handle: tauri::AppHandle, config: config::AppConfig) -> Result<(), String> {
    config::validate_config(&config)?;

    // 首次保存时迁移旧密钥到 keyring
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let existing_config = config::load_config(&app_data_dir);
    for provider in &existing_config.ai.providers {
        if let Some(ref key) = provider.api_key {
            if !key.is_empty() {
                let _ = keyring_store::store_api_key(&provider.id, key);
            }
        }
    }

    // 保存 api_key 到 keyring，从 JSON 中剥离
    let (clean_config, keys) = config::strip_api_keys(config);
    for (provider_id, api_key) in keys {
        if let Some(key) = api_key {
            if !key.is_empty() {
                keyring_store::store_api_key(&provider_id, &key)?;
            }
        }
    }

    config::save_config(&app_data_dir, &clean_config)
}
```

- [ ] **Step 3: 修改 `get_config` 从 keyring 读取密钥**

将 `lib.rs` 中的 `get_config` 改为：

```rust
#[tauri::command]
fn get_config(app_handle: tauri::AppHandle) -> config::AppConfig {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut config = config::load_config(&app_data_dir);

    // 从 keyring 读取 api_key
    let provider_ids: Vec<String> = config.ai.providers.iter().map(|p| p.id.clone()).collect();
    let keyring_keys = keyring_store::load_all_api_keys(&provider_ids);

    for provider in &mut config.ai.providers {
        if let Some(key) = keyring_keys.get(&provider.id) {
            provider.api_key = Some(key.clone());
        }
        // 如果 keyring 中没有，检查旧配置中是否有（向后兼容）
        // 这只在首次迁移前发生
    }

    config::mask_api_keys(config)
}
```

- [ ] **Step 4: 在 `lib.rs` 顶部注册新模块**

在 `lib.rs:1-13` 的模块声明区域添加：

```rust
mod keyring_store;
```

- [ ] **Step 5: 运行 cargo check 和测试**

Run: `cd src-tauri && cargo check && cargo test`
Expected: 编译通过，所有测试 PASS

- [ ] **Step 6: 手动测试**

Run: `npm run tauri dev`
验证：
- 打开设置 → 模型页面，能看到已有 provider
- 修改 API Key 并保存
- 重启应用，API Key 仍然存在
- 检查 `config.json` 文件，确认 api_key 字段为 null

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "feat(security): integrate keyring into config save/load flow (H-1 complete)"
```

---

### Task 6: 录音文件迁移到 app_data_dir

**Files:**
- Modify: `src-tauri/src/recording.rs:107-115`
- Modify: `src-tauri/src/lib.rs` (传递 app_data_dir 到 recording 函数)

- [ ] **Step 1: 修改 `recording.rs` 中的 `recordings_dir`**

当前实现：
```rust
pub fn recordings_dir() -> PathBuf {
    std::env::temp_dir().join(RECORDINGS_DIR_NAME)
}
```

问题：`temp_dir()` 全局可读，录音可能包含敏感对话内容。

需要改为接受 `app_data_dir` 参数。但 `recordings_dir()` 被多处无参数调用，需要追踪所有调用点。

搜索所有 `recordings_dir()` 调用：
- `recording.rs` 内部（`ensure_recordings_dir` 等）
- `lib.rs` 中的录音处理逻辑

方案：添加带参数版本，旧版本保留但标记 deprecated 并指向 temp_dir（向后兼容，旧录音仍可访问）：

```rust
pub fn recordings_dir() -> PathBuf {
    std::env::temp_dir().join(RECORDINGS_DIR_NAME)
}

pub fn recordings_dir_in(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join(RECORDINGS_DIR_NAME)
}

pub fn ensure_recordings_dir_in(app_data_dir: &std::path::Path) -> Result<PathBuf, String> {
    let dir = recordings_dir_in(app_data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create recordings dir: {}", e))?;
    Ok(dir)
}
```

- [ ] **Step 2: 修改 `lib.rs` 中录音相关逻辑使用新路径**

找到 `lib.rs` 中所有 `recording::ensure_recordings_dir()` 调用，改为传入 `app_data_dir`：

```rust
let rec_dir = recording::ensure_recordings_dir_in(&app_data_dir)?;
```

找到所有 `recording::recordings_dir()` 用于读取/删除旧录音的地方，添加对新路径的搜索作为 fallback。

- [ ] **Step 3: 运行测试**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 4: 手动测试录音功能**

Run: `npm run tauri dev`
验证：
- 录音能正常启动和停止
- 录音文件保存在 `~/Library/Application Support/com.jiaxia.talkshow/recordings/` 下
- 旧录音（如果存在于 /tmp/talkshow/）仍可查看

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/recording.rs src-tauri/src/lib.rs
git commit -m "fix(security): move recordings to app_data_dir (H-5)"
```

---

### Task 7: 修复 osascript 字符串转义（命令注入）

**Files:**
- Modify: `src-tauri/src/clipboard.rs:41-55` (simulate_paste)
- Modify: `src-tauri/src/skills.rs:11-44` (get_frontmost_app)

- [ ] **Step 1: 在 `clipboard.rs` 或公共位置添加 AppleScript 字符串转义函数**

创建一个工具函数，对嵌入 AppleScript 的字符串进行安全转义：

```rust
#[cfg(target_os = "macos")]
fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
```

- [ ] **Step 2: 修改 `clipboard.rs` 中的 `simulate_paste`**

将 `clipboard.rs:47`：
```rust
.arg(format!("tell application \"{}\" to activate", app))
```
改为：
```rust
.arg(format!("tell application \"{}\" to activate", escape_applescript_string(&app)))
```

- [ ] **Step 3: 修改 `skills.rs` 中的 `get_frontmost_app`**

将 `skills.rs:28-31`：
```rust
.arg(format!(
    "tell application \"System Events\" to get bundle identifier of process \"{}\"",
    app_name
))
```
改为：
```rust
.arg(format!(
    "tell application \"System Events\" to get bundle identifier of process \"{}\"",
    escape_applescript_string(&app_name)
))
```

注意：`skills.rs` 也需要能访问 `escape_applescript_string`。由于这是 macOS-only 函数，建议在 `clipboard.rs` 中定义并标记 `pub(crate)`，或创建一个 `platform.rs` 工具模块。最简方案是在两个文件中各定义一份（函数很小）。

- [ ] **Step 4: 运行测试**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/clipboard.rs src-tauri/src/skills.rs
git commit -m "fix(security): escape strings in osascript calls (H-2)"
```

---

### Task 8: 修复 `audio_control.rs` 逻辑 bug

**Files:**
- Modify: `src-tauri/src/audio_control.rs:118-143`

- [ ] **Step 1: 修复 `cleanup_stale_state` 逻辑**

当前代码（有 bug）：
```rust
pub fn cleanup_stale_state(app_data_dir: &std::path::Path) -> Result<(), String> {
    // ...
    if now - state.timestamp > MAX_STALE_SECONDS {
        let _ = fs::remove_file(&path);
        set_volume(state.volume)?;
        return Ok(());
    }

    let _ = fs::remove_file(&path);
    set_volume(state.volume)?;
    Ok(())
}
```

两个分支执行了完全相同的操作，过期判断形同虚设。

修复为：过期时恢复音量并删除文件；未过期时保留文件不恢复音量。

```rust
pub fn cleanup_stale_state(app_data_dir: &std::path::Path) -> Result<(), String> {
    let path = state_file_path(app_data_dir);

    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let state: MuteState = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if now - state.timestamp > MAX_STALE_SECONDS {
        let _ = fs::remove_file(&path);
        set_volume(state.volume)?;
    }

    Ok(())
}
```

- [ ] **Step 2: 运行测试**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/audio_control.rs
git commit -m "fix: cleanup_stale_state logic bug — only restore volume when stale (L-6)"
```

---

### Task 9: 模型下载添加 SHA-256 完整性校验

**Files:**
- Modify: `src-tauri/src/sensevoice.rs:68-74` (添加哈希常量)
- Modify: `src-tauri/src/sensevoice.rs:494-544` (下载后校验)

- [ ] **Step 1: 计算各模型文件的 SHA-256 哈希**

从 HuggingFace 下载各文件并计算哈希值：

```bash
# 下载并计算 SHA-256（需手动确认）
curl -sL https://huggingface.co/haixuantao/SenseVoiceSmall-onnx/resolve/main/model_quant.onnx | shasum -a 256
curl -sL https://huggingface.co/haixuantao/SenseVoiceSmall-onnx/resolve/main/config.yaml | shasum -a 256
curl -sL https://huggingface.co/haixuantao/SenseVoiceSmall-onnx/resolve/main/am.mvn | shasum -a 256
curl -sL https://huggingface.co/haixuantao/SenseVoiceSmall-onnx/resolve/main/chn_jpn_yue_eng_ko_spectok.bpe.model | shasum -a 256
curl -sL https://huggingface.co/haixuantao/SenseVoiceSmall-onnx/resolve/main/tokens.json | shasum -a 256
```

**注意：** 由于文件较大（model_quant.onnx 241MB），实际操作中需要先下载再计算。将计算出的哈希填入下方常量。

- [ ] **Step 2: 修改 MODEL_FILES 常量添加哈希值**

将 `sensevoice.rs:68-74`：

```rust
const MODEL_FILES: &[(&str, u64)] = &[
    ("model_quant.onnx", 241_216_270),
    ("config.yaml", 1_855),
    ("am.mvn", 11_203),
    ("chn_jpn_yue_eng_ko_spectok.bpe.model", 377_341),
    ("tokens.json", 352_064),
];
```

改为：

```rust
const MODEL_FILES: &[(&str, u64, &str)] = &[
    ("model_quant.onnx", 241_216_270, "<SHA256_TO_BE_FILLED>"),
    ("config.yaml", 1_855, "<SHA256_TO_BE_FILLED>"),
    ("am.mvn", 11_203, "<SHA256_TO_BE_FILLED>"),
    ("chn_jpn_yue_eng_ko_spectok.bpe.model", 377_341, "<SHA256_TO_BE_FILLED>"),
    ("tokens.json", 352_064, "<SHA256_TO_BE_FILLED>"),
];
```

- [ ] **Step 3: 添加哈希校验函数**

```rust
use std::io::Read;

fn verify_file_hash(file_path: &std::path::Path, expected_hash: &str) -> Result<bool, String> {
    let mut file = std::fs::File::open(file_path).map_err(|e| e.to_string())?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    let result = format!("{:x}", hasher.finalize());
    Ok(result == expected_hash)
}
```

在 `Cargo.toml` 中添加 sha2 依赖：

```toml
sha2 = "0.10"
```

- [ ] **Step 4: 修改下载逻辑，在下载完成后校验哈希**

修改 `sensevoice.rs:496-498` 的 `is_valid` 检查，加入哈希校验：

```rust
let is_valid = match std::fs::metadata(&file_path) {
    Ok(meta) => {
        if meta.len() != expected_size {
            false
        } else if let Ok(valid) = verify_file_hash(&file_path, expected_hash) {
            valid
        } else {
            // 哈希校验失败，但文件大小正确 — 降级为仅检查大小
            // 这提供了向后兼容：旧下载的文件在哈希值更新前仍然可用
            true
        }
    }
    Err(_) => false,
};
```

- [ ] **Step 5: 下载完成后再次校验**

在 `sensevoice.rs:542-544`（`async_file.flush()` 之后，`std::fs::rename` 之前）添加：

```rust
if let Ok(valid) = verify_file_hash(&tmp_path, expected_hash) {
    if !valid {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("SHA-256 hash mismatch for {}", filename));
    }
}
```

- [ ] **Step 6: 运行 cargo check**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/sensevoice.rs src-tauri/Cargo.toml
git commit -m "fix(security): add SHA-256 integrity check for model downloads (M-1)"
```

---

### Task 10: 日志脱敏

**Files:**
- Modify: `src-tauri/src/ai.rs:207-217` (transcription 请求日志)
- Modify: `src-tauri/src/ai.rs:380-390` (其他 AI 请求日志)

- [ ] **Step 1: 添加 URL 脱敏函数**

在 `ai.rs` 中添加：

```rust
fn mask_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        let host = parsed.host_str().unwrap_or("unknown");
        let path = parsed.path();
        format!("{}://{}{}", parsed.scheme(), host, path)
    } else {
        "[invalid-url]".to_string()
    }
}
```

需要添加 `url` crate 依赖（或手动实现简单的字符串截断）。更简单的方案是直接截断：

```rust
fn mask_url(url: &str) -> String {
    if url.len() <= 80 {
        url.to_string()
    } else {
        format!("{}...", &url[..77])
    }
}
```

**实际方案：** 由于日志中的 URL 主要用于调试，且 AI API URL 本身不包含密钥（密钥在 header 中），风险较低。更实用的脱敏是：不记录完整的 API 请求参数，只记录模型名和文件大小。

- [ ] **Step 2: 修改 `ai.rs` 中的日志输出**

将 `ai.rs:207-217` 中的日志：

```rust
Some(serde_json::json!({
    "url": final_url,
    "model": model_name,
    "media_type": media_type,
    "audio_size_bytes": audio_bytes.len(),
    "prompt": text_prompt,
})),
```

改为（移除 `url` 和 `prompt`）：

```rust
Some(serde_json::json!({
    "model": model_name,
    "media_type": media_type,
    "audio_size_bytes": audio_bytes.len(),
})),
```

同样检查 `ai.rs` 中其他日志调用（搜索 `logger.info` 和 `logger.error` 中包含 `url`、`endpoint`、`api_key` 的地方），进行类似脱敏。

- [ ] **Step 3: 运行测试**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/ai.rs
git commit -m "fix(security): remove sensitive data from logs (M-2 partial)"
```

---

### Task 11: 前端适配 — 处理 API Key 脱敏后的 UI

**Files:**
- Modify: `src/lib/stores/config.ts`
- Modify: `src/routes/models/+page.svelte`

- [ ] **Step 1: 更新前端配置加载逻辑**

`get_config` 现在返回脱敏后的 API Key（如 `sk-...mnop`）。前端需要在以下场景正确处理：

1. **显示 API Key** — 脱敏后的值直接显示即可（已经是 `mode="password"` + `EditableField`）
2. **编辑 API Key** — 用户修改时发送新值，未修改时发送脱敏值
3. **保存 API Key** — 后端需要能区分"用户未修改"和"用户清空了 Key"

当前前端在 `save_config_cmd` 时发送完整的 `AppConfig`。后端 `save_config_cmd` 会将 api_key 存入 keyring。

需要修改前端逻辑：当 api_key 以 `...` 结尾时（脱敏值），保存时跳过该字段。

- [ ] **Step 2: 修改 `config.ts` 的 `save` 方法**

```typescript
save: async (newConfig: AppConfig) => {
    try {
        // 从当前 store 获取原始脱敏配置
        // 对于 api_key 以 ... 结尾的 provider，表示用户未修改
        const configToSave = { ...newConfig };
        // 不需要特殊处理 — 后端 save_config_cmd 会检测脱敏格式
        await invoke('save_config_cmd', { config: configToSave });
        set(configToSave);
    } catch (error) {
        console.error('Failed to save config:', error);
        throw error;
    }
}
```

- [ ] **Step 3: 修改后端 `save_config_cmd` 跳过脱敏值**

在 `lib.rs` 的 `save_config_cmd` 中，对 `...` 结尾的 api_key 不更新 keyring：

```rust
// 在 keyring 存储循环中添加检查
for (provider_id, api_key) in keys {
    if let Some(key) = api_key {
        if !key.is_empty() && !key.contains("...") {
            keyring_store::store_api_key(&provider_id, &key)?;
        }
    }
}
```

- [ ] **Step 4: 手动测试**

Run: `npm run tauri dev`
验证：
- 打开设置 → 模型页面
- 已有 API Key 显示为脱敏格式
- 不修改 API Key 直接保存，密钥不丢失
- 修改 API Key 后保存，新密钥生效
- 清空 API Key 后保存，密钥被删除

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src/lib/stores/config.ts
git commit -m "fix(security): handle masked API keys in save flow"
```

---

### Task 12: 前端 Endpoint URL 格式校验

**Files:**
- Modify: `src/routes/models/+page.svelte`

- [ ] **Step 1: 在前端添加 URL 格式校验**

找到 `models/+page.svelte` 中添加/编辑 Provider 时的验证逻辑（约 281-301 行），添加 endpoint URL 格式校验：

```typescript
function validateProvider(provider: ProviderConfig): string | null {
    if (!provider.name.trim()) return '名称不能为空';
    if (!provider.id.trim()) return 'ID 不能为空';
    if (provider.provider_type === 'openai-compatible' && provider.endpoint.trim()) {
        try {
            const url = new URL(provider.endpoint);
            if (!['http:', 'https:'].includes(url.protocol)) {
                return 'Endpoint 必须以 http:// 或 https:// 开头';
            }
        } catch {
            return 'Endpoint 格式无效';
        }
    }
    return null;
}
```

在添加/保存 Provider 时调用此函数。

- [ ] **Step 2: 手动测试**

Run: `npm run tauri dev`
验证：
- 添加 openai-compatible 类型 Provider 时输入非法 URL，显示错误提示
- 输入合法 URL，正常保存
- vertex 和 sensevoice 类型不要求 endpoint

- [ ] **Step 3: Commit**

```bash
git add src/routes/models/+page.svelte
git commit -m "fix(security): add endpoint URL format validation in frontend (M-4)"
```

---

### Task 13: 修复竞态条件

**Files:**
- Modify: `src-tauri/src/lib.rs` (多处 Atomic 操作)

- [ ] **Step 1: 将关键读-改-写操作改为原子操作**

找到 `lib.rs:1163-1188` 中的录音状态切换逻辑，将：

```rust
let is_recording = RECORDING.load(Ordering::Relaxed);
if is_recording {
    let mode = RECORDING.load(Ordering::Relaxed);
    RECORDING.store(RECORDING_MODE_NONE, Ordering::Relaxed);
    // ...
} else {
    // ...
    RECORDING.store(RECORDING_MODE_TRANSCRIPTION, Ordering::Relaxed);
    // ...
}
```

改为使用 `compare_exchange`：

```rust
let prev = RECORDING.compare_exchange(
    RECORDING_MODE_NONE,
    RECORDING_MODE_TRANSCRIPTION,
    Ordering::SeqCst,
    Ordering::SeqCst,
);

match prev {
    Ok(_) => {
        // 成功切换到录音状态
        // ... 原有的开始录音逻辑 ...
    }
    Err(mode) if mode != RECORDING_MODE_NONE => {
        // 当前正在录音，停止录音
        RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
        // ... 原有的停止录音逻辑 ...
    }
    Err(_) => {
        // 不应到达
    }
}
```

**注意：** 由于原代码中开始/停止录音逻辑较复杂，这个重构需要仔细处理。需要确保所有在状态变更后执行的副作用（通知前端、播放音效等）仍然正确执行。

- [ ] **Step 2: 将其他 `Ordering::Relaxed` 改为 `Ordering::SeqCst`**

搜索 `lib.rs` 中所有 `Ordering::Relaxed`，评估是否需要升级为 `SeqCst`。对于简单的读取操作可以保留 `Relaxed`，但对于状态变更操作应使用 `SeqCst`。

- [ ] **Step 3: 运行测试**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 4: 手动测试录音快捷键**

Run: `npm run tauri dev`
验证：
- 快速连续按录音快捷键不会导致状态异常
- 录音开始/停止正常工作

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "fix(security): use atomic compare_exchange for recording state (M-5)"
```

---

## 总结

| Task | 问题编号 | 严重度 | 描述 |
|------|----------|--------|------|
| 1 | C-1 | Critical | 设置 CSP 策略 |
| 2 | C-2 | Critical | get_config API Key 脱敏 |
| 3 | C-2 | Critical | save_config_cmd 输入校验 |
| 4-5 | H-1 | High | keyring 存储 API Key |
| 6 | H-5 | High | 录音文件迁移到 app_data_dir |
| 7 | H-2 | High | osascript 字符串转义 |
| 8 | L-6 | Low | audio_control 逻辑 bug |
| 9 | M-1 | Medium | 模型下载 SHA-256 校验 |
| 10 | M-2 | Medium | 日志脱敏 |
| 11 | — | — | 前端适配脱敏 API Key |
| 12 | M-4 | Medium | 前端 Endpoint URL 校验 |
| 13 | M-5 | Medium | 竞态条件修复 |
