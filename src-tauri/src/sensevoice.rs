use ndarray::{Array1, Array2, Array3};
use ort::session::Session;
use std::path::PathBuf;
use tauri::{Emitter, Manager};

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
            .map_err(|e| SenseVoiceError::LoadFailed(e.to_string()))?
            .with_intra_threads(4)
            .map_err(|e| SenseVoiceError::LoadFailed(e.to_string()))?
            .commit_from_file(&onnx_path)
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

    pub fn transcribe(&mut self, wav_path: &PathBuf, language: i32) -> Result<String, SenseVoiceError> {
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
        &mut self,
        feats: &Array3<f32>,
        feats_len: i32,
        language: i32,
    ) -> Result<Vec<i32>, SenseVoiceError> {
        let (_, t_lfr, _) = feats.dim();
        let feats_len_arr = Array1::from_vec(vec![feats_len]);
        let language_arr = Array1::from_vec(vec![language]);
        let textnorm_arr = Array1::from_vec(vec![14i32]);

        let feats_input = ort::value::TensorRef::from_array_view(feats.view())
            .map_err(|e| SenseVoiceError::InferenceFailed(e.to_string()))?;
        let feats_len_input = ort::value::TensorRef::from_array_view(feats_len_arr.view())
            .map_err(|e| SenseVoiceError::InferenceFailed(e.to_string()))?;
        let language_input = ort::value::TensorRef::from_array_view(language_arr.view())
            .map_err(|e| SenseVoiceError::InferenceFailed(e.to_string()))?;
        let textnorm_input = ort::value::TensorRef::from_array_view(textnorm_arr.view())
            .map_err(|e| SenseVoiceError::InferenceFailed(e.to_string()))?;

        let outputs = self.session.run(ort::inputs![
            feats_input,
            feats_len_input,
            language_input,
            textnorm_input,
        ]).map_err(|e| SenseVoiceError::InferenceFailed(e.to_string()))?;

        let (logits_shape, logits_data) = outputs[0].try_extract_tensor::<f32>()
            .map_err(|e| SenseVoiceError::InferenceFailed(e.to_string()))?;
        let vocab_size = logits_shape[2] as usize;
        let t_lfr_out = logits_shape[1] as usize;
        let mut token_ids = Vec::new();
        for t in 0..t_lfr_out {
            let start = t * vocab_size;
            let row = &logits_data[start..start + vocab_size];
            if let Some((idx, _)) = row.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)) {
                if idx != 0 && token_ids.last().copied() != Some(idx as i32) {
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
    let mut resampler = FftFixedInOut::<f64>::new(src_rate, dst_rate, chunk_size, 1)
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
    use kaldi_native_fbank::{FbankComputer, FbankOptions, OnlineFeature};
    use kaldi_native_fbank::online::FeatureComputer;

    let mut opts = FbankOptions::default();
    opts.frame_opts.samp_freq = 16000.0;
    opts.frame_opts.frame_shift_ms = 10.0;
    opts.frame_opts.frame_length_ms = 25.0;
    opts.frame_opts.dither = 0.0;
    opts.frame_opts.preemph_coeff = 0.0;
    opts.frame_opts.remove_dc_offset = false;
    opts.frame_opts.window_type = "hamming".to_string();
    opts.mel_opts.num_bins = 80;
    opts.mel_opts.low_freq = 20.0;
    opts.mel_opts.high_freq = 0.0;
    opts.use_energy = false;
    opts.use_log_fbank = true;
    opts.use_power = true;

    let computer = FbankComputer::new(opts.clone())
        .map_err(|e| SenseVoiceError::InferenceFailed(format!("FbankComputer init failed: {}", e)))?;
    let mut online = OnlineFeature::new(FeatureComputer::Fbank(computer));

    let scaled: Vec<f32> = waveform.iter().map(|&s| s * 32768.0).collect();
    online.accept_waveform(16000.0, &scaled);
    online.input_finished();

    let num_frames = online.num_frames_ready();
    let num_bins = opts.mel_opts.num_bins;
    if num_frames == 0 {
        return Ok(Array2::from_shape_vec((0, num_bins), vec![]).unwrap_or(Array2::default((0, num_bins))));
    }

    let mut flat = Vec::with_capacity(num_frames * num_bins);
    for i in 0..num_frames {
        if let Some(frame) = online.get_frame(i) {
            flat.extend_from_slice(frame);
        }
    }
    Ok(Array2::from_shape_vec((num_frames, num_bins), flat).unwrap_or(Array2::default((num_frames, num_bins))))
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
    let feat_3d = feat.clone().insert_axis(ndarray::Axis(0));
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
    let text = sp.decode_piece_ids(&ids)
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
