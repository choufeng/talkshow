# 子任务 1：后端 Onboarding 状态管理

**所属设计**: `2026-04-05-onboarding-wizard-design.md`  
**依赖**: 无  
**可并行**: 是

---

## 目标

在后端 config 中新增 `onboarding_completed` 字段，并暴露 Tauri command 供前端查询和设置。

---

## 任务清单

### T1.1: config.rs 添加字段

**文件**: `src-tauri/src/config.rs`

**操作**:
- 在 `AppConfig` 结构体中新增 `onboarding_completed: bool` 字段
- 默认值为 `false`（通过 Default 实现或 serde default）
- 确保现有 config.json 缺少此字段时能正确反序列化（serde default）

### T1.2: lib.rs 添加 Tauri Command

**文件**: `src-tauri/src/lib.rs`

**操作**: 新增两个 Tauri command：

```rust
#[tauri::command]
fn get_onboarding_status(app_handle: tauri::AppHandle) -> bool {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let config = config::load_config(&app_data_dir);
    config.onboarding_completed
}

#[tauri::command]
fn set_onboarding_completed(app_handle: tauri::AppHandle, completed: bool) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut config = config::load_config(&app_data_dir);
    config.onboarding_completed = completed;
    config::save_config(&app_data_dir, &config)
}
```

- 在 `run()` 的 `invoke_handler` 中注册这两个 command

---

## 验证

```bash
cargo check
cargo test
```

---

## 验收标准

- `AppConfig` 包含 `onboarding_completed: bool` 字段
- 缺少此字段的旧 config.json 能正常加载（默认 `false`）
- 两个 command 注册到 Tauri invoke handler
- Rust 编译和测试通过
