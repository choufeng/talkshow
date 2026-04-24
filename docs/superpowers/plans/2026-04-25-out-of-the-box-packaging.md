# Out-of-the-Box Packaging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Package ONNX Runtime and FFmpeg within the application to ensure out-of-the-box functionality.

**Architecture:** Use Tauri Resources for dynamic libraries (ONNX Runtime) and Tauri Sidecars for external binaries (FFmpeg). Modify Rust logic to use bundled assets.

**Tech Stack:** Tauri (v2), Rust, ort (ONNX Runtime), FFmpeg.

---

### Task 1: Prepare Directory Structure

**Files:**
- Create: `src-tauri/resources/.gitkeep`
- Create: `src-tauri/binaries/.gitkeep`

- [ ] **Step 1: Create directories**

```bash
mkdir -p src-tauri/resources src-tauri/binaries
touch src-tauri/resources/.gitkeep src-tauri/binaries/.gitkeep
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/resources/.gitkeep src-tauri/binaries/.gitkeep
git commit -m "chore: create directories for resources and binaries"
```

### Task 2: Update Tauri Configuration

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Add resources and externalBin to tauri.conf.json**

```json
// Inside "bundle" object
"bundle": {
  "active": true,
  "targets": "all",
  "icon": [...],
  "macOS": {},
  "resources": [
    "resources/libonnxruntime.dylib",
    "resources/onnxruntime.dll"
  ],
  "externalBin": [
    "binaries/ffmpeg"
  ]
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "config: bundle onnxruntime and ffmpeg sidecar"
```

### Task 3: Refactor ONNX Runtime Initialization

**Files:**
- Modify: `src-tauri/src/sensevoice/engine.rs`

- [ ] **Step 1: Modify `find_onnxruntime_dylib` to use Tauri path resolver**

Update the function to prioritize the application's resource directory.

```rust
fn find_onnxruntime_dylib() -> Option<PathBuf> {
    // 1. Check bundled resource first
    // In production, resources are in the app bundle
    #[cfg(not(debug_assertions))]
    {
        // This requires access to AppHandle or a pre-resolved path
        // For now, let's assume we check the standard relative path in bundle
    }
    
    // Fallback to current logic for dev or if resource not found
    // ... existing candidates ...
}
```

- [ ] **Step 2: Implement bundled path resolution logic**

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/sensevoice/engine.rs
git commit -m "feat: use bundled onnxruntime dylib"
```

### Task 4: Refactor FFmpeg Call to use Sidecar

**Files:**
- Modify: `src-tauri/src/sensevoice/engine.rs`

- [ ] **Step 1: Update `load_audio` to use `tauri_plugin_shell::Sidecar`**

Replace `Command::new("ffmpeg")` with a call to the sidecar.

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/sensevoice/engine.rs
git commit -m "feat: use ffmpeg sidecar for audio conversion"
```

### Task 5: Final Verification and Placeholder Documentation

- [ ] **Step 1: Add a README note about downloading binaries**

Since I cannot download binary files (ffmpeg, libonnxruntime) from the internet directly, I must provide instructions for the user to place them in the correct folders.

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add instructions for binary assets"
```
