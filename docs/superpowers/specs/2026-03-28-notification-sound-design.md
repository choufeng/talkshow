# 录音提示音 & ESC 键修复设计

## 问题

1. **缺少提示音反馈**：用户按下快捷键开始/停止/取消录音时没有声音反馈，无法确认操作是否生效。
2. **ESC 键全局拦截**：ESC 作为全局快捷键始终注册，导致其他应用（包括应用内）无法使用 ESC 键。

## 设计

### 1. 录音提示音

在 Rust 后端通过 `std::process::Command` 调用 macOS 系统命令 `afplay` 播放系统内置音效文件（`/System/Library/Sounds/`），无需引入额外依赖。

| 场景 | 系统音效 | 语义 |
|------|---------|------|
| 开始录音 | `Tink.aiff` | 轻快的"叮"，表示录音已开始 |
| 停止录音（完成） | `Tink.aiff` | 相同音效，表示录音正常结束 |
| 取消录音 (ESC) | `Pop.aiff` | 不同的"噗"声，表示操作被取消 |

**实现细节**：

- 在 `lib.rs` 中新增 `play_sound(sound_name: &str)` 函数
- 函数内部根据 `cfg!(target_os = "macos")` 条件编译，调用 `afplay /System/Library/Sounds/<sound_name>`
- 在 handler 的三个关键位置调用：开始录音成功后、停止录音后、取消录音后
- 播放为异步（`spawn` 子进程），不阻塞录音流程

### 2. ESC 键动态注册/注销

将 ESC 快捷键从"始终注册"改为"仅录音期间注册"。

**当前行为**：
- 应用启动时注册 ESC → 全局拦截所有 ESC 按键

**目标行为**：
- 应用启动时不注册 ESC
- 开始录音时注册 ESC 全局快捷键
- 停止/取消录音时注销 ESC 全局快捷键

**实现细节**：

- 在 handler 闭包中通过 `app_handle.global_shortcut()` 访问快捷键管理器
- 需要将 `AppHandle` clone 到 handler 闭包中（当前 handler 已有 `app_handle`）
- 开始录音成功后：`app_handle.global_shortcut().register(esc_shortcut)?`
- 停止/取消录音后：`app_handle.global_shortcut().unregister(esc_shortcut)?`
- ESC handler 保持现有的条件判断作为兜底（防止极端情况下状态不同步）
- 将 `esc_shortcut` clone 到 handler 闭包中使用

## 修改范围

仅修改 `src-tauri/src/lib.rs`，涉及：

1. 新增 `play_sound()` 函数
2. 修改应用启动时的快捷键注册（移除 ESC 的初始注册）
3. 修改 handler 中开始录音的逻辑（添加注册 ESC + 播放提示音）
4. 修改 handler 中停止录音的逻辑（添加注销 ESC + 播放提示音）
5. 修改 handler 中取消录音的逻辑（添加注销 ESC + 播放取消提示音）
