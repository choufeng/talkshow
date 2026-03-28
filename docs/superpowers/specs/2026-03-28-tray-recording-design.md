# 托盘录音指示器设计

## 概述

用系统托盘图标 + 快捷键实现录音功能，替代之前的悬浮窗口方案。录音状态完全在 Rust 端管理，不依赖前端窗口。

## 状态机

```
          快捷键 X                快捷键 X
IDLE ──────────→ RECORDING ──────────→ IDLE
                     │
                     │ ESC
                     ↓
                   IDLE
```

- **IDLE**：托盘显示默认图标，tooltip "TalkShow"
- **RECORDING**：托盘显示录音图标（红色圆点），tooltip "录音中 00:23"

## 快捷键

- 新增独立快捷键，默认 `Control+Shift+R`，存入 `AppConfig.recording_shortcut`
- 非录音状态按下 → 开始录音，切换到 RECORDING
- 录音状态按下 → 完成录音，切换回 IDLE
- ESC 键（全局）仅在录音状态有效 → 取消录音，切换回 IDLE

## 托盘图标

- 默认图标：现有的 TalkShow 图标
- 录音图标：红色圆点 PNG（`icons/recording.png`），录音时通过 `TrayIconBuilder::icon()` 或 `tray.set_icon()` 切换

## 计时器

- Rust 端 `std::time::Instant::now()` 计时
- 录音开始时记录起始时间
- 通过 `tokio::spawn` 每 200ms 更新 tooltip 显示格式 `录音中 MM:SS`

## 事件

完成/取消录音后，通过 Tauri 的 `app_handle.emit()` 发出全局事件：
- `recording:complete` — payload: `{ duration: u64 }`（秒数）
- `recording:cancel` — payload: `{ duration: u64 }`

## ESC 全局监听

通过 Tauri 的全局快捷键插件注册 ESC，仅在录音状态下处理取消逻辑。

## 文件变更

| 文件 | 操作 |
|------|------|
| `src-tauri/src/lib.rs` | 新增录音状态管理、录音快捷键注册、ESC 监听、tooltip 更新 |
| `src-tauri/src/config.rs` | `AppConfig` 新增 `recording_shortcut` 字段 |
| `icons/recording.png` | 新增录音状态托盘图标 |
| `src/routes/recording/+page.svelte` | 删除 |
| `src/routes/+page.svelte` | 移除录音按钮相关代码 |

## AppConfig 变更

```rust
pub struct AppConfig {
    pub shortcut: String,              // 现有：显示/隐藏主窗口
    pub recording_shortcut: String,    // 新增：录音快捷键，默认 "Control+Shift+KeyR"
}
```
