# API Key 存储简化实现计划

**日期**: 2026-04-05  
**分支**: `feature/simplify-api-key-storage`  
**依赖**: `2026-04-05-simplify-api-key-storage-design.md`

---

## 概述

简化 API Key 存储架构，废弃 keyring，直接使用 config.json 存储。

---

## 任务清单

### T1: 删除 keyring_store.rs

**文件**: `src-tauri/src/keyring_store.rs`

**操作**: 删除整个文件

**验证**: 确认无其他文件引用 `keyring_store`

---

### T2: 移除 keyring 依赖

**文件**: `src-tauri/Cargo.toml`

**操作**:
```diff
- keyring = "3"
```

**验证**: `cargo check` 通过

---

### T3: 简化 config.rs

**文件**: `src-tauri/src/config.rs`

**操作**: 删除以下函数和测试

需要删除的函数：
- `mask_api_keys` (L430-443)
- `strip_api_keys` (L445-456)
- `merge_api_keys_into_config` (L458-468)

需要删除的测试：
- `test_mask_api_keys_masks_long_keys` (L776-791)
- `test_mask_api_keys_masks_short_keys` (L793-806)
- `test_mask_api_keys_preserves_empty_string` (L808-821)
- `test_merge_api_keys_into_config_fills_missing_keys` (L888-931)
- `test_merge_api_keys_into_config_overwrites_existing_key_from_keyring` (L932-955)
- `test_merge_api_keys_into_config_handles_no_keyring_keys` (L957-979)
- `test_merge_api_keys_into_config_ignores_unknown_provider_keys` (L981-1008)

**验证**: `cargo test` 通过

---

### T4: 简化 lib.rs

**文件**: `src-tauri/src/lib.rs`

#### 4.1 简化 `get_config` (L678-686)

**变更前**:
```rust
fn get_config(app_handle: tauri::AppHandle) -> config::AppConfig {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let config = config::load_config(&app_data_dir);
    let provider_ids: Vec<String> = config.ai.providers.iter().map(|p| p.id.clone()).collect();
    let keyring_keys = keyring_store::load_all_api_keys(&provider_ids);
    let config = config::merge_api_keys_into_config(config, &keyring_keys);
    config::mask_api_keys(config)
}
```

**变更后**:
```rust
fn get_config(app_handle: tauri::AppHandle) -> config::AppConfig {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    config::load_config(&app_data_dir)
}
```

#### 4.2 简化 `save_config_cmd` (L753-792)

**变更前**: 复杂的 keyring 存储逻辑

**变更后**:
```rust
fn save_config_cmd(app_handle: tauri::AppHandle, config: config::AppConfig) -> Result<(), String> {
    config::validate_config(&config)?;
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    config::save_config(&app_data_dir, &config)
}
```

#### 4.3 简化 `test_model_connectivity` (L802-949)

**变更前**:
```rust
let provider_ids: Vec<String> = app_config.ai.providers.iter().map(|p| p.id.clone()).collect();
let keyring_keys = keyring_store::load_all_api_keys(&provider_ids);
app_config = config::merge_api_keys_into_config(app_config, &keyring_keys);
```

**变更后**: 删除以上 3 行，直接使用 `load_config` 返回的配置

同时删除测试后保存时的 `strip_api_keys` 调用 (L940-942)：
```rust
// 变更前
let (clean_config, _) = config::strip_api_keys(app_config);
config::save_config(&app_data_dir, &clean_config)?;

// 变更后
config::save_config(&app_data_dir, &app_config)?;
```

**验证**: `cargo check` + `cargo test` 通过

---

### T5: 更新前端测试 Mock（如有）

**文件**: `src/routes/models/models-page.integration.test.ts`

**检查**: 确认 mock 数据中的 `api_key` 字段仍符合预期格式

**验证**: `npm test` 通过

---

### T6: 端到端验证

**操作**:
1. 启动开发服务器
2. 打开模型设置页面
3. 输入 API Key
4. 点击确认
5. 刷新页面，确认 Key 正确保存（显示为掩码）
6. 点击测试，确认连通性测试正常

---

## 执行顺序

```
T1 → T2 → T3 → T4 → T5 → T6
```

---

## 风险与回滚

| 风险 | 缓解措施 |
|------|---------|
| keyring 数据丢失 | 实施前备份 config.json |
| 依赖删除导致编译错误 | 逐步验证每步的 cargo check |
| 前端显示异常 | 检查 T5 测试 |

**回滚命令**: `git checkout HEAD~1` 回到上一个 commit
