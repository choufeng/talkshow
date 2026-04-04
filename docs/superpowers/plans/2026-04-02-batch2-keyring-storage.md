# Batch 2: P1 keyring 密钥存储 + 前端适配

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 使用系统密钥链（macOS Keychain / Windows Credential Manager / Linux Secret Service）存储 API Key，替代明文 JSON 文件。前端适配脱敏后的 API Key 显示和编辑流程。

**Architecture:** 新建 `keyring_store.rs` 模块作为密钥存储抽象层。修改配置读写流程：保存时将 api_key 写入 keyring 并从 JSON 剥离，加载时从 keyring 读取并合并。前端需要正确处理脱敏值的保存逻辑。

**Tech Stack:** Rust / keyring crate / SvelteKit

**前置依赖：** 无（但建议在 Batch 1 完成后执行，因为 Batch 1 已添加了 `mask_api_keys` 函数）

---

## File Structure

| 文件 | 操作 |
|------|------|
| `src-tauri/Cargo.toml` | 修改 — 添加 keyring 依赖 |
| `src-tauri/src/keyring_store.rs` | **新建** — 跨平台密钥存储 |
| `src-tauri/src/config.rs` | 修改 — 添加 `strip_api_keys` 函数 |
| `src-tauri/src/lib.rs` | 修改 — 注册模块 + 修改 get_config / save_config_cmd |
| `src/lib/stores/config.ts` | 修改 — 适配脱敏 API Key |

---

### Task 1: 添加 keyring 依赖并创建存储模块

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/keyring_store.rs`

- [ ] **Step 1: 添加 keyring 依赖**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 中添加：

```toml
keyring = "3"
```

- [ ] **Step 2: 创建 `src-tauri/src/keyring_store.rs`**

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

pub fn load_all_api_keys(provider_ids: &[String]) -> HashMap<String, String> {
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
Expected: 编译通过（会有 unused 警告，这是正常的）

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/keyring_store.rs
git commit -m "feat(security): add keyring-based API key storage module (H-1 partial)"
```

---

### Task 2: 将 keyring 集成到配置读写流程

**Files:**
- Modify: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs`

**Context:** 需要在 `save_config_cmd` 中将 api_key 写入 keyring 并从 JSON 中剥离；在 `get_config` 中从 keyring 读取 api_key 并合并返回（然后 mask）。

- [ ] **Step 1: 在 `config.rs` 中添加 `strip_api_keys` 函数**

在 `mask_api_keys` 函数之后添加：

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

- [ ] **Step 2: 在 `lib.rs` 顶部模块声明区域添加新模块**

在 `mod translation;` 之后添加：

```rust
mod keyring_store;
```

- [ ] **Step 3: 修改 `save_config_cmd`**

将当前的 `save_config_cmd`：

```rust
#[tauri::command]
fn save_config_cmd(app_handle: tauri::AppHandle, config: config::AppConfig) -> Result<(), String> {
    config::validate_config(&config)?;
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

    // 首次保存时迁移旧密钥到 keyring
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
            if !key.is_empty() && !key.contains("...") {
                keyring_store::store_api_key(&provider_id, &key)?;
            }
        }
    }

    config::save_config(&app_data_dir, &clean_config)
}
```

- [ ] **Step 4: 修改 `get_config`**

将当前的 `get_config`：

```rust
#[tauri::command]
fn get_config(app_handle: tauri::AppHandle) -> config::AppConfig {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let config = config::load_config(&app_data_dir);
    config::mask_api_keys(config)
}
```

改为：

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
    }

    config::mask_api_keys(config)
}
```

- [ ] **Step 5: 运行 cargo check 和测试**

Run: `cd src-tauri && cargo check && cargo test`
Expected: 编译通过，所有测试 PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "feat(security): integrate keyring into config save/load flow (H-1 complete)"
```

---

### Task 3: 前端适配脱敏 API Key

**Files:**
- Modify: `src/lib/stores/config.ts`

**Context:** `get_config` 现在返回脱敏后的 API Key（如 `sk-...mnop`）。前端需要正确处理：
1. 显示 — 脱敏值直接显示（已经是 `mode="password"`）
2. 保存 — 当 api_key 包含 `...` 时表示用户未修改，不应覆盖 keyring 中的值

- [ ] **Step 1: 验证前端当前行为**

前端 `config.ts:210-218` 的 `save` 方法直接将完整 `AppConfig` 发送到后端。由于后端 `save_config_cmd` 已经会跳过包含 `...` 的 api_key，前端无需额外修改。

确认 `src/routes/models/+page.svelte` 中 `EditableField` 的 `mode="password"` 行为：
- 显示时用脱敏值
- 用户清空输入框后保存 — api_key 为空字符串，keyring 中该密钥被删除
- 用户输入新值后保存 — api_key 为新值，keyring 中更新

- [ ] **Step 2: 手动测试完整流程**

Run: `npm run tauri dev`
验证：
1. 打开设置 → 模型页面，能看到已有 provider
2. 已有 API Key 显示为脱敏格式（如 `sk-...mnop`）
3. 不修改 API Key 直接保存，密钥不丢失（keyring 中保留）
4. 修改 API Key 后保存，新密钥生效
5. 清空 API Key 后保存，密钥被删除
6. 重启应用，API Key 仍然存在
7. 检查 `~/Library/Application Support/com.jiaxia.talkshow/config.json`，确认 api_key 字段为 null

- [ ] **Step 3: 如果前端需要修改，进行适配并 Commit**

```bash
git add src/lib/stores/config.ts src/routes/models/+page.svelte
git commit -m "fix(security): adapt frontend for masked API keys in keyring flow"
```
