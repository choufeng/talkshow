# Audio Recording Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现真实的音频录音功能，通过全局快捷键控制录音的开始/停止/取消，将录音保存为 FLAC 文件到系统临时目录。

**Architecture:** 新建 `recording.rs` 模块封装 `AudioRecorder`，使用 `cpal` 采集系统麦克风 PCM 数据，`hound` 写入 WAV 文件，系统 `flac` 命令转码为 FLAC。通过 `Arc<Mutex<AudioRecorder>>` 与现有快捷键 handler 集成。

**Tech Stack:** cpal 0.15, hound 3.5, Tauri 2, Rust

---

## File Structure

| 文件 | 职责 |
|------|------|
| `src-tauri/src/recording.rs` (新增) | `AudioRecorder` struct，音频采集、WAV 写入、FLAC 转码 |
| `src-tauri/src/lib.rs` (修改) | 集成 `AudioRecorder` 到快捷键 handler，修改事件 payload |
| `src-tauri/Cargo.toml` (修改) | 添加 `cpal` 和 `hound` 依赖 |
| `src-tauri/tauri.conf.json` (修改) | 添加 macOS 麦克风权限声明 |

---

### Task 1: 添加 Rust 依赖

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 添加 cpal 和 hound 依赖**

在 `[dependencies]` 末尾追加：

```toml
cpal = "0.15"
hound = "3.5"
```

- [ ] **Step 2: 验证编译通过**

Run: `cargo check`
Workdir: `.worktrees/feature-add-recording/src-tauri`
Expected: 编译成功，可能需要较长时间下载依赖

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: add cpal and hound dependencies for audio recording"
```

---

### Task 2: 添加 macOS 麦克风权限

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: 在 tauri.conf.json 中声明麦克风权限**

在 `"app"` 对象中添加 `macOS` 权限声明：

```json
{
  "app": {
    "macOSPrivateApi": true,
    "macos": {
      "permissions": {
        "microphone": "TalkShow needs microphone access to record audio."
      }
    },
    "windows": [
      {
        "title": "TalkShow",
        "width": 800,
        "height": 600,
        "visible": false
      }
    ],
    "security": {
      "csp": null
    }
  }
}
```

- [ ] **Step 2: 验证 JSON 合法**

Run: `python3 -m json.tool src-tauri/tauri.conf.json > /dev/null`
Workdir: `.worktrees/feature-add-recording`
Expected: 无输出（JSON 合法）

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "feat: add macOS microphone permission declaration"
```

---

### Task 3: 创建 recording.rs 模块 — 基础结构和辅助函数

**Files:**
- Create: `src-tauri/src/recording.rs`

- [ ] **Step 1: 创建 recording.rs，定义 RecordingResult 和辅助函数**

```rust
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecordingResult {
    pub path: PathBuf,
    pub duration_secs: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecordingCancelled {
    pub duration_secs: u64,
}

const RECORDINGS_DIR_NAME: &str = "talkshow";

pub fn recordings_dir() -> PathBuf {
    std::env::temp_dir().join(RECORDINGS_DIR_NAME)
}

pub fn generate_filename() -> String {
    let now = chrono::Local::now();
    format!("talkshow_{}.flac", now.format("%Y%m%d_%H%M%S"))
}

pub fn ensure_recordings_dir() -> Result<PathBuf, String> {
    let dir = recordings_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create recordings dir: {}", e))?;
    Ok(dir)
}

pub fn wav_to_flac(wav_path: &Path, flac_path: &Path) -> Result<(), String> {
    let output = std::process::Command::new("flac")
        .arg("--silent")
        .arg("--force")
        .arg("-o")
        .arg(flac_path)
        .arg(wav_path)
        .output()
        .map_err(|e| format!("Failed to execute flac: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("flac encoding failed: {}", stderr))
    } else {
        Ok(())
    }
}
```

> 注意：`generate_filename()` 使用了 `chrono`，需要在 Task 1 的 Cargo.toml 中额外添加 `chrono = "0.4"` 依赖。如果不想引入 chrono，可以用 `std::time::SystemTime` 手动格式化。本计划采用 `std::time::SystemTime` 方案以减少依赖。

- [ ] **Step 2: 替换 chrono 为纯标准库实现**

将 `generate_filename()` 改为：

```rust
pub fn generate_filename() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    let total_secs = duration.as_secs();
    let days = total_secs / 86400;

    let dt = chrono::DateTime::from_timestamp(total_secs as i64, 0)
        .unwrap_or_default();
    // ...
}
```

实际上，为减少依赖，直接用 `time` 模块手动计算：

```rust
pub fn generate_filename() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    let total_secs = now.as_secs();
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = (total_secs / 3600) % 24;

    // 计算从 Unix epoch 到现在的日期
    let days_since_epoch = total_secs / 86400;
    // 使用简化算法计算年月日
    let (year, month, day) = days_to_date(days_since_epoch);

    format!(
        "talkshow_{:04}{:02}{:02}_{:02}{:02}{:02}.flac",
        year, month, day, hours, mins, secs
    )
}

fn days_to_date(days_since_epoch: u64) -> (u64, u64, u64) {
    // 算法：从 1970-01-01 开始累加
    let mut days = days_since_epoch;
    let mut year = 1970u64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let month_days = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0u64;
    for (i, &d) in month_days.iter().enumerate() {
        if days < d {
            month = i as u64 + 1;
            break;
        }
        days -= d;
    }

    (year, month, days + 1)
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
```

- [ ] **Step 3: 验证编译**

Run: `cargo check`
Workdir: `.worktrees/feature-add-recording/src-tauri`
Expected: 编译成功

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/recording.rs
git commit -m "feat: add recording module with helpers (filename, dir, flac conversion)"
```

---

### Task 4: 实现 AudioRecorder 核心

**Files:**
- Modify: `src-tauri/src/recording.rs`

- [ ] **Step 1: 添加 AudioRecorder struct 和 new()**

在 `recording.rs` 中追加：

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const SAMPLE_RATE: u32 = 16000;
const CHANNELS: u16 = 1;

pub struct AudioRecorder {
    buffer: Arc<Mutex<Vec<i16>>>,
    stream: Option<cpal::Stream>,
    start_time: Option<Instant>,
}

impl AudioRecorder {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No microphone device available")?;

        let supported_configs = device
            .supported_input_configs()
            .map_err(|e| format!("Failed to query microphone configs: {}", e))?;

        let supported_config = supported_configs
            .find(|c| c.sample_format() == cpal::SampleFormat::I16)
            .ok_or("Microphone does not support 16-bit audio format")?;

        let config: cpal::StreamConfig = supported_config.with_sample_rate(cpal::SampleRate(SAMPLE_RATE));

        let buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = buffer.clone();

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let mut buf = buffer_clone.lock().unwrap();
                    buf.extend_from_slice(data);
                },
                |err| {
                    eprintln!("Audio input error: {}", err);
                },
                None,
            )
            .map_err(|e| format!("Failed to create audio stream: {}", e))?;

        Ok(Self {
            buffer,
            stream: Some(stream),
            start_time: None,
        })
    }
}
```

- [ ] **Step 2: 添加 start() 方法**

```rust
impl AudioRecorder {
    pub fn start(&mut self) -> Result<(), String> {
        // Clear previous buffer
        self.buffer.lock().unwrap().clear();

        // Start the audio stream
        if let Some(stream) = self.stream.as_ref() {
            stream.play().map_err(|e| format!("Failed to start recording: {}", e))?;
        } else {
            return Err("Audio stream not initialized".to_string());
        }

        self.start_time = Some(Instant::now());
        Ok(())
    }
}
```

- [ ] **Step 3: 添加 stop() 方法**

```rust
impl AudioRecorder {
    pub fn stop(&mut self) -> Result<RecordingResult, String> {
        // Stop the audio stream
        if let Some(stream) = self.stream.as_ref() {
            stream.pause().map_err(|e| format!("Failed to stop recording: {}", e))?;
        }

        let duration_secs = self
            .start_time
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);

        // Discard if recording is too short (likely accidental)
        if duration_secs == 0 {
            self.buffer.lock().unwrap().clear();
            self.start_time = None;
            return Err("Recording too short, discarded".to_string());
        }

        // Take buffer data
        let audio_data: Vec<i16> = {
            let mut buf = self.buffer.lock().unwrap();
            std::mem::take(&mut *buf)
        };
        self.start_time = None;

        // Ensure output directory exists
        let dir = ensure_recordings_dir()?;
        let flac_filename = generate_filename();
        let flac_path = dir.join(&flac_filename);

        // Write WAV to temp file first
        let wav_filename = flac_filename.replace(".flac", ".wav");
        let wav_path = dir.join(&wav_filename);

        let spec = hound::WavSpec {
            channels: CHANNELS,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(&wav_path, spec)
            .map_err(|e| format!("Failed to create WAV file: {}", e))?;

        for sample in &audio_data {
            writer
                .write_sample(*sample)
                .map_err(|e| format!("Failed to write WAV sample: {}", e))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV file: {}", e))?;

        // Try FLAC conversion, fallback to WAV
        let final_path = match wav_to_flac(&wav_path, &flac_path) {
            Ok(()) => {
                let _ = std::fs::remove_file(&wav_path);
                flac_path
            }
            Err(_) => {
                // Fallback: rename WAV to final location
                let final_name = flac_filename.replace(".flac", ".wav");
                let final_path = dir.join(&final_name);
                if wav_path != final_path {
                    let _ = std::fs::rename(&wav_path, &final_path);
                }
                final_path
            }
        };

        Ok(RecordingResult {
            path: final_path,
            duration_secs,
        })
    }
}
```

- [ ] **Step 4: 添加 cancel() 方法**

```rust
impl AudioRecorder {
    pub fn cancel(&mut self) -> u64 {
        if let Some(stream) = self.stream.as_ref() {
            let _ = stream.pause();
        }

        let duration_secs = self
            .start_time
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);

        self.buffer.lock().unwrap().clear();
        self.start_time = None;
        duration_secs
    }
}
```

- [ ] **Step 5: 添加 is_recording() 辅助方法**

```rust
impl AudioRecorder {
    pub fn is_recording(&self) -> bool {
        self.start_time.is_some()
    }
}
```

- [ ] **Step 6: 验证编译**

Run: `cargo check`
Workdir: `.worktrees/feature-add-recording/src-tauri`
Expected: 编译成功

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/recording.rs
git commit -m "feat: implement AudioRecorder with start/stop/cancel"
```

---

### Task 5: 集成 AudioRecorder 到 lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 添加 mod 声明和 import**

在 `lib.rs` 第 1 行 `mod config;` 后追加：

```rust
mod recording;
```

在现有 use 语句区域追加：

```rust
use recording::AudioRecorder;
```

- [ ] **Step 2: 修改 stop_recording 函数签名**

将现有的 `stop_recording` 函数（第 57-69 行）替换为接收 `AudioRecorder` 的版本：

```rust
fn stop_recording(
    app_handle: &tauri::AppHandle,
    recorder: &Arc<std::sync::Mutex<AudioRecorder>>,
    recording_start: &Arc<std::sync::Mutex<Option<Instant>>>,
    event_name: &str,
) {
    RECORDING.store(false, Ordering::Relaxed);
    let duration = recording_start
        .lock()
        .ok()
        .and_then(|mut start| start.take().map(|s| s.elapsed().as_secs()))
        .unwrap_or(0);

    match event_name {
        "recording:complete" => {
            match recorder.lock().ok().and_then(|mut r| r.stop()) {
                Ok(result) => {
                    let _ = app_handle.emit("recording:complete", result);
                }
                Err(e) => {
                    let _ = app_handle.emit("recording:error", e.to_string());
                }
            }
        }
        "recording:cancel" => {
            let cancelled = recording::RecordingCancelled { duration_secs: duration };
            let _ = app_handle.emit("recording:cancel", cancelled);
        }
        _ => {}
    }
}
```

> 注意：这里 recorder.stop() 可能会阻塞（因为包含 WAV 写入和 FLAC 转码）。对于通常的录音时长（几分钟），这在快捷键 handler 中阻塞是可以接受的。如果后续需要非阻塞，可以改用 tokio spawn。

- [ ] **Step 3: 在 setup() 中创建 AudioRecorder 实例**

在 setup() 闭包中，`recording_start` 声明之后（约第 228 行），添加：

```rust
let recorder: Arc<std::sync::Mutex<AudioRecorder>> =
    match AudioRecorder::new() {
        Ok(r) => Arc::new(std::sync::Mutex::new(r)),
        Err(e) => {
            eprintln!("Warning: Failed to initialize AudioRecorder: {}", e);
            // 创建一个 dummy recorder 以便 handler 不 panic
            // 实际 start() 调用时会返回错误
            Arc::new(std::sync::Mutex::new(unsafe {
                std::mem::zeroed()
            }))
        }
    };
```

> 实际上不应该用 `zeroed()`，应该用 `Option` 或让 `AudioRecorder::new()` 返回一个不依赖硬件的 fallback。更安全的做法是让 handler 在 start 时检查。

改为：

```rust
let recorder: Arc<std::sync::Mutex<AudioRecorder>> = Arc::new(
    std::sync::Mutex::new(AudioRecorder::new().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to initialize AudioRecorder: {}", e);
        panic!("AudioRecorder initialization failed: {}", e);
    }))
);
```

> 简单起见，如果初始化失败就直接 panic。macOS 上麦克风权限被拒绝是用户选择的问题，不应该静默继续。

实际上更好的做法是：允许初始化失败但不 panic，在 start 时再检查。需要修改 `AudioRecorder` 使其能处理"未初始化"状态。

- [ ] **Step 4: 重构 AudioRecorder 支持延迟初始化**

在 `recording.rs` 中，添加一个 `Unavailable` 变体：

```rust
pub enum AudioRecorder {
    Ready {
        buffer: Arc<Mutex<Vec<i16>>>,
        stream: cpal::Stream,
        start_time: Option<Instant>,
    },
    Unavailable(String),
}
```

并相应修改 `new()`、`start()`、`stop()`、`cancel()`、`is_recording()` 方法。

`new()` 失败时返回 `AudioRecorder::Unavailable(reason)`：

```rust
impl AudioRecorder {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(d) => d,
            None => return AudioRecorder::Unavailable("No microphone device available".into()),
        };
        // ... 同之前的逻辑 ...
        match stream_result {
            Ok(stream) => AudioRecorder::Ready {
                buffer,
                stream,
                start_time: None,
            },
            Err(e) => AudioRecorder::Unavailable(format!("Failed to create audio stream: {}", e)),
        }
    }
}
```

`start()` 在 `Unavailable` 状态下返回 Err：

```rust
pub fn start(&mut self) -> Result<(), String> {
    match self {
        AudioRecorder::Ready { buffer, stream, .. } => {
            buffer.lock().unwrap().clear();
            stream.play().map_err(|e| format!("Failed to start recording: {}", e))?;
            // 设置 start_time
            if let AudioRecorder::Ready { start_time, .. } = self {
                *start_time = Some(Instant::now());
            }
            Ok(())
        }
        AudioRecorder::Unavailable(reason) => Err(reason.clone()),
    }
}
```

> 注意 Rust 的 borrow checker 限制：不能在 match 中同时可变借用了 `self` 又不可变借用 `stream`。需要用内部可变性或重构为两个步骤。

**修正方案：** 将 start_time 提取为 `Arc<Mutex<Option<Instant>>>` 或使用 `Cell`/`RefCell`。更简单的方案是将 `AudioRecorder::Ready` 的字段拆开操作：

```rust
pub fn start(&mut self) -> Result<(), String> {
    let self_mut = self;
    match self_mut {
        AudioRecorder::Ready { buffer, stream, start_time } => {
            buffer.lock().unwrap().clear();
            stream.play().map_err(|e| format!("Failed to start recording: {}", e))?;
            *start_time = Some(Instant::now());
            Ok(())
        }
        AudioRecorder::Unavailable(reason) => Err(reason.clone()),
    }
}
```

> 这在 Rust 中是可以的，因为 match arm 的 `start_time` 是 `&mut Option<Instant>`，`stream` 是 `&mut Stream`，它们是 struct 的不同字段，Rust 允许同时可变借用不同字段。

- [ ] **Step 5: 验证编译**

Run: `cargo check`
Workdir: `.worktrees/feature-add-recording/src-tauri`
Expected: 编译成功

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/recording.rs src-tauri/src/lib.rs
git commit -m "feat: integrate AudioRecorder into shortcut handler"
```

---

### Task 6: 修改快捷键 handler 调用录音

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 修改 ESC handler 调用 cancel**

在快捷键 handler 的 ESC 分支（约第 248-258 行），替换为：

```rust
if id == esc_id {
    let is_recording = RECORDING.load(Ordering::Relaxed);
    if is_recording {
        let mut rec = recorder.lock().unwrap();
        rec.cancel();
        stop_recording(
            &app_handle,
            &recorder,
            &recording_start_handler,
            "recording:cancel",
        );
        restore_default_tray(&app_handle, default_icon_owned.clone());
    }
    return;
}
```

- [ ] **Step 2: 修改录音快捷键 handler 调用 start/stop**

在录音快捷键分支（约第 261-277 行），替换为：

```rust
if rec_id == Some(id) {
    let is_recording = RECORDING.load(Ordering::Relaxed);
    if is_recording {
        stop_recording(
            &app_handle,
            &recorder,
            &recording_start_handler,
            "recording:complete",
        );
        restore_default_tray(&app_handle, default_icon_owned.clone());
    } else {
        match recorder.lock().unwrap().start() {
            Ok(()) => {
                RECORDING.store(true, Ordering::Relaxed);
                *recording_start_handler.lock().unwrap() = Some(Instant::now());
                if let Some(tray) = app_handle.tray_by_id("main") {
                    let _ = tray.set_icon(Some(recording_icon_owned.clone()));
                }
            }
            Err(e) => {
                eprintln!("Failed to start recording: {}", e);
                // TODO: 发送系统通知
            }
        }
    }
    return;
}
```

- [ ] **Step 3: 将 recorder 添加到 handler 闭包的捕获列表**

在 `let recording_start_handler = recording_start.clone();` 之后（约第 239 行），添加：

```rust
let recorder_handler = recorder.clone();
```

然后修改 handler 闭包，将所有 `recorder` 引用改为 `recorder_handler`：

```rust
.with_handler(move |_app, shortcut, event| {
    // ... 使用 recorder_handler 替代 recorder ...
})
```

- [ ] **Step 4: 验证编译**

Run: `cargo check`
Workdir: `.worktrees/feature-add-recording/src-tauri`
Expected: 编译成功

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: wire up AudioRecorder start/stop/cancel in shortcut handler"
```

---

### Task 7: 添加系统通知支持

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 添加 tauri-plugin-notification 依赖**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 中添加：

```toml
tauri-plugin-notification = "2"
```

在 `lib.rs` 的 `tauri::Builder::default()` 中注册插件（在 `.plugin(tauri_plugin_opener::init())` 之后）：

```rust
.plugin(tauri_plugin_notification::init())
```

- [ ] **Step 2: 在录音失败时发送系统通知**

在 `lib.rs` 中添加通知辅助函数：

```rust
fn show_notification(app_handle: &tauri::AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    app_handle
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .ok();
}
```

- [ ] **Step 3: 在 start 失败时调用通知**

将 Task 6 Step 2 中的 `// TODO: 发送系统通知` 替换为：

```rust
Err(e) => {
    eprintln!("Failed to start recording: {}", e);
    show_notification(
        &app_handle,
        "录音失败",
        &format!("{}", e),
    );
}
```

- [ ] **Step 4: 验证编译**

Run: `cargo check`
Workdir: `.worktrees/feature-add-recording/src-tauri`
Expected: 编译成功

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs
git commit -m "feat: add system notifications for recording errors"
```

---

### Task 8: 手动集成测试

**Files:** 无新文件

- [ ] **Step 1: 构建并运行应用**

Run: `npm run tauri dev`
Workdir: `.worktrees/feature-add-recording`
Expected: 应用启动，系统托盘显示 TalkShow 图标

- [ ] **Step 2: 测试录音开始**

按录音快捷键（默认 `Control+Shift+Quote`）
Expected:
- 托盘图标变为红色录制图标
- 托盘 tooltip 显示 "录音中 00:00" 并开始计时
- 终端无错误输出

- [ ] **Step 3: 测试录音停止**

再次按录音快捷键
Expected:
- 托盘图标恢复默认
- 终端显示录音文件路径
- 在系统临时目录 `talkshow/` 下生成 `.flac` 文件
- 可以用 `afplay <文件路径>` 播放验证音频内容

- [ ] **Step 4: 测试 ESC 取消**

按录音快捷键开始录音，然后按 ESC
Expected:
- 托盘图标恢复默认
- 不生成任何文件
- 终端无错误

- [ ] **Step 5: 测试误触（短按）**

快速连按两次录音快捷键（间隔 <1 秒）
Expected:
- 录音被丢弃（时长为 0）
- 终端无错误

---

## 自审清单

### Spec 覆盖检查

| Spec 需求 | 对应 Task |
|-----------|-----------|
| cpal 采集麦克风音频 | Task 4 |
| hound 写入 WAV | Task 4 |
| flac 命令转码 | Task 3 + Task 4 |
| flac 不可用时回退 WAV | Task 4 stop() |
| 文件命名 `talkshow_YYYYMMDD_HHMMSS.flac` | Task 3 |
| 系统临时目录 `talkshow/` 子目录 | Task 3 |
| 快捷键 handler 集成 | Task 5 + Task 6 |
| ESC 取消 | Task 6 |
| 事件 `recording:complete` 携带 path + duration | Task 5 |
| 事件 `recording:cancel` 携带 duration | Task 5 |
| 系统通知提示错误 | Task 7 |
| macOS 麦克风权限 | Task 2 |
| 16kHz/16bit/mono | Task 4 常量 |
| 误触丢弃（时长 0） | Task 4 stop() |

### 类型一致性检查

- `RecordingResult` — `path: PathBuf`, `duration_secs: u64` — 在 recording.rs 定义，lib.rs 通过 serde Serialize 发送事件
- `RecordingCancelled` — `duration_secs: u64` — 同上
- `stop_recording` 函数签名在 Task 5 中定义，Task 6 中调用 — 一致

### 占位符扫描

- Task 6 Step 2 中原 `// TODO` 已在 Task 7 Step 3 中替换
- 无其他 TBD/TODO
