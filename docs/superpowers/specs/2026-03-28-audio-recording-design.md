# 音频录音功能设计

## 概述

为 TalkShow 实现实际的音频录音功能。当前项目的"录音"仅为 UI 状态标志（`AtomicBool`），需要接入系统麦克风进行真实的音频采集，并将录音保存为 FLAC 文件。

## 需求

1. 按录音快捷键，开始采集系统默认麦克风的音频
2. 再次按录音快捷键，停止录音并保存为 FLAC 文件到临时目录
3. 按 ESC 取消录音，丢弃数据
4. 录音失败时通过系统通知提示用户

## 技术决策

| 项目 | 决定 | 理由 |
|------|------|------|
| 音频格式 | FLAC | Gemini STT 最推荐格式，Whisper 原生支持，无损压缩 |
| 采样率 | 16kHz | Gemini 和 Whisper 均推荐的语音识别最佳采样率 |
| 位深度 | 16-bit | 语音识别标准配置 |
| 声道 | 单声道 | 语音场景仅需单声道 |
| 采集库 | `cpal` | Rust 跨平台音频 I/O 事实标准 |
| WAV 编写 | `hound` | 纯 Rust，零依赖，成熟稳定 |
| FLAC 转码 | 系统 `flac` 命令 | macOS 自带，Windows/Linux 可 bundle |
| 实现层级 | Rust 后端 | 全局快捷键触发时 WebView 可能不可见 |
| 临时目录 | `std::env::temp_dir()/talkshow/` | 系统标准位置，跨平台 |
| 错误提示 | 系统通知 | 用户无需打开 App 即可感知 |

## 架构

### 模块结构

```
src-tauri/src/
├── lib.rs          # 修改：快捷键 handler 集成录音模块
├── config.rs       # 不变
└── recording.rs    # 新增：录音核心模块
```

### recording.rs — AudioRecorder

```rust
pub struct AudioRecorder {
    buffer: Arc<Mutex<Vec<i16>>>,
    stream: Option<cpal::Stream>,
    recording_start: Instant,
}
```

**方法：**

- `AudioRecorder::new() -> Result<Self>` — 初始化，检测默认麦克风可用性
- `start(&mut self) -> Result<()>` — 打开默认麦克风，开始采集 PCM 到内存 buffer
- `stop(&mut self) -> Result<PathBuf>` — 停止采集，写入 WAV 临时文件，调用 `flac` 转码，返回 FLAC 文件路径
- `cancel(&mut self)` — 停止采集，丢弃 buffer 数据

**文件命名：** `talkshow_YYYYMMDD_HHMMSS.flac`

**文件位置：** `{temp_dir}/talkshow/talkshow_20260328_163000.flac`

### 数据流

```
录音快捷键 → lib.rs handler
  → RECORDING = true
  → 更新托盘图标
  → AudioRecorder::start()
    → cpal 打开默认麦克风
    → 音频线程：PCM 数据写入 Arc<Mutex<Vec<i16>>>

停止快捷键 → lib.rs handler
  → result = AudioRecorder::stop()
    → 停止 cpal stream
    → hound 写入临时 WAV 文件
    → std::process::Command("flac") 转码
    → 删除临时 WAV，返回 FLAC 路径
  → RECORDING = false
  → 恢复默认托盘图标
  → Ok(path) → emit("recording:complete", { path, duration })
  → Err(e)   → 系统通知提示错误

ESC → lib.rs handler
  → AudioRecorder::cancel()
  → RECORDING = false
  → 恢复默认托盘图标
  → emit("recording:cancel", { duration })
```

### 与现有代码集成

**lib.rs 修改点：**

1. 在 `setup()` 闭包中创建 `AudioRecorder` 实例，用 `Arc<Mutex<>>` 共享给 handler
2. 录音快捷键 handler 中：
   - 未录音 → 调用 `recorder.start()`，失败则发通知并保持非录音状态
   - 录音中 → 调用 `recorder.stop()`，将文件路径通过事件传递给前端
3. ESC handler 中：调用 `recorder.cancel()`

**前端事件：**

| 事件 | 数据 | 触发时机 |
|------|------|---------|
| `recording:complete` | `{ path: string, duration: number }` | 正常停止录音 |
| `recording:cancel` | `{ duration: number }` | ESC 取消录音 |

**权限需求：**

- macOS：`NSMicrophoneUsageDescription`（Info.plist 中声明麦克风使用说明）
- Tauri capabilities：无需额外权限（录音在 Rust 层完成，不通过 WebView）

## Rust 依赖

```toml
[dependencies]
cpal = "0.15"    # 跨平台音频采集
hound = "3.5"    # WAV 文件读写
```

## 错误处理

| 场景 | 处理方式 |
|------|---------|
| 无可用麦克风设备 | 系统通知"未找到麦克风设备"，不进入录音状态 |
| 麦克风被占用 | 系统通知"麦克风被占用"，不进入录音状态 |
| 麦克风权限被拒绝 | 系统通知"请授予麦克风权限"，不进入录音状态 |
| `flac` 命令不存在 | 回退保存为 WAV，通知"FLAC 编码不可用，已保存为 WAV" |
| 磁盘空间不足 | 系统通知"磁盘空间不足" |
| 录音时长为 0（误触） | 静默取消，视为 `recording:cancel` |

## 内存与性能

- PCM buffer 内存占用：16kHz × 16bit × 1ch = 32 KB/s，每分钟约 1.92 MB
- 典型录音场景（几分钟到十几分钟）内存完全可控
- 取消录音时立即释放 buffer
- `flac` 转码通过 `std::process::Command` 在子进程中执行

## 线程模型

- cpal 音频回调：独立音频线程，通过 `Arc<Mutex<Vec<i16>>>` 共享 buffer
- `start()`/`stop()`/`cancel()`：Tauri handler 线程
- `flac` 转码：子进程，`stop()` 中阻塞等待完成

## 不包含

- 不包含前端录音状态 UI（当前设计范围仅为后端录音功能）
- 不包含语音转文字（后续功能）
- 不包含录音文件管理（清理、列表等）
- 不包含 `recording-indicator` 窗口（已有独立设计文档）
