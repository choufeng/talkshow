use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dasp_sample::Sample as DaspSample;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// 实时麦克风电平(最近一个 chunk 的 RMS,f32 bitcast),音频回调写、UI 定时器读,无锁
pub static MIC_LEVEL: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecordingResult {
    pub path: PathBuf,
    pub duration_secs: u64,
    pub format: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecordingCancelled {
    pub duration_secs: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum RecordingError {
    TooShort,
    MicrophoneUnavailable(String),
    AudioStreamError(String),
    FileError(String),
}

impl std::fmt::Display for RecordingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecordingError::TooShort => write!(f, "Recording too short, discarded"),
            RecordingError::MicrophoneUnavailable(e) => write!(f, "{}", e),
            RecordingError::AudioStreamError(e) => write!(f, "{}", e),
            RecordingError::FileError(e) => write!(f, "{}", e),
        }
    }
}

const RECORDINGS_DIR_NAME: &str = "talkshow";

/// 可用音频输入设备
#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

/// 枚举系统输入设备(cpal 默认 host),默认设备排在首位并带标记
pub fn list_input_devices() -> Vec<AudioDeviceInfo> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());
    let mut infos: Vec<AudioDeviceInfo> = host
        .input_devices()
        .map(|devices| {
            devices
                .filter_map(|d| d.name().ok())
                .map(|name| AudioDeviceInfo {
                    is_default: Some(&name) == default_name.as_ref(),
                    name,
                })
                .collect()
        })
        .unwrap_or_default();
    // 默认设备置顶,前端首项即“跟随系统”的落点
    infos.sort_by_key(|d| !d.is_default);
    infos.dedup_by(|a, b| a.name == b.name);
    infos
}

/// 设备名匹配:忽略首尾空白,大小写不敏感(系统名称大小写不稳定)
fn device_matches(name: &str, want: &str) -> bool {
    name.trim().eq_ignore_ascii_case(want.trim())
}

pub enum AudioRecorder {
    /// 未持流:不占用麦克风(懒建流,macOS 状态栏不亮麦图标)
    Idle {
        device_name: Option<String>,
        output_dir: Option<PathBuf>,
    },
    /// 录音中:持有激活的输入流,stop/cancel 后 drop 流释放设备
    Recording {
        buffer: Arc<Mutex<Vec<i16>>>,
        stream: cpal::Stream,
        sample_rate: u32,
        channels: u16,
        start_time: Instant,
        device_name: Option<String>,
        output_dir: Option<PathBuf>,
    },
}

/// 全局快捷键管线的录音器,managed state 供 set_audio_input_device 运行时重建
pub struct RecorderState {
    pub recorder: Arc<Mutex<AudioRecorder>>,
}

pub fn generate_filename() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    let total_secs = now.as_secs();
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = (total_secs / 3600) % 24;

    let days_since_epoch = total_secs / 86400;
    let (year, month, day) = days_to_date(days_since_epoch);

    format!(
        "talkshow_{:04}{:02}{:02}_{:02}{:02}{:02}.flac",
        year, month, day, hours, mins, secs
    )
}

fn days_to_date(days_since_epoch: u64) -> (u64, u64, u64) {
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

#[allow(unknown_lints, clippy::manual_is_multiple_of)]
fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub fn recordings_dir() -> PathBuf {
    std::env::temp_dir().join(RECORDINGS_DIR_NAME)
}

pub fn ensure_recordings_dir() -> Result<PathBuf, String> {
    let dir = recordings_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create recordings dir: {}", e))?;
    Ok(dir)
}

pub fn recordings_dir_in(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join(RECORDINGS_DIR_NAME)
}

pub fn ensure_recordings_dir_in(app_data_dir: &std::path::Path) -> Result<PathBuf, String> {
    let dir = recordings_dir_in(app_data_dir);
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

// SAFETY: cpal::Stream uses platform-specific audio backends (CoreAudio on macOS,
// WASAPI on Windows, ALSA/PulseAudio on Linux) that are thread-safe.
// AudioRecorder is always accessed through Arc<Mutex<>> in lib.rs,
// ensuring no concurrent access to the stream.
unsafe impl Send for AudioRecorder {}

fn sample_to_i16<T: cpal::Sample>(sample: T) -> i16
where
    T::Float: dasp_sample::Duplex<f32>,
{
    let f: f32 = sample.to_float_sample().to_sample();
    (f * 32767.0).clamp(-32768.0, 32767.0) as i16
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: Arc<Mutex<Vec<i16>>>,
) -> Result<cpal::Stream, String>
where
    T: cpal::Sample + cpal::SizedSample,
    T::Float: dasp_sample::Duplex<f32>,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let mut buf = buffer.lock().unwrap_or_else(|e| e.into_inner());
                let mut sum_sq = 0.0f64;
                for &sample in data {
                    let s = sample_to_i16(sample);
                    buf.push(s);
                    let f = f64::from(s) / 32768.0;
                    sum_sq += f * f;
                }
                if !data.is_empty() {
                    let rms = (sum_sq / data.len() as f64).sqrt() as f32;
                    MIC_LEVEL.store(rms.to_bits(), Ordering::Relaxed);
                }
            },
            |err| {
                eprintln!("Audio input error: {}", err);
            },
            None,
        )
        .map_err(|e| format!("Failed to create audio stream: {}", e))
}

impl AudioRecorder {
    /// 按设备名创建录音器。懒建流:只记住设备名,不打开麦克风;
    /// 真正建流在 start(),stop/cancel 后 drop 流释放设备,
    /// macOS 状态栏麦克风图标仅在录音期间显示。
    pub fn new_with_device(device_name: Option<&str>) -> Self {
        AudioRecorder::Idle {
            device_name: device_name
                .filter(|n| !n.trim().is_empty())
                .map(str::to_string),
            output_dir: None,
        }
    }

    pub fn start(&mut self) -> Result<(), RecordingError> {
        match self {
            AudioRecorder::Idle { .. } => self.open_and_play(),
            AudioRecorder::Recording { start_time, .. } => {
                // 已在录音:仅重置计时
                *start_time = Instant::now();
                Ok(())
            }
        }
    }

    /// 建流并播放。设备解析/建流/启动任一步失败即返回 Err,状态保持 Idle
    fn open_and_play(&mut self) -> Result<(), RecordingError> {
        let (device_name, output_dir) = match self {
            AudioRecorder::Idle {
                device_name,
                output_dir,
            } => (device_name.clone(), output_dir.clone()),
            AudioRecorder::Recording { .. } => return Ok(()),
        };

        let host = cpal::default_host();
        let device = match device_name
            .as_deref()
            .filter(|n| !n.trim().is_empty())
            .and_then(|want| {
                host.input_devices().ok().and_then(|mut ds| {
                    ds.find(|d| d.name().map(|n| device_matches(&n, want)).unwrap_or(false))
                })
            }) {
            Some(d) => d,
            None => match host.default_input_device() {
                Some(d) => {
                    if let Some(ref want) = device_name {
                        if !want.trim().is_empty() {
                            eprintln!(
                                "[TalkShow] Audio device '{}' not found, falling back to default",
                                want
                            );
                        }
                    }
                    d
                }
                None => {
                    return Err(RecordingError::MicrophoneUnavailable(
                        "No microphone device available".into(),
                    ));
                }
            },
        };

        let mut supported_configs = device.supported_input_configs().map_err(|e| {
            RecordingError::MicrophoneUnavailable(format!(
                "Failed to query microphone configs: {}",
                e
            ))
        })?;

        let supported_config =
            supported_configs
                .next()
                .ok_or(RecordingError::MicrophoneUnavailable(
                    "Microphone has no supported input configurations".into(),
                ))?;

        // 目标 16kHz：SenseVoice 原生采样率，转写零重采样；FLAC 体积 ~1/6。
        // 设备不支持 16k 时取最近支持值，转写端 resample_to_16k 兜底。
        let min_rate = supported_config.min_sample_rate().0;
        let max_rate = supported_config.max_sample_rate().0;
        let config = cpal::StreamConfig {
            channels: supported_config.channels(),
            sample_rate: cpal::SampleRate(16000.clamp(min_rate, max_rate)),
            buffer_size: cpal::BufferSize::Default,
        };
        let buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));

        let sample_format = supported_config.sample_format();
        let stream = match sample_format {
            SampleFormat::I16 => build_stream::<i16>(&device, &config, buffer.clone()),
            SampleFormat::I32 => build_stream::<i32>(&device, &config, buffer.clone()),
            SampleFormat::F32 => build_stream::<f32>(&device, &config, buffer.clone()),
            SampleFormat::F64 => build_stream::<f64>(&device, &config, buffer.clone()),
            other => {
                return Err(RecordingError::MicrophoneUnavailable(format!(
                    "Unsupported sample format: {:?}",
                    other
                )));
            }
        }
        .map_err(RecordingError::AudioStreamError)?;

        stream.play().map_err(|e| {
            RecordingError::AudioStreamError(format!("Failed to start recording: {}", e))
        })?;

        *self = AudioRecorder::Recording {
            buffer,
            stream,
            sample_rate: config.sample_rate.0,
            channels: config.channels,
            start_time: Instant::now(),
            device_name,
            output_dir,
        };
        Ok(())
    }

    pub fn stop(&mut self) -> Result<RecordingResult, RecordingError> {
        let AudioRecorder::Recording {
            buffer,
            stream,
            sample_rate,
            channels,
            start_time,
            device_name,
            output_dir,
        } = self
        else {
            return Err(RecordingError::TooShort);
        };

        let _ = stream.pause();
        let duration_secs = start_time.elapsed().as_secs();

        if duration_secs == 0 {
            // 过短:丢弃,直接回 Idle(drop 流释放设备)
            *self = AudioRecorder::Idle {
                device_name: device_name.clone(),
                output_dir: output_dir.clone(),
            };
            return Err(RecordingError::TooShort);
        }

        let audio_data: Vec<i16> = {
            let mut buf = buffer.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *buf)
        };

        let dir = if let Some(d) = &output_dir {
            ensure_recordings_dir_in(d).map_err(RecordingError::FileError)?
        } else {
            ensure_recordings_dir().map_err(RecordingError::FileError)?
        };
        let flac_filename = generate_filename();
        let flac_path = dir.join(&flac_filename);
        let wav_path = flac_path.with_extension("wav");

        let spec = hound::WavSpec {
            channels: *channels,
            sample_rate: *sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(&wav_path, spec)
            .map_err(|e| RecordingError::FileError(format!("Failed to create WAV file: {}", e)))?;

        for sample in &audio_data {
            writer.write_sample(*sample).map_err(|e| {
                RecordingError::FileError(format!("Failed to write WAV sample: {}", e))
            })?;
        }
        writer.finalize().map_err(|e| {
            RecordingError::FileError(format!("Failed to finalize WAV file: {}", e))
        })?;

        let (final_path, format) = match wav_to_flac(&wav_path, &flac_path) {
            Ok(()) => {
                let _ = std::fs::remove_file(&wav_path);
                (flac_path, "flac".to_string())
            }
            Err(_) => {
                let final_path = flac_path.with_extension("wav");
                if wav_path != final_path {
                    let _ = std::fs::rename(&wav_path, &final_path);
                }
                (final_path, "wav".to_string())
            }
        };

        // 回 Idle:stream 随 Recording 分支销毁被 drop,麦克风释放
        *self = AudioRecorder::Idle {
            device_name: device_name.clone(),
            output_dir: output_dir.clone(),
        };

        Ok(RecordingResult {
            path: final_path,
            duration_secs,
            format,
        })
    }

    pub fn cancel(&mut self) -> u64 {
        match self {
            AudioRecorder::Recording {
                start_time,
                device_name,
                output_dir,
                ..
            } => {
                let duration_secs = start_time.elapsed().as_secs();
                *self = AudioRecorder::Idle {
                    device_name: device_name.clone(),
                    output_dir: output_dir.clone(),
                };
                duration_secs
            }
            AudioRecorder::Idle { .. } => 0,
        }
    }

    #[allow(dead_code)]
    pub fn is_recording(&self) -> bool {
        matches!(self, AudioRecorder::Recording { .. })
    }

    pub fn set_output_dir(&mut self, dir: PathBuf) {
        match self {
            AudioRecorder::Idle { output_dir, .. } => *output_dir = Some(dir),
            AudioRecorder::Recording { output_dir, .. } => *output_dir = Some(dir),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_leap_year() {
        assert!(!is_leap_year(2025));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(2100));
        assert!(is_leap_year(0));
        assert!(is_leap_year(4));
    }

    #[test]
    fn test_days_to_date_epoch() {
        assert_eq!(days_to_date(0), (1970, 1, 1));
    }

    #[test]
    fn test_days_to_date_2024_new_year() {
        let days_from_epoch: u64 = 19723;
        let (year, month, day) = days_to_date(days_from_epoch);
        assert_eq!(year, 2024);
        assert_eq!(month, 1);
        assert_eq!(day, 1);
    }

    #[test]
    fn test_days_to_date_leap_year_feb() {
        let days_from_epoch: u64 = 19723 + 31 + 28;
        let (year, month, day) = days_to_date(days_from_epoch);
        assert_eq!(year, 2024);
        assert_eq!(month, 2);
        assert_eq!(day, 29);
    }

    #[test]
    fn test_days_to_date_year_boundary() {
        let days_in_2023: u64 = 365;
        let days_2023_start: u64 = 19358;
        let (_, month, day) = days_to_date(days_2023_start);
        assert_eq!(month, 1);
        assert_eq!(day, 1);

        let (_, month, day) = days_to_date(days_2023_start + days_in_2023 - 1);
        assert_eq!(month, 12);
        assert_eq!(day, 31);
    }

    #[test]
    fn test_generate_filename_format() {
        let filename = generate_filename();
        assert!(filename.starts_with("talkshow_"));
        assert!(filename.ends_with(".flac"));
        let parts: Vec<&str> = filename
            .strip_prefix("talkshow_")
            .unwrap()
            .strip_suffix(".flac")
            .unwrap()
            .split('_')
            .collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 6);
    }

    #[test]
    fn test_recordings_dir() {
        let dir = recordings_dir();
        assert!(dir.ends_with("talkshow"));
    }

    #[test]
    fn test_ensure_recordings_dir() {
        let dir = ensure_recordings_dir().unwrap();
        assert!(dir.exists());
    }

    #[test]
    fn test_device_matches() {
        assert!(device_matches(
            "MacBook Pro Microphone",
            "macbook pro microphone"
        ));
        assert!(device_matches("  AirPods  ", "AirPods"));
        assert!(!device_matches("AirPods Pro", "AirPods Max"));
        assert!(device_matches("", ""));
    }

    #[test]
    fn test_list_input_devices_smoke() {
        // CI 无音频设备时应返回空表而非 panic
        let devices = list_input_devices();
        assert!(devices.len() <= 32);
        assert!(devices.iter().filter(|d| d.is_default).count() <= 1);
    }

    #[test]
    fn test_new_is_idle_no_stream() {
        // 懒建流:构造后必须处于 Idle,不得占用麦克风
        let r = AudioRecorder::new_with_device(None);
        assert!(matches!(r, AudioRecorder::Idle { .. }));
        assert!(!r.is_recording());

        let r = AudioRecorder::new_with_device(Some("Some Device"));
        match r {
            AudioRecorder::Idle { device_name, .. } => {
                assert_eq!(device_name.as_deref(), Some("Some Device"))
            }
            _ => panic!("expected Idle"),
        }
    }

    #[test]
    fn test_stop_cancel_when_idle() {
        // 未录音时 stop/cancel 不 panic
        let mut r = AudioRecorder::new_with_device(None);
        assert!(matches!(r.stop(), Err(RecordingError::TooShort)));
        assert_eq!(r.cancel(), 0);
        assert!(matches!(r, AudioRecorder::Idle { .. }));

        let mut r = AudioRecorder::new_with_device(Some("d"));
        r.set_output_dir(std::env::temp_dir());
        match r {
            AudioRecorder::Idle { output_dir, .. } => assert!(output_dir.is_some()),
            _ => panic!("expected Idle"),
        }
    }
}
