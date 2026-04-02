use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dasp_sample::Sample as DaspSample;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

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

pub enum AudioRecorder {
    Ready {
        buffer: Arc<Mutex<Vec<i16>>>,
        stream: cpal::Stream,
        sample_rate: u32,
        channels: u16,
        start_time: Option<Instant>,
    },
    Unavailable(String),
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
                for &sample in data {
                    buf.push(sample_to_i16(sample));
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
    pub fn new() -> Self {
        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(d) => d,
            None => return AudioRecorder::Unavailable("No microphone device available".into()),
        };

        let mut supported_configs = match device.supported_input_configs() {
            Ok(configs) => configs,
            Err(e) => {
                return AudioRecorder::Unavailable(format!(
                    "Failed to query microphone configs: {}",
                    e
                ));
            }
        };

        let supported_config = match supported_configs.next() {
            Some(c) => c,
            None => {
                return AudioRecorder::Unavailable(
                    "Microphone has no supported input configurations".into(),
                );
            }
        };

        let config: cpal::StreamConfig = supported_config.with_max_sample_rate().into();

        let buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));

        let sample_format = supported_config.sample_format();
        let stream = match sample_format {
            SampleFormat::I16 => build_stream::<i16>(&device, &config, buffer.clone()),
            SampleFormat::I32 => build_stream::<i32>(&device, &config, buffer.clone()),
            SampleFormat::F32 => build_stream::<f32>(&device, &config, buffer.clone()),
            SampleFormat::F64 => build_stream::<f64>(&device, &config, buffer.clone()),
            _ => {
                return AudioRecorder::Unavailable(format!(
                    "Unsupported sample format: {:?}",
                    sample_format
                ));
            }
        };

        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                return AudioRecorder::Unavailable(format!("Failed to create audio stream: {}", e));
            }
        };

        AudioRecorder::Ready {
            buffer,
            stream,
            sample_rate: config.sample_rate.0,
            channels: config.channels,
            start_time: None,
        }
    }

    pub fn start(&mut self) -> Result<(), RecordingError> {
        match self {
            AudioRecorder::Ready {
                buffer,
                stream,
                start_time,
                ..
            } => {
                if let Ok(mut b) = buffer.lock() {
                    b.clear()
                }
                stream.play().map_err(|e| {
                    RecordingError::AudioStreamError(format!("Failed to start recording: {}", e))
                })?;
                *start_time = Some(Instant::now());
                Ok(())
            }
            AudioRecorder::Unavailable(reason) => {
                Err(RecordingError::MicrophoneUnavailable(reason.clone()))
            }
        }
    }

    pub fn stop(&mut self) -> Result<RecordingResult, RecordingError> {
        match self {
            AudioRecorder::Ready {
                buffer,
                stream,
                sample_rate,
                channels,
                start_time,
            } => {
                let _ = stream.pause();

                let duration_secs = start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0);

                if duration_secs == 0 {
                    if let Ok(mut b) = buffer.lock() {
                        b.clear()
                    }
                    *start_time = None;
                    return Err(RecordingError::TooShort);
                }

                let audio_data: Vec<i16> = {
                    let mut buf = buffer.lock().unwrap_or_else(|e| e.into_inner());
                    std::mem::take(&mut *buf)
                };
                *start_time = None;

                let dir = ensure_recordings_dir().map_err(RecordingError::FileError)?;
                let flac_filename = generate_filename();
                let flac_path = dir.join(&flac_filename);
                let wav_path = flac_path.with_extension("wav");

                let spec = hound::WavSpec {
                    channels: *channels,
                    sample_rate: *sample_rate,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };

                let mut writer = hound::WavWriter::create(&wav_path, spec).map_err(|e| {
                    RecordingError::FileError(format!("Failed to create WAV file: {}", e))
                })?;

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

                Ok(RecordingResult {
                    path: final_path,
                    duration_secs,
                    format,
                })
            }
            AudioRecorder::Unavailable(reason) => {
                Err(RecordingError::MicrophoneUnavailable(reason.clone()))
            }
        }
    }

    pub fn cancel(&mut self) -> u64 {
        match self {
            AudioRecorder::Ready {
                buffer,
                stream,
                start_time,
                ..
            } => {
                let _ = stream.pause();
                let duration_secs = start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0);
                if let Ok(mut b) = buffer.lock() {
                    b.clear()
                }
                *start_time = None;
                duration_secs
            }
            AudioRecorder::Unavailable(_) => 0,
        }
    }

    #[allow(dead_code)]
    pub fn is_recording(&self) -> bool {
        match self {
            AudioRecorder::Ready { start_time, .. } => start_time.is_some(),
            AudioRecorder::Unavailable(_) => false,
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
}
