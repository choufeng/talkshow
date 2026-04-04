# 浮窗性能优化设计

## 问题描述

用户通过快捷键触发录音或翻译功能时，录音浮窗呈现有明显延迟（>200ms）。该延迟最早出现于"自动调低系统其他 App 音量"功能引入之后。

## 根因分析

在 `src-tauri/src/lib.rs` 快捷键 handler 中，`show_indicator()` 被调用前，代码同步执行了多个耗时操作：

| 操作 | 耗时估算 | 说明 |
|------|----------|------|
| `osascript` 获取最前端应用名 | ~50ms | 启动子进程 |
| `clipboard::detect_selected_text()` | ~100ms | 含 osascript 发送 Cmd+C + 50ms sleep |
| `config::load_config()` | ~10ms | 磁盘 I/O |
| `audio_control::save_and_mute()` | ~100ms | 两次 osascript 调用 |
| **总计** | **~260ms** | 全部同步阻塞 |

此外，浮窗首次使用时动态创建，Webview 初始化 + URL 加载带来额外 ~50-100ms 开销。

## 优化方案

采用 **异步化 + 预加载** 组合方案。

### 架构

```
快捷键触发
    │
    ▼
[阶段1: 立即响应]  (< 10ms)
    ├── 更新录音状态原子变量
    ├── 立即显示浮窗
    └── 播放提示音、更新 tray
    │
    ▼
[阶段2: 后台异步]  (~300ms, 不阻塞UI)
    ├── 检测选中文本
    ├── 获取最前端应用名
    ├── 加载配置 & 自动静音
    ├── 更新浮窗内容 (replaceMode)
    ├── 隐藏主窗口
    └── 注册 ESC 快捷键
```

## 详细设计

### 改动文件

| 文件 | 变更 |
|------|------|
| `src-tauri/src/lib.rs` | 主要改动：启动逻辑、show_indicator、快捷键 handler |

### 1. 启动时预创建浮窗

**位置**: `lib.rs` 应用启动逻辑

在 `setup()` 中创建 indicator 窗口，初始隐藏：

```rust
let indicator_window = WebviewWindowBuilder::new(&app, "recording-indicator", tauri::WebviewUrl::App("/recording".into()))
    .inner_size(180.0, 48.0)
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .focusable(false)
    .build()
    .ok();

if let Some(w) = &indicator_window {
    #[cfg(target_os = "macos")]
    {
        let _ = macos::floating_panel::make_window_nonactivating(w);
    }
}
```

### 2. 重构 show_indicator()

**位置**: `lib.rs` 第 534-601 行

简化为获取预创建窗口 + show + emit：

```rust
fn show_indicator(app_handle: &tauri::AppHandle, selected_text: Option<&str>) {
    let payload = serde_json::json!({
        "replaceMode": selected_text.is_some(),
        "selectedPreview": selected_text.map(|t| t.chars().take(50).collect::<String>()).unwrap_or_default()
    });
    
    if let Some(window) = app_handle.get_webview_window(INDICATOR_LABEL) {
        // 动态调整位置
        if let Some(main_window) = app_handle.get_webview_window("main") {
            if let Ok(Some(monitor)) = main_window.primary_monitor() {
                let size = monitor.size();
                let scale = monitor.scale_factor();
                let screen_w = size.width as f64 / scale;
                let win_w = 180.0;
                let win_h = 48.0;
                let _ = window.set_position(tauri::LogicalPosition::new(
                    (screen_w - win_w) / 2.0,
                    size.height as f64 / scale - win_h - 24.0,
                ));
            }
        }
        
        let _ = window.show();
        let _ = app_handle.emit_to(INDICATOR_LABEL, "indicator:recording", &payload);
        return;
    }
    
    // Fallback: 保留原有创建逻辑
}
```

### 3. 重构快捷键 handler

**录音模式** (第 1212-1285 行) 和 **翻译模式** (第 1340-1403 行) 同样重构：

```rust
// 阶段1: 立即响应
show_indicator(&app_handle, None);
play_sound("Ping.aiff");
// 更新 tray...

// 阶段2: 后台异步
let app_handle_async = app_handle.clone();
std::thread::spawn(move || {
    // 耗时操作：检测文本、获取应用、自动静音等
    // 完成后通过 emit 更新浮窗状态
});
```

## 性能预期

| 指标 | 优化前 | 优化后 | 改善 |
|------|--------|--------|------|
| 浮窗可见延迟 | ~260ms | < 10ms | >25x |
| 首次创建浮窗 | ~50-100ms | 0ms（预加载） | 消除 |

## 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| 异步操作中浮窗内容延迟更新 | 时间差 < 300ms，用户感知不明显 |
| 后台线程生命周期 | Tauri AppHandle 是 Arc 包装，安全 |
| ESC 快捷键注册延迟 | 已有 500ms 防抖保护 |

## 测试策略

1. 手动测试：目测浮窗是否即时出现
2. 性能测试：添加 `Instant::now()` 日志测量各阶段耗时
3. 回归测试：确认录音、翻译、自动静音、选中文本检测正常
4. 边界测试：快速连续按快捷键，确认防抖正常
