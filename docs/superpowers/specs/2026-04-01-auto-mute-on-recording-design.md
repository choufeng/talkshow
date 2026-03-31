# 录音时自动静音其他应用

## 背景

TalkShow 是一个语音转文字工具，用户通过全局快捷键录音。录音期间，其他应用（如 Spotify、浏览器）的声音会干扰录音体验。需要一个功能在录音开始时自动静音其他应用，录音结束后恢复。

## 需求

1. 录音开始时自动静音系统声音
2. 录音结束时立即恢复音量
3. 设置中提供开关，默认关闭
4. TalkShow 自身的通知音效（Ping/Pop/Submarine）不受影响
5. 应用崩溃或强制退出时，下次启动自动恢复被静音的音量

## 技术方案

### 为什么选择系统总音量静音

macOS 没有公开的 AppleScript API 来按应用控制音量。按应用静音需要 CoreAudio 的半私有 API（`AudioObjectSetPropertyData` 配合进程 PID），依赖 `coreaudio-sys`，代码复杂度高且可能在 macOS 更新后失效。

采用**系统总音量静音 + 通知音效时序控制**的务实方案，通过调整 `play_sound()` 的调用时序，确保 TalkShow 通知音效始终正常播放。

### 音效时序

```
录音开始：play_sound("Ping") → mute_system() → 开始录音
录音结束：stop_recording() → restore_volume() → play_sound("Submarine")
录音取消：restore_volume() → play_sound("Pop")
```

### 新增模块：`audio_control.rs`

独立的 Rust 模块，职责单一：

- `save_system_volume() -> Result<f64, String>` — 获取当前系统音量并保存到状态文件，返回音量值
- `mute_system() -> Result<(), String>` — 将系统音量设为 0
- `restore_volume() -> Result<(), String>` — 从状态文件读取并恢复音量，删除状态文件
- `cleanup_stale_state(app_data_dir: &PathBuf) -> Result<(), String>` — 启动时检查状态文件，如果存在且未过期（10 分钟内），恢复音量

实现方式：通过 `osascript -e "set volume output volume 0"` 控制音量，与项目现有模式一致。

状态文件路径：`{app_data_dir}/mute_state.json`，内容：

```json
{ "volume": 50, "timestamp": 1743484800000 }
```

### 配置结构

在 `FeaturesConfig` 中新增 `RecordingFeaturesConfig`：

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct RecordingFeaturesConfig {
    pub auto_mute: bool,
}

pub struct FeaturesConfig {
    pub transcription: TranscriptionConfig,
    pub translation: TranslationConfig,
    pub skills: SkillsConfig,
    pub recording: RecordingFeaturesConfig,
}
```

使用 `#[serde(default)]` 保证旧配置文件兼容，无需数据迁移。默认 `auto_mute: false`。

### 设置页面 UI

在设置页新增"录音"section，位于"快捷键"和"外观"之间：

```
┌─ 录音 ──────────────────────────────────┐
│  录音时自动静音                          │
│  开始录音后自动静音其他应用的声音，        │
│  录音结束后自动恢复。                     │
│  [  关  ]  ← toggle 开关                │
└──────────────────────────────────────────┘
```

### 集成点

**`lib.rs` 录音开始分支**（recording 快捷键 + translate 快捷键）：

```rust
// 现有流程
play_sound("Ping.aiff");
// 新增：在播放通知音之后静音
if app_config.features.recording.auto_mute {
    let _ = audio_control::save_system_volume(&app_data_dir);
    let _ = audio_control::mute_system();
}
// 继续录音...
```

**`lib.rs` stop_recording() 函数**：

```rust
// 函数开头，在停止录音之前恢复音量
let _ = audio_control::restore_volume();
// 然后继续现有流程...
```

**取消录音路径**（Escape 快捷键、indicator:cancel 事件）：

```rust
let _ = audio_control::restore_volume();
play_sound("Pop.aiff");
```

**`lib.rs` setup() 启动时**：

```rust
// 在 setup 开头执行异常恢复检查
let _ = audio_control::cleanup_stale_state(&app_data_dir);
```

### 文件变更清单

| 文件 | 变更 |
|------|------|
| `src-tauri/src/audio_control.rs` | 新增模块 |
| `src-tauri/src/lib.rs` | 集成静音/恢复调用到录音流程 |
| `src-tauri/src/config.rs` | 新增 `RecordingFeaturesConfig` |
| `src/lib/stores/config.ts` | 同步新增类型定义 |
| `src/routes/settings/+page.svelte` | 新增"录音"设置 section |

### 错误处理

所有 `audio_control` 调用使用 `let _ =` 忽略错误，静音失败不应阻断录音流程。错误通过 Logger 记录。
