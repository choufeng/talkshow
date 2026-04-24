use ndarray::{Array1, Array2, Array3};
use ort::session::Session;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

use super::SenseVoiceError;
use super::bundled_paths;
#[cfg(target_os = "macos")]
use ort_sys;

/// Loads `libonnxruntime.dylib` via `RTLD_LAZY | RTLD_GLOBAL` and manually
/// initialises the ORT API using `ort::set_api`.
///
/// This bypasses ort's `G_ORT_LIB` / `std::sync::Once` loading path, which
/// deadlocks on macOS when called from within a Tauri process (the `Once`
/// ends up spinning forever — root cause appears to be an interaction between
/// libloading's dlopen call and macOS's dyld lock in this runtime context).
///
/// By using raw `libc::dlopen` + `dlsym` we load the library successfully,
/// then hand the resulting `OrtApi` pointer directly to `ort::set_api` which
/// stores it in `G_ORT_API` without touching `G_ORT_LIB` at all.
#[cfg(target_os = "macos")]
unsafe fn load_ort_and_set_api(path: &std::path::Path) -> Result<(), SenseVoiceError> {
    use std::ffi::CString;

    let path_cstr = CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|e| SenseVoiceError::LoadFailed(format!("bad dylib path: {e}")))?;

    eprintln!("[ort-init] dlopen RTLD_LAZY | RTLD_GLOBAL");
    let handle = libc::dlopen(path_cstr.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL);
    if handle.is_null() {
        let err = std::ffi::CStr::from_ptr(libc::dlerror())
            .to_string_lossy()
            .into_owned();
        return Err(SenseVoiceError::LoadFailed(format!("dlopen failed: {err}")));
    }
    eprintln!("[ort-init] dlopen OK");

    // OrtGetApiBase
    let sym_name = b"OrtGetApiBase\0";
    let sym = libc::dlsym(handle, sym_name.as_ptr() as *const libc::c_char);
    if sym.is_null() {
        return Err(SenseVoiceError::LoadFailed(
            "OrtGetApiBase not found".into(),
        ));
    }
    eprintln!("[ort-init] OrtGetApiBase symbol OK");

    type OrtGetApiBaseFn = unsafe extern "C" fn() -> *const ort_sys::OrtApiBase;
    let get_api_base: OrtGetApiBaseFn = std::mem::transmute(sym);

    let api_base = get_api_base();
    if api_base.is_null() {
        return Err(SenseVoiceError::LoadFailed("OrtApiBase is null".into()));
    }
    eprintln!("[ort-init] OrtApiBase OK");

    let api_ptr = ((*api_base).GetApi)(ort_sys::ORT_API_VERSION);
    if api_ptr.is_null() {
        return Err(SenseVoiceError::LoadFailed(format!(
            "OrtApi v{} is null — dylib version mismatch?",
            ort_sys::ORT_API_VERSION
        )));
    }
    eprintln!("[ort-init] OrtApi OK, calling ort::set_api");

    // Copy the OrtApi struct and hand it to ort.  The library stays loaded
    // because we never call dlclose on the handle above.
    let api: ort_sys::OrtApi = std::ptr::read(api_ptr);
    ort::set_api(api);
    eprintln!("[ort-init] ort::set_api done");
    Ok(())
}

static ORT_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Public entry point for pre-initialising ORT on the main thread at app
/// startup.  Calling this early avoids a macOS dyld/CoreML deadlock that
/// occurs when `dlopen(libonnxruntime.dylib)` is first called from a
/// `spawn_blocking` background thread.
pub fn ensure_ort_initialized_pub(app: &AppHandle) -> Result<(), SenseVoiceError> {
    ensure_ort_initialized(app)
}

// Safety: concurrent calls to ensure_ort_initialized are prevented by the
// Mutex<Option<SenseVoiceEngine>> lock held at the call site in pipeline.rs.
// The AtomicBool is used only to skip re-initialization on subsequent calls
// after the engine has already been loaded once.
fn ensure_ort_initialized(app: &AppHandle) -> Result<(), SenseVoiceError> {
    eprintln!("[ort-init] checking ORT_INITIALIZED flag");
    if ORT_INITIALIZED.load(Ordering::Acquire) {
        eprintln!("[ort-init] already initialized, skipping");
        return Ok(());
    }
    eprintln!("[ort-init] resolving dylib path");
    let dylib_path = bundled_paths::onnxruntime_dylib_path(app).ok_or_else(|| {
        SenseVoiceError::LoadFailed(
            "ONNX Runtime library not found in app bundle. Please reinstall the application."
                .to_string(),
        )
    })?;
    eprintln!("[ort-init] dylib path = {:?}", dylib_path);
    // On macOS, ort's normal load path (ort::init_from → libloading → G_ORT_LIB
    // Once) deadlocks inside Tauri processes.  We bypass it entirely by loading
    // the dylib ourselves and injecting the OrtApi via ort::set_api.
    #[cfg(target_os = "macos")]
    unsafe {
        load_ort_and_set_api(&dylib_path)?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("[ort-init] calling ort::init_from (dlopen)");
        let builder = ort::init_from(&dylib_path).map_err(|e| {
            SenseVoiceError::LoadFailed(format!(
                "Failed to load ONNX Runtime from {}: {}",
                dylib_path.display(),
                e
            ))
        })?;
        eprintln!("[ort-init] dlopen done, calling commit()");
        builder.commit();
        eprintln!("[ort-init] commit() done");
    }
    ORT_INITIALIZED.store(true, Ordering::Release);
    Ok(())
}

pub struct SenseVoiceEngine {
    session: Session,
    cmvn_means: Array1<f64>,
    cmvn_vars: Array1<f64>,
    model_dir: PathBuf,
    ffmpeg_path: PathBuf,
}

impl SenseVoiceEngine {
    pub fn new(model_dir: &std::path::Path, app: &AppHandle) -> Result<Self, SenseVoiceError> {
        eprintln!("[sensevoice] step 1: ensure_ort_initialized");
        ensure_ort_initialized(app)?;
        eprintln!("[sensevoice] step 1: done");

        let onnx_path = model_dir.join("model_quant.onnx");
        eprintln!("[sensevoice] step 2: Session::builder()");
        let builder = Session::builder().map_err(|e| SenseVoiceError::LoadFailed(e.to_string()))?;
        eprintln!("[sensevoice] step 2: with_intra_threads");
        let mut builder = builder
            .with_intra_threads(4)
            .map_err(|e| SenseVoiceError::LoadFailed(e.to_string()))?;
        eprintln!(
            "[sensevoice] step 2: commit_from_file (loading {})",
            onnx_path.display()
        );
        let session = builder
            .commit_from_file(&onnx_path)
            .map_err(|e| SenseVoiceError::LoadFailed(e.to_string()))?;
        eprintln!("[sensevoice] step 2: done");

        eprintln!("[sensevoice] step 3: parse_cmvn");
        let (cmvn_means, cmvn_vars) =
            parse_cmvn(&model_dir.join("am.mvn")).map_err(SenseVoiceError::LoadFailed)?;
        eprintln!("[sensevoice] step 3: done");

        eprintln!("[sensevoice] step 4: ffmpeg_bin_path");
        let ffmpeg_path = bundled_paths::ffmpeg_bin_path(app).ok_or_else(|| {
            SenseVoiceError::LoadFailed(
                "ffmpeg not found in app bundle. Please reinstall the application.".to_string(),
            )
        })?;
        eprintln!("[sensevoice] step 4: done → {:?}", ffmpeg_path);

        Ok(Self {
            session,
            cmvn_means,
            cmvn_vars,
            model_dir: model_dir.to_path_buf(),
            ffmpeg_path,
        })
    }

    pub fn transcribe(
        &mut self,
        audio_path: &PathBuf,
        language: i32,
    ) -> Result<String, SenseVoiceError> {
        let (waveform, sample_rate) = load_audio(audio_path, &self.ffmpeg_path)?;
        let waveform_16k = resample_to_16k(&waveform, sample_rate)?;
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
        let (_, _t_lfr, _) = feats.dim();
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

        let outputs = self
            .session
            .run(ort::inputs![
                feats_input,
                feats_len_input,
                language_input,
                textnorm_input,
            ])
            .map_err(|e| SenseVoiceError::InferenceFailed(e.to_string()))?;

        let (logits_shape, logits_data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| SenseVoiceError::InferenceFailed(e.to_string()))?;
        let vocab_size = logits_shape[2] as usize;
        let t_lfr_out = logits_shape[1] as usize;
        let mut token_ids = Vec::new();
        for t in 0..t_lfr_out {
            let start = t * vocab_size;
            let row = &logits_data[start..start + vocab_size];
            if let Some((idx, _)) = row
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                && idx != 0
                && token_ids.last().copied() != Some(idx as i32)
            {
                token_ids.push(idx as i32);
            }
        }
        Ok(token_ids)
    }
}

fn parse_cmvn(mvn_path: &PathBuf) -> Result<(Array1<f64>, Array1<f64>), String> {
    let content =
        std::fs::read_to_string(mvn_path).map_err(|e| format!("Failed to read am.mvn: {}", e))?;
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

fn load_audio(path: &PathBuf, ffmpeg_path: &PathBuf) -> Result<(Vec<f32>, u32), SenseVoiceError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let (wav_path, converted) = if ext == "wav" {
        (path.clone(), false)
    } else {
        let tmp_dir = std::env::temp_dir().join("talkshow");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let tmp_wav = tmp_dir.join(format!(
            "{}_sensevoice.wav",
            path.file_stem().and_then(|s| s.to_str()).unwrap_or("tmp")
        ));
        let output = std::process::Command::new(ffmpeg_path)
            .args(["-y", "-i"])
            .arg(path)
            .args(["-ar", "16000", "-ac", "1", "-f", "wav"])
            .arg(&tmp_wav)
            .output()
            .map_err(|e| SenseVoiceError::InvalidAudio(format!("ffmpeg failed: {}", e)))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SenseVoiceError::InvalidAudio(format!(
                "ffmpeg conversion failed: {}",
                stderr
            )));
        }
        (tmp_wav, true)
    };
    let mut reader = hound::WavReader::open(&wav_path)
        .map_err(|e| SenseVoiceError::InvalidAudio(e.to_string()))?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = 2i32.pow(spec.bits_per_sample as u32 - 1) as f32;
            reader
                .samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max_val)
                .collect()
        }
        hound::SampleFormat::Float => reader.samples::<f32>().filter_map(|s| s.ok()).collect(),
    };
    if converted {
        let _ = std::fs::remove_file(&wav_path);
    }
    Ok((samples, sample_rate))
}

fn resample_to_16k(samples: &[f32], src_rate: u32) -> Result<Vec<f32>, SenseVoiceError> {
    use rubato::{FftFixedInOut, Resampler};
    let src_rate = src_rate as usize;
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
        let out = resampler
            .process(&[&padded], None)
            .map_err(|e| SenseVoiceError::InvalidAudio(format!("Resample failed: {}", e)))?;
        output.extend(out[0].iter().map(|&v| v as f32));
    }
    Ok(output)
}

fn extract_fbank(waveform: &[f32]) -> Result<Array2<f32>, SenseVoiceError> {
    use kaldi_native_fbank::online::FeatureComputer;
    use kaldi_native_fbank::{FbankComputer, FbankOptions, OnlineFeature};

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

    let computer = FbankComputer::new(opts.clone()).map_err(|e| {
        SenseVoiceError::InferenceFailed(format!("FbankComputer init failed: {}", e))
    })?;
    let mut online = OnlineFeature::new(FeatureComputer::Fbank(computer));

    let scaled: Vec<f32> = waveform.iter().map(|&s| s * 32768.0).collect();
    online.accept_waveform(16000.0, &scaled);
    online.input_finished();

    let num_frames = online.num_frames_ready();
    let num_bins = opts.mel_opts.num_bins;
    if num_frames == 0 {
        return Ok(
            Array2::from_shape_vec((0, num_bins), vec![]).unwrap_or(Array2::default((0, num_bins)))
        );
    }

    let mut flat = Vec::with_capacity(num_frames * num_bins);
    for i in 0..num_frames {
        if let Some(frame) = online.get_frame(i) {
            flat.extend_from_slice(frame);
        }
    }
    Ok(Array2::from_shape_vec((num_frames, num_bins), flat)
        .unwrap_or(Array2::default((num_frames, num_bins))))
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

fn decode_tokens(
    token_ids: &[i32],
    model_dir: &std::path::Path,
) -> Result<String, SenseVoiceError> {
    let bpe_path = model_dir.join("chn_jpn_yue_eng_ko_spectok.bpe.model");
    if !bpe_path.exists() {
        return Err(SenseVoiceError::LoadFailed("BPE model not found".into()));
    }
    let sp = sentencepiece::SentencePieceProcessor::open(bpe_path.to_str().unwrap_or(""))
        .map_err(|e| SenseVoiceError::LoadFailed(format!("Failed to load BPE model: {}", e)))?;
    let ids: Vec<u32> = token_ids
        .iter()
        .filter_map(|&id| if id > 0 { Some(id as u32) } else { None })
        .collect();
    let text = sp
        .decode_piece_ids(&ids)
        .map_err(|e| SenseVoiceError::InferenceFailed(format!("BPE decode failed: {}", e)))?;
    Ok(text)
}

fn postprocess(text: &str) -> String {
    let re = regex::Regex::new(r"<\|[^|]*\|>").unwrap();
    re.replace_all(text, "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_cmvn_valid_format() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "<AddShift>").unwrap();
        writeln!(file, "[").unwrap();
        writeln!(file, "  1.0 2.0 3.0").unwrap();
        writeln!(file, "]").unwrap();
        writeln!(file, "<Rescale>").unwrap();
        writeln!(file, "[").unwrap();
        writeln!(file, "  0.5 1.0 1.5").unwrap();
        writeln!(file, "]").unwrap();

        let (means, vars) = parse_cmvn(&file.path().to_path_buf()).unwrap();
        assert_eq!(means.len(), 3);
        assert_eq!(means[0], 1.0);
        assert_eq!(means[1], 2.0);
        assert_eq!(means[2], 3.0);
        assert_eq!(vars.len(), 3);
        assert_eq!(vars[0], 0.5);
    }

    #[test]
    fn test_parse_cmvn_multiline_values() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "<AddShift>").unwrap();
        writeln!(file, "[").unwrap();
        writeln!(file, "  1.0").unwrap();
        writeln!(file, "  2.0").unwrap();
        writeln!(file, "  3.0").unwrap();
        writeln!(file, "]").unwrap();
        writeln!(file, "<Rescale>").unwrap();
        writeln!(file, "[").unwrap();
        writeln!(file, "  1.0").unwrap();
        writeln!(file, "  2.0").unwrap();
        writeln!(file, "  3.0").unwrap();
        writeln!(file, "]").unwrap();

        let (means, vars) = parse_cmvn(&file.path().to_path_buf()).unwrap();
        assert_eq!(means.len(), 3);
        assert_eq!(vars.len(), 3);
    }

    #[test]
    fn test_postprocess_removes_special_tokens() {
        assert_eq!(postprocess("<|nospeech|>"), "");
        assert_eq!(postprocess("<|zh|><|en|>Hello"), "Hello");
        assert_eq!(postprocess("Hello <|ja|> World"), "Hello  World");
        assert_eq!(postprocess("plain text"), "plain text");
    }

    #[test]
    fn test_postprocess_trims_whitespace() {
        assert_eq!(postprocess("  <|zh|>hello  "), "hello");
        assert_eq!(postprocess(""), "");
    }

    #[test]
    fn test_apply_lfr_basic() {
        let feat = Array2::from_shape_vec((10, 80), vec![1.0f32; 10 * 80]).unwrap();
        let result = apply_lfr(&feat);
        let (t_lfr, lfr_dim) = result.dim();
        assert_eq!(lfr_dim, 80 * 7);
        assert!(t_lfr > 0);
    }

    #[test]
    fn test_apply_lfr_empty_input() {
        let feat = Array2::from_shape_vec((0, 80), vec![]).unwrap();
        let result = apply_lfr(&feat);
        assert_eq!(result.dim(), (0, 80 * 7));
    }

    #[test]
    fn test_apply_cmvn_dimensions() {
        let feat = Array2::from_shape_vec((5, 3), vec![1.0f32; 15]).unwrap();
        let means = Array1::from_vec(vec![0.0f64; 3]);
        let vars = Array1::from_vec(vec![1.0f64; 3]);
        let result = apply_cmvn(&feat, &means, &vars);
        assert_eq!(result.dim(), (1, 5, 3));
    }

    #[test]
    fn test_apply_cmvn_applies_transform() {
        let feat = Array2::from_shape_vec((1, 2), vec![0.0f32, 0.0]).unwrap();
        let means = Array1::from_vec(vec![10.0f64, 20.0f64]);
        let vars = Array1::from_vec(vec![2.0f64, 0.5f64]);
        let result = apply_cmvn(&feat, &means, &vars);
        assert_eq!(result[[0, 0, 0]], (0.0 + 10.0) * 2.0);
        assert_eq!(result[[0, 0, 1]], (0.0 + 20.0) * 0.5);
    }

    #[test]
    fn test_pad_features_returns_correct_length() {
        let feat = Array3::from_shape_vec((1, 10, 560), vec![0.0f32; 10 * 560]).unwrap();
        let (padded, len) = pad_features(&feat);
        assert_eq!(len, 10);
        assert_eq!(padded.dim(), (1, 10, 560));
    }
}
