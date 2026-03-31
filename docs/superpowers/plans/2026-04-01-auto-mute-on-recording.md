# 录音时自动静音 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 录音开始时自动静音系统音量，录音结束后恢复，在设置页提供开关。

**Architecture:** 新增 `audio_control.rs` 模块，通过 `osascript` 控制系统音量。用 JSON 状态文件持久化音量值，用于异常恢复。集成到现有录音流程的开始/结束/取消路径。

**Tech Stack:** Rust, osascript, serde_json, Svelte 5

---

### Task 1: 新增 `audio_control.rs` 模块

**Files:**
- Create: `src-tauri/src/audio_control.rs`
- Modify: `src-tauri/src/lib.rs:1` (添加 `mod audio_control;`)

- [ ] **Step 1: 创建 `audio_control.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MUTE_STATE_FILE: &str = "mute_state.json";
const MAX_STALE_SECONDS: u64 = 600;

#[derive(Serialize, Deserialize)]
struct MuteState {
    volume: f64,
    timestamp: u64,
}

fn state_file_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join(MUTE_STATE_FILE)
}

fn get_current_volume() -> Result<f64, String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg("output volume of (get volume settings)")
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("Failed to get volume".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    stdout
        .parse::<f64>()
        .map_err(|_| format!("Failed to parse volume: {}", stdout))
}

fn set_volume(volume: f64) -> Result<(), String> {
    let vol = volume.round() as i64;
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(format!("set volume output volume {}", vol))
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn save_and_mute(app_data_dir: &PathBuf, logger: Option<&logger::Logger>) -> Result<(), String> {
    let volume = get_current_volume()?;

    if volume == 0.0 {
        return Ok(());
    }

    let state = MuteState {
        volume,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    if let Some(parent) = state_file_path(app_data_dir).parent() {
        let _ = fs::create_dir_all(parent);
    }

    let content = serde_json::to_string(&state).map_err(|e| e.to_string())?;
    fs::write(state_file_path(app_data_dir), content).map_err(|e| e.to_string())?;

    set_volume(0.0)?;

    if let Some(lg) = logger {
        lg.info("audio_control", &format!("系统已静音 (原音量: {})", volume), None);
    }

    Ok(())
}

pub fn restore(app_data_dir: &PathBuf, logger: Option<&logger::Logger>) -> Result<(), String> {
    let path = state_file_path(app_data_dir);

    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let state: MuteState = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let _ = fs::remove_file(&path);

    set_volume(state.volume)?;

    if let Some(lg) = logger {
        lg.info("audio_control", &format!("系统音量已恢复 ({})", state.volume), None);
    }

    Ok(())
}

pub fn cleanup_stale_state(app_data_dir: &PathBuf) -> Result<(), String> {
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
        return Ok(());
    }

    let _ = fs::remove_file(&path);
    set_volume(state.volume)?;

    Ok(())
}
```

- [ ] **Step 2: 在 `lib.rs` 顶部添加模块声明**

在 `src-tauri/src/lib.rs` 第 8 行（`mod translation;` 之后）添加：

```rust
mod audio_control;
```

- [ ] **Step 3: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/audio_control.rs src-tauri/src/lib.rs
git commit -m "feat: add audio_control module for system mute/restore"
```

---

### Task 2: 新增配置字段

**Files:**
- Modify: `src-tauri/src/config.rs:206-212`
- Modify: `src/lib/stores/config.ts:72-84`

- [ ] **Step 1: 在 Rust 配置中新增 `RecordingFeaturesConfig`**

在 `src-tauri/src/config.rs` 中，在 `FeaturesConfig` 结构体定义之前（约第 206 行）新增：

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct RecordingFeaturesConfig {
    pub auto_mute: bool,
}
```

在 `FeaturesConfig` 中添加字段：

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct FeaturesConfig {
    pub transcription: TranscriptionConfig,
    pub translation: TranslationConfig,
    pub skills: SkillsConfig,
    pub recording: RecordingFeaturesConfig,
}
```

- [ ] **Step 2: 在前端 TypeScript 类型中同步新增**

在 `src/lib/stores/config.ts` 中，在 `SkillsConfig` 接口之后新增：

```typescript
export interface RecordingFeaturesConfig {
  auto_mute: boolean;
}
```

修改 `FeaturesConfig` 接口：

```typescript
export interface FeaturesConfig {
  transcription: TranscriptionConfig;
  translation: TranslationConfig;
  skills: SkillsConfig;
  recording: RecordingFeaturesConfig;
}
```

修改 `createConfigStore()` 中的默认值，在 `features` 对象内添加：

```typescript
recording: {
  auto_mute: false
}
```

- [ ] **Step 3: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml` 和 `npx tsc --noEmit`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/config.rs src/lib/stores/config.ts
git commit -m "feat: add recording.auto_mute config field"
```

---

### Task 3: 集成到录音流程

**Files:**
- Modify: `src-tauri/src/lib.rs` (setup 中启动时恢复、录音开始时静音、录音结束时恢复、取消时恢复)

- [ ] **Step 1: setup() 启动时执行异常恢复**

在 `src-tauri/src/lib.rs` 的 `.setup(|app| {` 块中，`let app_data_dir = app.path()...` 之后（约第 815 行之后），添加：

```rust
let _ = audio_control::cleanup_stale_state(&app_data_dir);
```

- [ ] **Step 2: 录音开始时静音（recording 快捷键分支）**

在 recording 快捷键处理中，`play_sound("Ping.aiff");`（约第 1046 行）之后、`show_indicator(...)` 之前，添加：

```rust
let app_data_dir_mute = app_handle.path().app_data_dir().unwrap_or_default();
let app_config_mute = config::load_config(&app_data_dir_mute);
if app_config_mute.features.recording.auto_mute {
    let _ = audio_control::save_and_mute(
        &app_data_dir_mute,
        app_handle.try_state::<Logger>().as_deref(),
    );
}
```

- [ ] **Step 3: 录音开始时静音（translate 快捷键分支）**

在 translate 快捷键处理中，`play_sound("Ping.aiff");`（约第 1141 行）之后、`show_indicator(...)` 之前，添加同样的代码：

```rust
let app_data_dir_mute = app_handle.path().app_data_dir().unwrap_or_default();
let app_config_mute = config::load_config(&app_data_dir_mute);
if app_config_mute.features.recording.auto_mute {
    let _ = audio_control::save_and_mute(
        &app_data_dir_mute,
        app_handle.try_state::<Logger>().as_deref(),
    );
}
```

- [ ] **Step 4: `stop_recording()` 函数开头恢复音量**

在 `stop_recording()` 函数体开头（约第 99 行，`let duration = ...` 之前），添加：

```rust
let app_data_dir_restore = app_handle.path().app_data_dir().unwrap_or_default();
let _ = audio_control::restore(
    &app_data_dir_restore,
    app_handle.try_state::<Logger>().as_deref(),
);
```

- [ ] **Step 5: 取消录音路径恢复音量（Escape 快捷键）**

在 Escape 快捷键处理中，所有调用 `stop_recording(..., "recording:cancel", ...)` 的地方，`stop_recording` 已经会恢复音量，无需额外处理。但 `destroy_indicator` 分支（非录音状态的取消）不需要恢复。确认 `stop_recording` 的 `recording:cancel` 分支已覆盖。

- [ ] **Step 6: 取消录音路径恢复音量（indicator:cancel 事件）**

同样由 `stop_recording("recording:cancel", ...)` 覆盖，无需额外处理。

- [ ] **Step 7: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: integrate auto-mute into recording start/stop/cancel flow"
```

---

### Task 4: 设置页面 UI

**Files:**
- Modify: `src/routes/settings/+page.svelte`

- [ ] **Step 1: 在设置页新增"录音" section**

在"快捷键" section（`</section>` 约第 60 行）和"外观" section（`<section>` 约第 62 行）之间，插入新的录音 section：

```svelte
  <section class="mb-10">
    <div class="text-xs text-muted-foreground uppercase tracking-wider mb-3">录音</div>
    <div class="rounded-xl border border-border bg-background-alt p-5">
      <div class="flex items-center justify-between gap-4">
        <div>
          <div class="text-[15px] font-semibold text-foreground mb-1">录音时自动静音</div>
          <div class="text-sm text-foreground-alt">开始录音后自动静音其他应用的声音，录音结束后自动恢复</div>
        </div>
        <button
          class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors {$config.features.recording.auto_mute ? 'bg-btn-primary-to' : 'bg-border'}"
          onclick={() => {
            const newConfig = {
              ...$config,
              features: {
                ...$config.features,
                recording: {
                  ...$config.features.recording,
                  auto_mute: !$config.features.recording.auto_mute
                }
              }
            };
            config.save(newConfig);
          }}
        >
          <span class="pointer-events-none block h-4 w-4 rounded-full bg-white shadow-lg transition-transform {$config.features.recording.auto_mute ? 'translate-x-5' : 'translate-x-0'}" />
        </button>
      </div>
    </div>
  </section>
```

- [ ] **Step 2: 编译验证**

Run: `npm run check` 或 `npx svelte-check --tsconfig ./tsconfig.json`
Expected: 无类型错误

- [ ] **Step 3: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat: add auto-mute toggle to settings page"
```

---

### Task 5: 手动验证

- [ ] **Step 1: 启动应用**

Run: `npm run tauri dev`

- [ ] **Step 2: 验证设置 UI**

打开设置页面，确认"录音" section 出现在"快捷键"和"外观"之间，toggle 开关默认为关。

- [ ] **Step 3: 验证关闭状态不静音**

保持 auto_mute 关闭，按录音快捷键，确认系统音量不变。

- [ ] **Step 4: 验证开启状态正常静音/恢复**

开启 auto_mute，播放一些音乐，按录音快捷键 → 确认音乐静音且 Ping 音效已播放 → 按快捷键停止 → 确认音乐恢复且 Submarine 音效播放。

- [ ] **Step 5: 验证取消路径**

开启 auto_mute，按录音快捷键 → 按 Escape 取消 → 确认音量恢复。

- [ ] **Step 6: 验证异常恢复**

开启 auto_mute，按录音快捷键（此时系统被静音）→ 用 Activity Monitor 强制杀掉 TalkShow 进程 → 重新启动 TalkShow → 确认系统音量自动恢复。
