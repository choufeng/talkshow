# Out-of-the-Box Packaging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bundle `onnxruntime` and `ffmpeg` inside the Tauri app package so users get a fully working app with no external dependencies.

**Architecture:** `libonnxruntime.dylib` is bundled as a Tauri resource and loaded via its resolved path at runtime; `ffmpeg` is bundled as a Tauri sidecar binary and invoked via a resolved binary path at runtime. Code in `sensevoice/engine.rs` is updated to use these paths instead of searching the system.

**Tech Stack:** Rust, Tauri 2, `ort` crate (load-dynamic), `tauri-plugin-shell`

---

## File Map

| File | Action |
|------|--------|
| `src-tauri/tauri.conf.json` | Add `resources` and `externalBin` to `bundle` |
| `src-tauri/Cargo.toml` | Add `tauri-plugin-shell` dependency |
| `src-tauri/capabilities/default.json` | Add shell plugin permissions |
| `src-tauri/src/lib.rs` | Register `tauri_plugin_shell` |
| `src-tauri/src/sensevoice/engine.rs` | Replace system-search logic with bundled resource paths |
| `src-tauri/src/sensevoice/mod.rs` | Pass `AppHandle` to engine where needed |
| `src-tauri/resources/` | Create directory, add `libonnxruntime.dylib` (manual step) |
| `src-tauri/binaries/` | Create directory, add `ffmpeg-aarch64-apple-darwin` (manual step) |

---

## Task 1: Download and place bundled binaries

This is a manual preparation task — no code changes.

- [ ] **Step 1: Download ffmpeg for macOS Apple Silicon**

```bash
# Download a static build of ffmpeg
curl -L "https://evermeet.cx/ffmpeg/getrelease/ffmpeg/zip" -o /tmp/ffmpeg.zip
unzip /tmp/ffmpeg.zip -d /tmp/ffmpeg-extract
```

- [ ] **Step 2: Place ffmpeg as a Tauri sidecar**

```bash
mkdir -p src-tauri/binaries
cp /tmp/ffmpeg-extract/ffmpeg src-tauri/binaries/ffmpeg-aarch64-apple-darwin
chmod +x src-tauri/binaries/ffmpeg-aarch64-apple-darwin
```

- [ ] **Step 3: Download onnxruntime dylib for macOS Apple Silicon**

```bash
# Download official onnxruntime release (v1.20.x matches ort 2.0.0-rc.12)
curl -L "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-osx-arm64-1.20.1.tgz" -o /tmp/ort.tgz
tar -xzf /tmp/ort.tgz -C /tmp/
```

- [ ] **Step 4: Place onnxruntime as a Tauri resource**

```bash
mkdir -p src-tauri/resources
cp /tmp/onnxruntime-osx-arm64-1.20.1/lib/libonnxruntime.1.20.1.dylib src-tauri/resources/libonnxruntime.dylib
```

- [ ] **Step 5: Verify files exist**

```bash
ls -lh src-tauri/binaries/ffmpeg-aarch64-apple-darwin
ls -lh src-tauri/resources/libonnxruntime.dylib
```

Expected output: both files present, `ffmpeg` is executable.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/binaries/.gitkeep src-tauri/resources/.gitkeep
# Note: add the actual binaries to .gitignore if too large; use git-lfs or document download steps
git commit -m "chore: add directories for bundled binaries"
```

---

## Task 2: Update `tauri.conf.json` to bundle resources

- [ ] **Step 1: Edit `src-tauri/tauri.conf.json`**

Replace the existing `bundle` section:

```json
"bundle": {
  "active": true,
  "targets": "all",
  "icon": [
    "icons/32x32.png",
    "icons/128x128.png",
    "icons/128x128@2x.png",
    "icons/icon.icns",
    "icons/icon.ico"
  ],
  "resources": [
    "resources/libonnxruntime.dylib"
  ],
  "externalBin": [
    "binaries/ffmpeg"
  ],
  "macOS": {}
}
```

- [ ] **Step 2: Verify JSON is valid**

```bash
node -e "JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json','utf8')); console.log('OK')"
```

Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "feat: bundle onnxruntime and ffmpeg as tauri resources"
```

---

## Task 3: Add `tauri-plugin-shell` for sidecar invocation

- [ ] **Step 1: Add dependency to `src-tauri/Cargo.toml`**

In the `[dependencies]` section, add:

```toml
tauri-plugin-shell = "2"
```

- [ ] **Step 2: Register plugin in `src-tauri/src/lib.rs`**

Find the `tauri::Builder::default()` chain and add `.plugin(tauri_plugin_shell::init())`. It should look like:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    // ... existing plugins ...
```

Add the use statement at the top of `lib.rs` if not already present:
```rust
// No use statement needed; it's accessed via tauri_plugin_shell::init()
```

- [ ] **Step 3: Add shell permission to `src-tauri/capabilities/default.json`**

Add `"shell:allow-execute"` and `"shell:allow-open"` to the `permissions` array:

```json
"permissions": [
    "core:default",
    "opener:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "global-shortcut:allow-is-registered",
    "core:window:allow-start-dragging",
    "core:window:allow-show",
    "core:window:allow-close",
    "core:window:allow-set-focus",
    "core:window:allow-center",
    "core:webview:allow-create-webview-window",
    "notification:default",
    "shell:allow-execute"
]
```

- [ ] **Step 4: Verify it compiles**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

Expected: no errors (warnings OK).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs src-tauri/capabilities/default.json
git commit -m "feat: add tauri-plugin-shell for sidecar binary support"
```

---

## Task 4: Resolve bundled resource paths in engine

This is the core code change. We replace `find_onnxruntime_dylib` and the `ffmpeg` `Command::new` call with path resolution from the Tauri app package.

- [ ] **Step 1: Check the current signature of `SenseVoiceEngine::new` and its call sites**

```bash
grep -rn "SenseVoiceEngine::new\|SenseVoiceState" src-tauri/src/ | grep -v "test"
```

Note the call sites — we need to thread an `AppHandle` down to the engine.

- [ ] **Step 2: Add a helper module for resolving bundled paths**

Create `src-tauri/src/sensevoice/bundled_paths.rs`:

```rust
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Returns the path to the bundled libonnxruntime dylib inside the app package.
pub fn onnxruntime_dylib_path(app: &AppHandle) -> Option<PathBuf> {
    let resource_path = app
        .path()
        .resource_dir()
        .ok()?
        .join("resources")
        .join("libonnxruntime.dylib");
    if resource_path.exists() {
        Some(resource_path)
    } else {
        None
    }
}

/// Returns the path to the bundled ffmpeg sidecar binary.
pub fn ffmpeg_bin_path(app: &AppHandle) -> Option<PathBuf> {
    let bin_path = app
        .path()
        .resource_dir()
        .ok()?
        .join("binaries")
        .join(format!("ffmpeg-{}", std::env::consts::ARCH));
    // Tauri sidecar naming: ffmpeg-aarch64-apple-darwin (no extension on macOS)
    if bin_path.exists() {
        return Some(bin_path);
    }
    // Try the full Tauri triple format if simple arch doesn't match
    let triple = tauri::utils::platform::target_triple().ok()?;
    let full_path = app
        .path()
        .resource_dir()
        .ok()?
        .join("binaries")
        .join(format!("ffmpeg-{}", triple));
    if full_path.exists() {
        Some(full_path)
    } else {
        None
    }
}
```

- [ ] **Step 3: Register the new module in `src-tauri/src/sensevoice/mod.rs`**

Open `src-tauri/src/sensevoice/mod.rs` and add:

```rust
pub mod bundled_paths;
```

- [ ] **Step 4: Rewrite `find_onnxruntime_dylib` and `ensure_ort_initialized` in `engine.rs`**

Replace the entire `find_onnxruntime_dylib` function and `ensure_ort_initialized` with:

```rust
use tauri::AppHandle;
use super::bundled_paths;

fn ensure_ort_initialized(app: &AppHandle) -> Result<(), SenseVoiceError> {
    if ORT_INITIALIZED.load(Ordering::Acquire) {
        return Ok(());
    }
    let dylib_path = bundled_paths::onnxruntime_dylib_path(app).ok_or_else(|| {
        SenseVoiceError::LoadFailed(
            "ONNX Runtime library not found in app bundle. Please reinstall the application."
                .to_string(),
        )
    })?;
    ort::init_from(&dylib_path)
        .map_err(|e| {
            SenseVoiceError::LoadFailed(format!(
                "Failed to load ONNX Runtime from {}: {}",
                dylib_path.display(),
                e
            ))
        })?
        .commit();
    ORT_INITIALIZED.store(true, Ordering::Release);
    Ok(())
}
```

- [ ] **Step 5: Update `SenseVoiceEngine::new` to accept `AppHandle`**

Replace the signature and body of `SenseVoiceEngine::new`:

```rust
pub fn new(model_dir: &std::path::Path, app: &AppHandle) -> Result<Self, SenseVoiceError> {
    ensure_ort_initialized(app)?;
    // rest stays the same
    let onnx_path = model_dir.join("model_quant.onnx");
    let session = Session::builder()
        .map_err(|e| SenseVoiceError::LoadFailed(e.to_string()))?
        .with_intra_threads(4)
        .map_err(|e| SenseVoiceError::LoadFailed(e.to_string()))?
        .commit_from_file(&onnx_path)
        .map_err(|e| SenseVoiceError::LoadFailed(e.to_string()))?;
    let (cmvn_means, cmvn_vars) =
        parse_cmvn(&model_dir.join("am.mvn")).map_err(SenseVoiceError::LoadFailed)?;
    Ok(Self {
        session,
        cmvn_means,
        cmvn_vars,
        model_dir: model_dir.to_path_buf(),
    })
}
```

Also add `AppHandle` to the `use` statements at the top of `engine.rs`:

```rust
use tauri::AppHandle;
```

- [ ] **Step 6: Update `load_audio` to use bundled ffmpeg**

The `load_audio` function currently calls `Command::new("ffmpeg")`. We need to pass the ffmpeg path into it. Update the signature and body:

```rust
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
    // rest stays the same
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
```

- [ ] **Step 7: Store ffmpeg path in `SenseVoiceEngine` and pass it to `transcribe`**

Add `ffmpeg_path` field to the struct and resolve it during `new`:

```rust
pub struct SenseVoiceEngine {
    session: Session,
    cmvn_means: Array1<f64>,
    cmvn_vars: Array1<f64>,
    model_dir: PathBuf,
    ffmpeg_path: PathBuf,
}
```

In `new`, resolve and store ffmpeg path:

```rust
let ffmpeg_path = bundled_paths::ffmpeg_bin_path(app).ok_or_else(|| {
    SenseVoiceError::LoadFailed(
        "ffmpeg not found in app bundle. Please reinstall the application.".to_string(),
    )
})?;
Ok(Self {
    session,
    cmvn_means,
    cmvn_vars,
    model_dir: model_dir.to_path_buf(),
    ffmpeg_path,
})
```

In `transcribe`, pass `&self.ffmpeg_path` to `load_audio`:

```rust
let (waveform, sample_rate) = load_audio(audio_path, &self.ffmpeg_path)?;
```

- [ ] **Step 8: Fix all call sites of `SenseVoiceEngine::new`**

```bash
grep -rn "SenseVoiceEngine::new" src-tauri/src/
```

For each call site, add the `app` argument. Typically in `pipeline.rs` or `providers/sensevoice.rs`:

```rust
// Before:
SenseVoiceEngine::new(&model_dir)?
// After:
SenseVoiceEngine::new(&model_dir, &app_handle)?
```

The `app_handle` should be available via the Tauri `AppHandle` that is already threaded through pipeline state.

- [ ] **Step 9: Verify it compiles**

```bash
cd src-tauri && cargo check 2>&1 | tail -20
```

Expected: no errors.

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/sensevoice/
git commit -m "feat: load onnxruntime and ffmpeg from app bundle instead of system paths"
```

---

## Task 5: Verify end-to-end in dev mode

- [ ] **Step 1: Run in dev mode**

```bash
npm run tauri dev
```

Expected: app launches without errors about missing onnxruntime or ffmpeg.

- [ ] **Step 2: Test transcription**

Trigger a recording and verify transcription works correctly.

- [ ] **Step 3: Run Rust tests**

```bash
cd src-tauri && cargo test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 4: Commit any fixes**

```bash
git add -A && git commit -m "fix: resolve bundle path issues found during dev testing"
```

---

## Task 6: Build and verify the app bundle

- [ ] **Step 1: Build the app**

```bash
npm run tauri build
```

Expected: build succeeds, `.dmg` or `.app` created under `src-tauri/target/release/bundle/`.

- [ ] **Step 2: Verify bundled resources exist in the `.app`**

```bash
ls src-tauri/target/release/bundle/macos/TalkShow.app/Contents/Resources/
ls src-tauri/target/release/bundle/macos/TalkShow.app/Contents/MacOS/
```

Expected:
- `Resources/` contains `libonnxruntime.dylib`
- `MacOS/` contains `ffmpeg-aarch64-apple-darwin`

- [ ] **Step 3: Test the built `.app` on a clean environment**

Move the `.app` to a machine (or Docker container) without Homebrew or Python and verify it starts and transcribes correctly.

- [ ] **Step 4: Final commit**

```bash
git add -A && git commit -m "chore: verify out-of-the-box packaging works end-to-end"
```
