# SenseVoice 本地转写服务集成 — 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 SenseVoice-Small ONNX 模型集成到 TalkShow 桌面应用中，作为第三种本地转写 provider。

**Architecture:** 新增 `sensevoice.rs` 模块实现完整推理管线（音频预处理 → ONNX 推理 → 解码后处理），通过 `provider_type: "sensevoice"` 与现有 provider 架构集成。模型文件按需从 HuggingFace 下载到 app_data_dir，引擎懒加载单例管理。

**Tech Stack:** Rust (ort 2.0, ndarray, kaldi-native-fbank, sentencepiece, rubato, reqwest), SvelteKit 5, Tauri v2

**Spec:** `docs/superpowers/specs/2026-03-30-sensevoice-local-transcription-design.md`

---

## File Structure

```
创建:
  src-tauri/src/sensevoice.rs                  # SenseVoice 推理引擎 + 模型下载 + Tauri 命令

修改:
  src-tauri/Cargo.toml                         # 新增依赖
  src-tauri/src/lib.rs                         # 注册 sensevoice 模块 + State + 命令 + 转写调度分支
  src-tauri/src/config.rs                      # 新增内置 sensevoice provider
  src/lib/stores/config.ts                     # 前端内置 provider + 类型
  src/routes/models/+page.svelte               # SenseVoice 卡片 UI + 下载进度 + 语言选择
```

---

### Task 1: 添加 Cargo 依赖

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 在 `[dependencies]` 末尾添加新依赖**

```toml
ort = { version = "2.0.0-rc.12", features = ["ndarray"] }
ndarray = "0.17"
rubato = "0.15"
reqwest = { version = "0.12", features = ["stream"] }
```

注意：`kaldi-native-fbank` 和 `sentencepiece` 暂不添加，在 Task 3 和 Task 5 中分别处理。

- [ ] **Step 2: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译成功（可能有 unused warnings，无 error）

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: add ONNX Runtime and audio processing dependencies"
```

---

### Task 2: 添加内置 SenseVoice Provider 配置

**Files:**
- Modify: `src-tauri/src/config.rs`
- Modify: `src/lib/stores/config.ts`

- [ ] **Step 1: 在 `config.rs` 的 `builtin_providers()` 函数中，`dashscope` provider 后新增 sensevoice provider**

在 `builtin_providers()` 函数的 `vec![` 内，dashscope 的 `},` 之后、`];` 之前插入：

```rust
        ProviderConfig {
            id: "sensevoice".to_string(),
            provider_type: "sensevoice".to_string(),
            name: "SenseVoice (本地)".to_string(),
            endpoint: String::new(),
            api_key: None,
            models: vec![ModelConfig {
                name: "SenseVoice-Small".to_string(),
                capabilities: vec!["transcription".to_string()],
                verified: None,
            }],
        },
```

- [ ] **Step 2: 在 `config.ts` 的 `BUILTIN_PROVIDERS` 数组末尾新增 sensevoice provider**

在 `BUILTIN_PROVIDERS` 数组的 dashscope 对象后添加：

```typescript
  {
    id: 'sensevoice',
    type: 'sensevoice',
    name: 'SenseVoice (本地)',
    endpoint: '',
    models: [{ name: 'SenseVoice-Small', capabilities: ['transcription'] }]
  }
```

- [ ] **Step 3: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/config.rs src/lib/stores/config.ts
git commit -m "feat: add SenseVoice as builtin provider"
```

---

### Task 3: 创建 sensevoice.rs 骨架 — 错误类型 + 模型下载

**Files:**
- Create: `src-tauri/src/sensevoice.rs`

- [ ] **Step 1: 创建 `sensevoice.rs` 基础骨架**

创建文件 `src-tauri/src/sensevoice.rs`，包含错误类型、模型文件常量、下载逻辑和 Tauri 命令。

```rust
use ndarray::{Array1, Array2, Array3};
use ort::session::Session;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::Emitter;

const MODEL_DIR_NAME: &str = "sensevoice";
const HF_REPO: &str = "haixuantao/SenseVoiceSmall-onnx";

const MODEL_FILES: &[(&str, &str)] = &[
    ("model_quant.onnx", "241MB"),
    ("config.yaml", "1.86KB"),
    ("am.mvn", "11.2KB"),
    ("chn_jpn_yue_eng_ko_spectok.bpe.model", "377KB"),
    ("tokens.json", "352KB"),
];

#[derive(Debug)]
pub enum SenseVoiceError {
    ModelNotDownloaded,
    DownloadFailed(String),
    LoadFailed(String),
    InferenceFailed(String),
    InvalidAudio(String),
}

impl std::fmt::Display for SenseVoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SenseVoiceError::ModelNotDownloaded => write!(f, "SenseVoice 模型未下载"),
            SenseVoiceError::DownloadFailed(e) => write!(f, "模型下载失败: {}", e),
            SenseVoiceError::LoadFailed(e) => write!(f, "模型加载失败: {}", e),
            SenseVoiceError::InferenceFailed(e) => write!(f, "推理失败: {}", e),
            SenseVoiceError::InvalidAudio(e) => write!(f, "音频无效: {}", e),
        }
    }
}

impl std::error::Error for SenseVoiceError {}

fn model_dir(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join("models").join(MODEL_DIR_NAME)
}

pub fn model_status(app_data_dir: &PathBuf) -> SenseVoiceModelStatus {
    let dir = model_dir(app_data_dir);
    let onnx_path = dir.join("model_quant.onnx");
    if onnx_path.exists() {
        let size = std::fs::metadata(&onnx_path).map(|m| m.len()).unwrap_or(0);
        if size > 100_000_000 {
            return SenseVoiceModelStatus::Ready {
                size_bytes: size,
            };
        }
    }
    SenseVoiceModelStatus::NotDownloaded
}

#[derive(serde::Serialize, Clone)]
#[serde(tag = "status")]
pub enum SenseVoiceModelStatus {
    #[serde(rename = "not_downloaded")]
    NotDownloaded,
    #[serde(rename = "downloading")]
    Downloading { file: String, percent: f64, downloaded: u64, total: u64 },
    #[serde(rename = "ready")]
    Ready { size_bytes: u64 },
    #[serde(rename = "error")]
    Error { message: String },
}

pub struct SenseVoiceEngine {
    session: Session,
    cmvn_means: Array1<f64>,
    cmvn_vars: Array1<f64>,
}

impl SenseVoiceEngine {
    pub fn new(model_dir: &PathBuf) -> Result<Self, SenseVoiceError> {
        let onnx_path = model_dir.join("model_quant.onnx");
        let session = Session::builder()
            .and_then(|b| b.with_intra_threads(4))
            .and_then(|b| b.commit_from_file(&onnx_path))
            .map_err(|e| SenseVoiceError::LoadFailed(e.to_string()))?;

        let (cmvn_means, cmvn_vars) = parse_cmvn(&model_dir.join("am.mvn"))
            .map_err(|e| SenseVoiceError::LoadFailed(e))?;

        Ok(Self { session, cmvn_means, cmvn_vars })
    }

    pub fn transcribe(&self, wav_path: &PathBuf, language: i32) -> Result<String, SenseVoiceError> {
        let waveform = load_wav(wav_path)?;
        let waveform_16k = resample_to_16k(&waveform)?;
        if waveform_16k.len() < 4800 {
            return Ok(String::new());
        }
        let feats = extract_fbank(&waveform_16k)?;
        let feats = apply_lfr(&feats);
        let feats = apply_cmvn(&feats, &self.cmvn_means, &self.cmvn_vars);
        let (feats_padded, feats_len) = pad_features(&feats);
        let token_ids = self.infer(&feats_padded, feats_len, language)?;
        let text = decode_tokens(&token_ids, &model_dir)?;
        let text = postprocess(&text);
        Ok(text)
    }

    fn infer(
        &self,
        feats: &Array3<f32>,
        feats_len: i32,
        language: i32,
    ) -> Result<Vec<i32>, SenseVoiceError> {
        let (_, t_lfr, _) = feats.dim();
        let feats_len_arr = Array1::from_vec(vec![feats_len]);
        let language_arr = Array1::from_vec(vec![language]);
        let textnorm_arr = Array1::from_vec(vec![14i32]);

        let outputs = self.session.run(ort::inputs![
            ort::value::TensorRef::from_array_view(&feats.view()),
            ort::value::TensorRef::from_array_view(&feats_len_arr.view()),
            ort::value::TensorRef::from_array_view(&language_arr.view()),
            ort::value::TensorRef::from_array_view(&textnorm_arr.view()),
        ].unwrap()).map_err(|e| SenseVoiceError::InferenceFailed(e.to_string()))?;

        let logits = outputs[0].try_extract_tensor::<f32>()
            .map_err(|e| SenseVoiceError::InferenceFailed(e.to_string()))?;
        let logits_view = logits.view();
        let (_, _, vocab_size) = logits.dim();
        let mut token_ids = Vec::new();
        for t in 0..t_lfr {
            let row = logits_view.row(t).to_slice().unwrap_or(&[]);
            if let Some((idx, _)) = row.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)) {
                if *idx != 0 && token_ids.last().copied() != Some(idx as i32) {
                    token_ids.push(idx as i32);
                }
            }
        }
        Ok(token_ids)
    }
}

fn parse_cmvn(mvn_path: &PathBuf) -> Result<(Array1<f64>, Array1<f64>), String> {
    let content = std::fs::read_to_string(mvn_path)
        .map_err(|e| format!("Failed to read am.mvn: {}", e))?;
    let mut means: Vec<f64> = Vec::new();
    let mut vars: Vec<f64> = Vec::new();
    let mut current_section = "";
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("<AddShift>") {
            current_section = "means";
            continue;
        } else if trimmed.contains("<Rescale>") {
            current_section = "vars";
            continue;
        }
        if trimmed.starts_with('[') || trimmed.starts_with(']') || trimmed.is_empty() {
            continue;
        }
        if current_section == "means" || current_section == "vars" {
            for val in trimmed.split_whitespace() {
                if let Ok(v) = val.parse::<f64>() {
                    if current_section == "means" {
                        means.push(v);
                    } else {
                        vars.push(v);
                    }
                }
            }
        }
    }
    Ok((Array1::from_vec(means), Array1::from_vec(vars)))
}

fn load_wav(path: &PathBuf) -> Result<Vec<f32>, SenseVoiceError> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|e| SenseVoiceError::InvalidAudio(e.to_string()))?;
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = 2i32.pow(spec.bits_per_sample as u32 - 1) as f32;
            reader.samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max_val)
                .collect()
        }
        hound::SampleFormat::Float => {
            reader.samples::<f32>()
                .filter_map(|s| s.ok())
                .collect()
        }
    };
    Ok(samples)
}

fn resample_to_16k(samples: &[f32]) -> Result<Vec<f32>, SenseVoiceError> {
    use rubato::{FftFixedInOut, Resampler};
    let src_rate = 48000usize;
    let dst_rate = 16000usize;
    if src_rate == dst_rate {
        return Ok(samples.to_vec());
    }
    let chunk_size = 1024;
    let mut resampler = FftFixedInOut::<f64>::new(src_rate, dst_rate, chunk_size)
        .map_err(|e| SenseVoiceError::InvalidAudio(format!("Resampler init failed: {}", e)))?;
    let input_f64: Vec<f64> = samples.iter().map(|&s| s as f64).collect();
    let mut output = Vec::new();
    let chunks = resampler.input_frames_next();
    for chunk in input_f64.chunks(chunks) {
        let padded: Vec<f64> = if chunk.len() < chunks {
            let mut v = chunk.to_vec();
            v.resize(chunks, 0.0);
            v
        } else {
            chunk.to_vec()
        };
        let out = resampler.process(&[&padded], None)
            .map_err(|e| SenseVoiceError::InvalidAudio(format!("Resample failed: {}", e)))?;
        output.extend(out[0].iter().map(|&v| v as f32));
    }
    Ok(output)
}

fn extract_fbank(waveform: &[f32]) -> Result<Array2<f32>, SenseVoiceError> {
    let scaled: Vec<f32> = waveform.iter().map(|&s| s * 32768.0).collect();
    let frame_length_ms = 25.0f64;
    let frame_shift_ms = 10.0f64;
    let sample_rate = 16000.0f64;
    let frame_length = (sample_rate * frame_length_ms / 1000.0) as usize;
    let frame_shift = (sample_rate * frame_shift_ms / 1000.0) as usize;
    let n_mels = 80;
    let n_fft = frame_length;
    let mut features = Vec::new();
    let num_frames = if scaled.len() >= frame_length {
        (scaled.len() - frame_length) / frame_shift + 1
    } else {
        0
    };
    for i in 0..num_frames {
        let start = i * frame_shift;
        let frame = &scaled[start..start + frame_length];
        let windowed: Vec<f64> = frame.iter().enumerate().map(|(j, &s)| {
            let hamming = 0.54 - 0.46 * (2.0 * std::f64::consts::PI * j as f64 / (frame_length as f64 - 1.0)).cos();
            s as f64 * hamming
        }).collect();
        let mut spectrum = vec![0.0f64; n_fft / 2 + 1];
        for k in 0..=n_fft / 2 {
            let mut re = 0.0f64;
            let mut im = 0.0f64;
            for n in 0..frame_length {
                let angle = -2.0 * std::f64::consts::PI * k as f64 * n as f64 / n_fft as f64;
                re += windowed[n] * angle.cos();
                im += windowed[n] * angle.sin();
            }
            spectrum[k] = re * re + im * im;
        }
        let mel_filters = make_mel_filters(n_mels, n_fft, sample_rate as f64);
        let mut mel_energies = vec![0.0f64; n_mels];
        for (m, filter) in mel_filters.iter().enumerate() {
            for (k, &w) in filter.iter().enumerate() {
                if k < spectrum.len() {
                    mel_energies[m] += spectrum[k] * w;
                }
            }
            mel_energies[m] = mel_energies[m].max(1e-10).ln();
        }
        let dct_result = apply_dct(&mel_energies, 13);
        let fbank_feat: Vec<f32> = dct_result[..n_mels].iter().map(|&v| v as f32).collect();
        features.push(fbank_feat);
    }
    if features.is_empty() {
        return Ok(Array2::from_shape_vec((0, n_mels), vec![]).unwrap_or(Array2::default((0, n_mels))));
    }
    let rows = features.len();
    let cols = features[0].len();
    let flat: Vec<f32> = features.into_iter().flatten().collect();
    Ok(Array2::from_shape_vec((rows, cols), flat).unwrap_or(Array2::default((rows, cols))))
}

fn make_mel_filters(n_mels: usize, n_fft: usize, sample_rate: f64) -> Vec<Vec<f64>> {
    let low_freq = 0.0f64;
    let high_freq = sample_rate / 2.0;
    let low_mel = hz_to_mel(low_freq);
    let high_mel = hz_to_mel(high_freq);
    let mel_points: Vec<f64> = (0..=n_mels + 1)
        .map(|i| low_mel + (high_mel - low_mel) * i as f64 / (n_mels + 1) as f64)
        .collect();
    let hz_points: Vec<f64> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();
    let bin_points: Vec<f64> = hz_points.iter().map(|&f| (n_fft as f64 + 1) * f / sample_rate).collect();
    let n_bins = n_fft / 2 + 1;
    let mut filters = Vec::with_capacity(n_mels);
    for i in 0..n_mels {
        let mut filter = vec![0.0f64; n_bins];
        let left = bin_points[i];
        let center = bin_points[i + 1];
        let right = bin_points[i + 2];
        for k in 0..n_bins {
            let k_f = k as f64;
            if k_f >= left && k_f < center && center > left {
                filter[k] = (k_f - left) / (center - left);
            } else if k_f >= center && k_f <= right && right > center {
                filter[k] = (right - k_f) / (right - center);
            }
        }
        filters.push(filter);
    }
    filters
}

fn hz_to_mel(hz: f64) -> f64 {
    2595.0 * (1.0 + hz / 700.0).ln() / std::f64::consts::LN_10
}

fn mel_to_hz(mel: f64) -> f64 {
    700.0 * (10.0_f64.powf(mel / 2595.0) - 1.0)
}

fn apply_dct(input: &[f64], n_out: usize) -> Vec<f64> {
    let n = input.len();
    let mut output = Vec::with_capacity(n_out);
    for k in 0..n_out.min(n) {
        let mut sum = 0.0f64;
        for (n_idx, &val) in input.iter().enumerate() {
            sum += val * (std::f64::consts::PI * (n_idx as f64 + 0.5) * k as f64 / n as f64).cos();
        }
        output.push(sum);
    }
    output
}

fn apply_lfr(feat: &Array2<f32>) -> Array2<f32> {
    let (t, dim) = feat.dim();
    let lfr_m = 7usize;
    let lfr_n = 6usize;
    let left_pad = (lfr_m - 1) / 2;
    let mut padded_rows: Vec<Vec<f32>> = Vec::with_capacity(t + left_pad);
    if t > 0 {
        let first = feat.row(0).to_vec();
        for _ in 0..left_pad {
            padded_rows.push(first.clone());
        }
        for i in 0..t {
            padded_rows.push(feat.row(i).to_vec());
        }
    }
    let padded_t = padded_rows.len();
    let t_lfr = if padded_t >= lfr_m {
        (padded_t - lfr_m) / lfr_n + 1
    } else {
        0
    };
    let lfr_dim = dim * lfr_m;
    let mut result = vec![0.0f32; t_lfr * lfr_dim];
    for i in 0..t_lfr {
        let start = i * lfr_n;
        for j in 0..lfr_m {
            let src_idx = start + j;
            if src_idx < padded_t {
                for d in 0..dim {
                    result[i * lfr_dim + j * dim + d] = padded_rows[src_idx][d];
                }
            }
        }
    }
    Array2::from_shape_vec((t_lfr, lfr_dim), result).unwrap_or(Array2::default((0, lfr_dim)))
}

fn apply_cmvn(feat: &Array2<f32>, means: &Array1<f64>, vars: &Array1<f64>) -> Array3<f32> {
    let (t, dim) = feat.dim();
    let feat_3d = feat.insert_axis(ndarray::Axis(0));
    let mut result = feat_3d.clone();
    for i in 0..t {
        for j in 0..dim.min(means.len()).min(vars.len()) {
            result[[0, i, j]] = ((feat_3d[[0, i, j]] as f64 + means[j]) * vars[j]) as f32;
        }
    }
    result
}

fn pad_features(feat: &Array3<f32>) -> (Array3<f32>, i32) {
    let (_, t_lfr, _) = feat.dim();
    (feat.clone(), t_lfr as i32)
}

fn decode_tokens(token_ids: &[i32], model_dir: &PathBuf) -> Result<String, SenseVoiceError> {
    let bpe_path = model_dir.join("chn_jpn_yue_eng_ko_spectok.bpe.model");
    if !bpe_path.exists() {
        return Err(SenseVoiceError::LoadFailed("BPE model not found".into()));
    }
    let sp = sentencepiece::SentencePieceProcessor::open(bpe_path.to_str().unwrap_or(""))
        .map_err(|e| SenseVoiceError::LoadFailed(format!("Failed to load BPE model: {}", e)))?;
    let ids: Vec<u32> = token_ids.iter().filter_map(|&id| if id > 0 { Some(id as u32) } else { None }).collect();
    let text = sp.decode(&ids)
        .map_err(|e| SenseVoiceError::InferenceFailed(format!("BPE decode failed: {}", e)))?;
    Ok(text)
}

fn postprocess(text: &str) -> String {
    let re = regex::Regex::new(r"<\|[^|]*\|>").unwrap();
    re.replace_all(text, "").trim().to_string()
}

#[tauri::command]
pub async fn get_sensevoice_status(
    app_handle: tauri::AppHandle,
) -> Result<SenseVoiceModelStatus, String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(model_status(&app_data_dir))
}

#[tauri::command]
pub async fn download_sensevoice_model(
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = model_dir(&app_data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let client = reqwest::Client::new();
    for &(filename, _) in MODEL_FILES {
        let file_path = dir.join(filename);
        if file_path.exists() {
            continue;
        }
        let url = format!("https://huggingface.co/{}/resolve/main/{}", HF_REPO, filename);
        let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
        let total = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;
        let mut file = std::fs::File::create(&file_path).map_err(|e| e.to_string())?;
        use tokio::io::AsyncWriteExt;
        let mut async_file = tokio::fs::File::from_std(file);
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            async_file.write_all(&chunk).await.map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;
            let percent = if total > 0 { (downloaded as f64 / total as f64) * 100.0 } else { 0.0 };
            let _ = app_handle.emit("sensevoice:download-progress", serde_json::json!({
                "file": filename,
                "downloaded": downloaded,
                "total": total,
                "percent": percent,
            }));
        }
        async_file.flush().await.map_err(|e| e.to_string())?;
    }
    let _ = app_handle.emit("sensevoice:download-complete", serde_json::json!({}));
    Ok(())
}

#[tauri::command]
pub async fn delete_sensevoice_model(
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = model_dir(&app_data_dir);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

- [ ] **Step 2: 在 `Cargo.toml` 中添加额外依赖**

在 `[dependencies]` 中补充：

```toml
sentencepiece = "0.13"
futures-util = "0.3"
regex = "1"
```

- [ ] **Step 3: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译成功。注意 `postprocess` 函数使用了 `regex::Regex::new`，需确保 `regex` 在依赖中。`extract_fbank` 的 FBank 实现是纯 Rust 手写的简化版，先用这个通过编译，后续用 `kaldi-native-fbank` 替换。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/sensevoice.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat: add SenseVoice engine with ONNX inference pipeline"
```

---

### Task 4: 集成 SenseVoice 到 lib.rs 转写调度

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: 在 `lib.rs` 顶部添加模块声明**

在 `mod ai;` 之后添加：

```rust
mod sensevoice;
```

- [ ] **Step 2: 添加 SenseVoice 全局 State 结构体**

在 `lib.rs` 的 `struct ShortcutIds` 之前，添加：

```rust
use std::sync::OnceCell;
use sensevoice::{SenseVoiceEngine, SenseVoiceError};

struct SenseVoiceState {
    engine: Arc<Mutex<Option<SenseVoiceEngine>>>,
    language: Arc<Mutex<i32>>,
}
```

- [ ] **Step 3: 修改转写调度逻辑**

在 `lib.rs` 的 `stop_recording` 函数中，找到 `tauri::async_runtime::spawn` 块内 `match ai::send_audio_prompt` 的调用。将整个 spawn 块中的 AI 请求逻辑替换为根据 provider_type 分派：

找到以下代码段（约在 `lib.rs` 行 148-173）：

```rust
                        let prompt = "请将这段音频转录为文字，只输出转录结果，不要添加任何解释。";
                        let logger = h.state::<Logger>();
                        match ai::send_audio_prompt(&logger, &audio_path, prompt, &model_name, &provider).await {
```

替换为：

```rust
                        let logger = h.state::<Logger>();
                        let text_result = if provider.provider_type == "sensevoice" {
                            let sv_state = h.state::<SenseVoiceState>();
                            let lang = *sv_state.language.lock().unwrap_or_else(|e| e.into_inner());
                            let app_data_dir = h.path().app_data_dir().unwrap_or_default();
                            let model_dir = app_data_dir.join("models").join("sensevoice");
                            let engine = {
                                let guard = sv_state.engine.lock().unwrap_or_else(|e| e.into_inner());
                                guard.as_ref().map(|_| ())
                            };
                            if engine.is_none() {
                                drop(engine);
                                match SenseVoiceEngine::new(&model_dir) {
                                    Ok(e) => {
                                        let mut guard = sv_state.engine.lock().unwrap_or_else(|e| e.into_inner());
                                        *guard = Some(e);
                                        logger.info("sensevoice", "SenseVoice 引擎加载完成", None);
                                    }
                                    Err(e) => {
                                        logger.error("sensevoice", "SenseVoice 引擎加载失败", Some(serde_json::json!({ "error": e.to_string() })));
                                        Err(e.to_string())
                                    }
                                }
                            }
                            let guard = sv_state.engine.lock().unwrap_or_else(|e| e.into_inner());
                            match guard.as_ref() {
                                Some(engine) => engine.transcribe(&audio_path, lang).map_err(|e| e.to_string()),
                                None => Err("SenseVoice 引擎未初始化".to_string()),
                            }
                        } else {
                            let prompt = "请将这段音频转录为文字，只输出转录结果，不要添加任何解释。";
                            ai::send_audio_prompt(&logger, &audio_path, prompt, &model_name, &provider).await.map_err(|e| e.to_string())
                        };
                        match text_result {
```

同时把后续的 `Ok(text) =>` 和 `Err(e) =>` 改为直接引用 `text_result` 的解构。找到原来的：

```rust
                            Ok(text) => {
                                logger.info("ai", "AI 转写成功",
```

和：

```rust
                            Err(e) => {
                                logger.error("ai", "AI 转写失败",
```

将 `text_result` 的 match 展开，确保编译通过。

- [ ] **Step 4: 在 `run()` 函数的 `setup` 中注册 SenseVoiceState**

在 `lib.rs` 的 `app.manage(logger);` 之前添加：

```rust
            let sensevoice_state = SenseVoiceState {
                engine: Arc::new(Mutex::new(None)),
                language: Arc::new(Mutex::new(0)),
            };
            app.manage(sensevoice_state);
```

- [ ] **Step 5: 在 `invoke_handler` 中注册新命令**

在 `lib.rs` 的 `tauri::generate_handler!` 宏中，`get_vertex_env_info,` 之后添加：

```rust
            sensevoice::get_sensevoice_status,
            sensevoice::download_sensevoice_model,
            sensevoice::delete_sensevoice_model,
```

- [ ] **Step 6: 修改 `test_model_connectivity` 命令**

在 `lib.rs` 的 `test_model_connectivity` 函数中，找到 `let result = if provider.provider_type == "vertex" {`，在其之前添加：

```rust
    if provider.provider_type == "sensevoice" {
        return Ok(TestResult {
            status: "ok".to_string(),
            latency_ms: Some(0),
            message: "本地模型，无需连通性测试".to_string(),
        });
    }
```

- [ ] **Step 7: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译成功

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: integrate SenseVoice into transcription dispatch"
```

---

### Task 5: 前端 UI — SenseVoice Provider 卡片

**Files:**
- Modify: `src/routes/models/+page.svelte`
- Modify: `src/lib/stores/config.ts`

- [ ] **Step 1: 在 `config.ts` 中添加 SenseVoice 相关类型和辅助函数**

在 `config.ts` 末尾 `export const config = createConfigStore();` 之前添加：

```typescript
export interface SenseVoiceModelStatus {
  status: 'not_downloaded' | 'downloading' | 'ready' | 'error';
  file?: string;
  percent?: number;
  downloaded?: number;
  total?: number;
  size_bytes?: number;
  message?: string;
}

export const SENSEVOICE_LANGUAGES = [
  { value: 0, label: '自动检测' },
  { value: 3, label: '中文' },
  { value: 4, label: '英文' },
  { value: 11, label: '日文' },
];
```

- [ ] **Step 2: 在 models 页面添加 SenseVoice 相关状态和逻辑**

在 `+page.svelte` 的 `<script>` 中，在 `let vertexEnvInfo` 之后添加：

```typescript
  import { listen } from '@tauri-apps/api/event';
  import { SENSEVOICE_LANGUAGES } from '$lib/stores/config';

  let sensevoiceStatus = $state<{ status: string; size_bytes?: number } | null>(null);
  let sensevoiceDownloading = $state(false);
  let sensevoiceDownloadProgress = $state({ file: '', percent: 0, downloaded: 0, total: 0 });
  let sensevoiceLanguage = $state(0);

  async function loadSenseVoiceStatus() {
    try {
      sensevoiceStatus = await invoke<{ status: string; size_bytes?: number }>('get_sensevoice_status');
    } catch {
      sensevoiceStatus = null;
    }
  }

  async function downloadSenseVoice() {
    sensevoiceDownloading = true;
    try {
      await invoke('download_sensevoice_model');
      await loadSenseVoiceStatus();
    } catch (e) {
      console.error('Download failed:', e);
    } finally {
      sensevoiceDownloading = false;
    }
  }

  async function deleteSenseVoiceModel() {
    try {
      await invoke('delete_sensevoice_model');
      await loadSenseVoiceStatus();
    } catch (e) {
      console.error('Delete failed:', e);
    }
  }

  onMount(async () => {
    config.load();
    try {
      vertexEnvInfo = await invoke<{ project: string; location: string }>('get_vertex_env_info');
    } catch {
      vertexEnvInfo = null;
    }
    await loadSenseVoiceStatus();
    listen('sensevoice:download-progress', (event) => {
      sensevoiceDownloadProgress = event.payload as any;
    });
  });
```

注意：将原来的 `onMount` 内容合并到新的 `onMount` 中，不要重复定义。

- [ ] **Step 3: 修改 Provider 卡片渲染，为 SenseVoice 添加特殊 UI**

在 `{#each $config.ai.providers || [] as provider (provider.id)}` 循环内，找到 `{#if provider.type === 'vertex'}` 的条件块。在其 `{/if}` 之后、`<div>` 标签（Models 部分）之前，插入 SenseVoice 的条件渲染：

```svelte
          {#if provider.type === 'sensevoice'}
            <div class="mb-2.5">
              <label class="block text-[11px] text-foreground-alt mb-1">模型状态</label>
              <div class="text-[10px] bg-background rounded-md border border-border p-2 space-y-1">
                {#if sensevoiceStatus?.status === 'ready'}
                  <div class="flex items-center justify-between">
                    <span class="text-green-500">已就绪</span>
                    <span class="text-muted-foreground">{(sensevoiceStatus!.size_bytes! / 1024 / 1024).toFixed(0)} MB</span>
                  </div>
                  <button
                    class="text-xs text-red-400 hover:text-red-300 transition-colors"
                    onclick={deleteSenseVoiceModel}
                  >
                    删除模型
                  </button>
                {:else if sensevoiceDownloading}
                  <div>
                    <div class="text-muted-foreground mb-1">下载中: {sensevoiceDownloadProgress.file}</div>
                    <div class="w-full bg-border rounded-full h-1.5">
                      <div class="bg-accent-foreground h-1.5 rounded-full transition-all" style="width: {sensevoiceDownloadProgress.percent}%"></div>
                    </div>
                    <div class="text-muted-foreground mt-0.5">{sensevoiceDownloadProgress.percent.toFixed(1)}%</div>
                  </div>
                {:else}
                  <button
                    class="text-xs text-accent-foreground hover:underline"
                    onclick={downloadSenseVoice}
                  >
                    下载模型 (约 242 MB)
                  </button>
                {/if}
              </div>
            </div>
            <div class="mb-2.5">
              <label class="block text-[11px] text-foreground-alt mb-1">转写语言</label>
              <select
                class="flex h-7 w-full rounded-md border border-border-input bg-background px-2 py-1 text-xs ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20"
                bind:value={sensevoiceLanguage}
              >
                {#each SENSEVOICE_LANGUAGES as lang}
                  <option value={lang.value}>{lang.label}</option>
                {/each}
              </select>
            </div>
          {:else if provider.type === 'vertex'}
```

注意：需要把原来的 `{#if provider.type === 'vertex'}` 改为 `{:else if provider.type === 'vertex'}`，使其成为 SenseVoice 条件的 else-if 分支。原来的 `{#if needsApiKey(provider)}` 块也需相应调整为 `{:else}` 或 `{:else if needsApiKey(provider)}`。

- [ ] **Step 4: 验证前端编译**

Run: `npm run build`
Expected: 编译成功

- [ ] **Step 5: Commit**

```bash
git add src/routes/models/+page.svelte src/lib/stores/config.ts
git commit -m "feat: add SenseVoice UI with download progress and language selection"
```

---

### Task 6: 用 kaldi-native-fbank 替换手写 FBank

**Files:**
- Modify: `src-tauri/src/sensevoice.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 在 `Cargo.toml` 中添加 kaldi-native-fbank**

```toml
kaldi-native-fbank = "0.1"
```

- [ ] **Step 2: 替换 `extract_fbank` 函数及相关辅助函数**

将 `sensevoice.rs` 中的 `extract_fbank`、`make_mel_filters`、`hz_to_mel`、`mel_to_hz`、`apply_dct` 函数替换为使用 `kaldi-native-fbank` crate 的实现：

```rust
fn extract_fbank(waveform: &[f32]) -> Result<Array2<f32>, SenseVoiceError> {
    use kaldi_native_fbank::{FbankComputer, FbankOptions, FrameExtractionOptions, MelBanksOptions};
    let frame_opts = FrameExtractionOptions {
        samp_freq: 16000.0,
        frame_shift_ms: 10.0,
        frame_length_ms: 25.0,
        dither: 0.0,
        preemph_coeff: 0.0,
        remove_dc_offset: false,
        window_type: "hamming".to_string(),
        round_to_power_of_two: true,
        snip_edges: true,
        max_feature_vectors: 0,
        allow_downsample: false,
        allow_upsample: false,
        ..Default::default()
    };
    let mel_opts = MelBanksOptions {
        num_bins: 80,
        low_freq: 20.0,
        high_freq: -400.0,
        vtln_low: 100.0,
        vtln_high: -500.0,
        debug_mel: false,
        htk_mode: false,
    };
    let opts = FbankOptions {
        frame_opts,
        mel_opts,
        use_energy: false,
        energy_floor: 0.0,
        raw_energy: true,
        htk_compat: false,
        use_log_fbank: true,
        use_power_fbank: false,
    };
    let mut computer = FbankComputer::new(opts)
        .map_err(|e| SenseVoiceError::InferenceFailed(format!("FBank init failed: {}", e)))?;
    let samples_i16: Vec<i16> = waveform.iter().map(|&s| (s * 32768.0).clamp(-32768.0, 32767.0) as i16).collect();
    let samples_f32: Vec<f32> = samples_i16.iter().map(|&s| s as f32).collect();
    let features = computer.compute(&samples_f32)
        .map_err(|e| SenseVoiceError::InferenceFailed(format!("FBank compute failed: {}", e)))?;
    if features.is_empty() {
        return Ok(Array2::default((0, 80)));
    }
    let rows = features.len();
    let cols = features[0].len();
    let flat: Vec<f32> = features.into_iter().flatten().collect();
    Ok(Array2::from_shape_vec((rows, cols), flat).unwrap_or_else(|_| Array2::default((rows, cols))))
}
```

同时删除不再使用的函数：`make_mel_filters`、`hz_to_mel`、`mel_to_hz`、`apply_dct`。

- [ ] **Step 3: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译成功。如果 `kaldi-native-fbank` API 与上述不匹配，需根据其文档调整参数。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/sensevoice.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat: replace hand-rolled FBank with kaldi-native-fbank crate"
```

---

### Task 7: 端到端集成测试

**Files:**
- None (manual testing)

- [ ] **Step 1: 启动开发模式**

Run: `npm run tauri dev`
Expected: 应用正常启动

- [ ] **Step 2: 验证 Provider 列表**

打开模型管理页，确认 SenseVoice (本地) 出现在 Provider 列表中。

- [ ] **Step 3: 下载模型**

点击"下载模型"按钮，确认进度条正常显示，下载完成后状态变为"已就绪"。

- [ ] **Step 4: 选择 SenseVoice 为当前转写服务**

在 Transcription 选择器中选择 SenseVoice / SenseVoice-Small。

- [ ] **Step 5: 测试转写**

使用录音快捷键录制一段语音（如中文"你好世界"），松开后确认：
- 托盘图标正常切换
- 转写结果写入剪贴板
- 自动粘贴生效

- [ ] **Step 6: 测试其他 provider 不受影响**

切换回 Vertex AI 或阿里云 provider，确认原有转写功能正常。

- [ ] **Step 7: Commit（如有修复）**

```bash
git add -A
git commit -m "fix: address integration issues found during testing"
```
