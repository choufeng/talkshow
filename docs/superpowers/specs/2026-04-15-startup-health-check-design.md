# 启动时通用健康检查机制

## 背景

TalkShow 的 SenseVoice 本地转写功能依赖 ONNX Runtime 动态库（`.dylib`），当前采用惰性加载——用户首次触发转写时才检查依赖是否存在。如果缺失，用户会在录音完成后才看到错误，体验很差。

需要建立启动时的通用健康检查机制，提前发现并提示缺失的可选依赖。

## 目标

1. 应用启动时自动检查所有可选外部依赖
2. 检查结果以警告形式通知用户，不阻止应用启动
3. 提供可扩展的框架，未来新增依赖检查时只需实现 trait 并注册

## 设计

### Rust 端：health 模块

新增 `src-tauri/src/health/mod.rs`，定义核心类型：

```rust
pub enum HealthStatus {
    Ok,
    Warning { message: String, fix_hint: String },
}

pub struct HealthCheckResult {
    pub id: String,
    pub name: String,
    pub status: HealthStatus,
}

pub trait HealthCheck {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn check(&self) -> HealthStatus;
}
```

### 检查项

**当前：**

- `OnnxRuntimeCheck` — 检查 ONNX Runtime 动态库是否存在
  - 复用 `sensevoice/engine.rs` 中现有的 `find_onnxruntime_dylib()` 逻辑
  - 将该函数提取为 `pub(crate)` 以便 health 模块调用
  - 仅检查文件存在性，不执行 `ort::init_from()`（全局初始化不应提前执行）
  - fix_hint: `brew install onnxruntime` 或 `pip3 install onnxruntime`

**未来扩展示例：**

- FFmpeg 检查
- 音频设备可用性检查

新增检查项只需：实现 `HealthCheck` struct + 在注册列表中添加一行。

### 集成到启动流程

在 `lib.rs` 的 `setup` 闭包中，`app.manage(sensevoice_state)` 之前，执行所有注册的检查：

```rust
let health_checks: Vec<Box<dyn HealthCheck>> = vec![
    Box::new(health::OnnxRuntimeCheck),
];
let health_results: Vec<HealthCheckResult> = health_checks
    .iter()
    .map(|c| HealthCheckResult {
        id: c.id().to_string(),
        name: c.name().to_string(),
        status: c.check(),
    })
    .collect();
let health_state = health::HealthState { checks: health_results };
app.manage(health_state);
```

### Tauri Command

暴露 `get_health_status()` 命令供前端调用：

```rust
#[tauri::command]
pub fn get_health_status(state: tauri::State<HealthState>) -> Vec<HealthCheckResult> {
    state.checks.clone()
}
```

### 前端展示

主窗口加载后调用 `get_health_status()`，如有 `Warning` 状态，显示 toast 或通知条，包含 `fix_hint` 引导用户安装。

## 文件变更清单

| 文件 | 操作 |
|---|---|
| `src-tauri/src/health/mod.rs` | 新建：health 模块 |
| `src-tauri/src/sensevoice/engine.rs` | 修改：`find_onnxruntime_dylib()` 改为 `pub(crate)` |
| `src-tauri/src/lib.rs` | 修改：注册 health 模块、执行检查、注册 command |
| 前端相关文件 | 修改：调用 `get_health_status()` 并展示警告 |

## 约束

- 检查不阻止启动
- 检查逻辑轻量，不执行实际初始化（如 `ort::init_from()`）
- 错误信息使用中文，fix_hint 提供具体的安装命令
