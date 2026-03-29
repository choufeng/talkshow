use ndarray::{Array1, Array2, Array3};
use ort::session::Session;
use std::path::PathBuf;
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
    model_dir: PathBuf,
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

        Ok(Self {
            session,
            cmvn_means,
            cmvn_vars,
            model_dir: model_dir.clone(),
        })
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
        let text = decode_tokens(&token_ids, &self.model_dir)?;
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
        let (_, _, _vocab_size) = logits.dim();
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
        let file = std::fs::File::create(&file_path).map_err(|e| e.to_string())?;
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
