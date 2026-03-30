# SenseVoice 本地转写服务集成设计

## 概述

将阿里达摩院的 SenseVoice-Small 语音识别模型以 ONNX Runtime 方式集成到 TalkShow 桌面应用中，作为第三种转写 provider（本地推理），无需网络连接和 Python 依赖。

**模型**: SenseVoice-Small（234M 参数，量化 ONNX 241MB）
**推理方式**: ONNX Runtime CPU，嵌入 Rust 进程
**语言**: 中文、英文、日文 + 自动检测
**能力**: 仅 ASR 转写（不含情感识别和事件检测）

## 架构

### 新增模块

```
src-tauri/src/
├── lib.rs           # 修改：SenseVoice provider 分支 + State 注册
├── ai.rs            # 不变：Vertex / OpenAI-Compatible 远端推理
├── sensevoice.rs    # 新增：SenseVoice 本地推理全部逻辑
├── config.rs        # 修改：新增 "sensevoice" provider_type + 内置 provider
├── recording.rs     # 不变
├── clipboard.rs     # 不变
└── logger.rs        # 不变
```

### Provider 集成

SenseVoice 作为第三种 provider_type，与现有的远端 API 调用并行：

```
ProviderConfig {
    provider_type: "vertex"            → ai.rs (gRPC via rig-vertexai)
    provider_type: "openai-compatible" → ai.rs (REST via rig-core)
    provider_type: "sensevoice"        → sensevoice.rs (本地 ONNX)  [新增]
}
```

转写调度（`lib.rs`）：

```
录音完成 → 读取 config.features.transcription
  → 找到对应的 provider
  → match provider_type
      "vertex" | "openai-compatible" → ai::send_audio_prompt()
      "sensevoice"                   → sensevoice_engine.transcribe()
  → 写入剪贴板 + 模拟粘贴（不变）
```

### 全局状态

`SenseVoiceEngine` 是重量级资源（~300MB），通过 Tauri State 管理：

- **懒加载**: 应用启动时不加载，首次使用时初始化
- **单例**: `OnceCell<SenseVoiceEngine>`，应用生命周期内唯一
- **预加载**: 用户选择 SenseVoice 为当前转写服务时可提前触发加载

```rust
struct SenseVoiceState {
    engine: OnceCell<SenseVoiceEngine>,
    download_progress: Arc<Mutex<DownloadProgress>>,
    language: Arc<Mutex<SenseVoiceLanguage>>,
}
```

## 推理管线

### 完整数据流

```
录音数据 (WAV bytes, 48kHz i16 PCM)
  │
  ▼
[1] 解码 WAV → f32 波形 (hound crate)
  │
  ▼
[2] 重采样 48kHz → 16kHz (rubato crate)
  │
  ▼
[3] FBank 特征提取 (kaldi-native-fbank)
  │   80 mel bins, 25ms frame, 10ms shift, hamming window, dither=0
  │   输入: waveform * (1 << 15)
  │   输出: (T, 80) f32
  │
  ▼
[4] LFR 特征堆叠 (自实现)
  │   lfr_m=7, lfr_n=6, 左填充 3 帧
  │   输出: (T_lfr, 560) f32
  │
  ▼
[5] CMVN 归一化 (自实现)
  │   从 am.mvn 解析 means/vars，feat = (feat + means) * vars
  │   输出: (T_lfr, 560) f32
  │
  ▼
[6] ONNX 推理 (ort crate)
  │   输入: feats(1,T_lfr,560), feats_len(1,), language(1,), textnorm(1,)
  │   输出: ctc_logits(1,T_lfr,vocab_size), encoder_out_lens(1,)
  │
  ▼
[7] CTC 解码 (自实现)
  │   argmax → token IDs, 去连续重复 + 去 blank
  │
  ▼
[8] SentencePiece 解码 (sentencepiece crate)
  │   token IDs → 原始文本
  │
  ▼
[9] 后处理 (自实现)
  │   去除 <|HAPPY|>、<|BGM|>、<|zh|>、<|withitn|> 等标签
  │   输出干净文本
  │
  ▼
最终转写文本
```

### 语言 ID 映射

| 显示名称 | language ID |
|----------|------------|
| 自动检测 | 0 |
| 中文 | 3 |
| 英文 | 4 |
| 日文 | 11 |

默认使用"自动检测"。textnorm 统一使用 `withitn`（ID=14）。

### 性能预估

| 音频时长 | 预计推理时间 |
|----------|------------|
| 5 秒 | ~40ms |
| 30 秒 | ~235ms |
| 60 秒 | ~470ms |

相比远端 API（1-3 秒），有 5-10 倍速度优势。

## 模型文件管理

### 文件清单

从 HuggingFace `haixuantao/SenseVoiceSmall-onnx` 下载：

| 文件 | 大小 | 用途 |
|------|------|------|
| `model_quant.onnx` | 241 MB | 量化 ONNX 模型 |
| `config.yaml` | 1.86 KB | 模型配置 |
| `am.mvn` | 11.2 KB | CMVN 均值方差 |
| `chn_jpn_yue_eng_ko_spectok.bpe.model` | 377 KB | BPE 分词模型 |
| `tokens.json` | 352 KB | 词汇表 |

**总大小**: ~242 MB

### 存储位置

`{app_data_dir}/models/sensevoice/`（macOS: `~/Library/Application Support/com.jiaxia.talkshow/models/sensevoice/`）

### 自动下载流程

```
用户首次选择 SenseVoice 作为转写服务
  → sensevoice::ensure_model(app_handle)
    → 检查目录是否存在且文件完整
      → 完整 → 返回路径
      → 不完整 → 触发下载
        → 前端显示下载进度对话框
        → 从 HuggingFace 逐文件下载（带进度事件）
        → 校验文件大小
        → 返回路径
```

下载 URL: `https://huggingface.co/haixuantao/SenseVoiceSmall-onnx/resolve/main/{filename}`

### 内置 Provider 注册

在 `config.rs` 的内置 providers 中新增：

```rust
ProviderConfig {
    id: "sensevoice",
    provider_type: "sensevoice",
    name: "SenseVoice (本地)",
    endpoint: String::new(),
    api_key: None,
    models: vec![ModelConfig {
        name: "SenseVoice-Small",
        capabilities: vec!["transcription".into()],
        verified: None,
    }],
}
```

SenseVoice provider 无需 endpoint、api_key、连通性测试。

### 下载取消与恢复

用户取消下载时，删除已下载的部分文件和整个模型目录，下次重新下载从头开始。不支持断点续传（文件来自 CDN，下载速度快，复杂度不值得）。

## 前端 UI

### 模型管理页变更

Provider 列表新增 SenseVoice (本地) 卡片，特殊性：
- 不显示 Endpoint 和 API Key 配置
- 不显示连通性测试按钮
- 显示模型下载状态和磁盘占用

#### 状态展示

| 状态 | 显示 | 可操作 |
|------|------|--------|
| 模型未下载 | "未下载 (242 MB)" + 下载按钮 | 点击下载 |
| 下载中 | 进度条 + 百分比 + 已下载/总大小 | 可取消 |
| 已就绪 | "已就绪" + 磁盘占用 | 可删除模型 |
| 下载失败 | "下载失败" + 重试按钮 | 重试 |

#### 语言选择

SenseVoice 模型卡片内新增语言下拉菜单：自动检测 / 中文 / 英文 / 日文。

### 转写服务选择器

在现有选择器中新增 SenseVoice 选项。选择 SenseVoice 时：
- 模型未下载 → 弹出下载对话框
- 下载完成 → 自动设为当前转写服务

## Tauri 命令

| 命令 | 类型 | 说明 |
|------|------|------|
| `get_sensevoice_status` | 新增 | 返回模型下载状态 |
| `download_sensevoice_model` | 新增 | 触发模型下载，通过事件发送进度 |
| `delete_sensevoice_model` | 新增 | 删除本地模型文件 |
| `set_sensevoice_language` | 新增 | 设置转写语言 |
| `test_model_connectivity` | 修改 | SenseVoice 跳过测试直接返回成功 |

### Tauri 事件

| 事件名 | Payload | 说明 |
|--------|---------|------|
| `sensevoice:download-progress` | `{file, downloaded, total, percent}` | 下载进度 |
| `sensevoice:download-complete` | `{}` | 下载完成 |
| `sensevoice:download-error` | `{error}` | 下载失败 |
| `sensevoice:model-loading` | `{}` | 模型加载到内存 |
| `sensevoice:model-ready` | `{}` | 模型加载完成 |

## 新增依赖

```toml
[dependencies]
ort = { version = "2.0.0-rc.12", features = ["ndarray"] }
ndarray = "0.17"
kaldi-native-fbank = "0.1"
sentencepiece = "0.13"
rubato = "0.15"
reqwest = { version = "0.12", features = ["stream"] }
```

## 错误处理

| 场景 | 处理 |
|------|------|
| 模型文件缺失 | 自动触发下载，下载失败则报错提示 |
| ONNX 加载失败 | 返回错误，提示重新下载模型 |
| 音频过短（<0.3s） | 返回空字符串 |
| 推理结果为空 | 返回空字符串 |
| 内存不足 | ort 返回错误，应用层提示 |
| 网络下载失败 | 重试机制，前端显示错误信息 |

## 风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| `kaldi-native-fbank` 与 Python 版数值不一致 | 特征不匹配，识别准确率下降 | 对比测试验证，必要时自行实现 |
| `ort` crate 仍为 rc 版本 | API 可能变化 | 锁定版本 |
| `sentencepiece` 依赖 C 库 | 交叉编译复杂 | 考虑纯 Rust BPE 实现 |
| 模型 242MB | 应用占用空间增大 | 运行时按需下载，支持删除 |

## 不在范围内

- GPU 加速（CoreML / CUDA）— 后续迭代
- 情感识别和音频事件检测 — 仅用 ASR 转写
- 流式推理 — SenseVoice 本身是非自回归模型，处理速度快，不需要
- 模型微调 — 桌面应用不需要
- 多语言完整 50+ 支持 — 仅中日英 + 自动检测
