# Batch 4: P2 逻辑 Bug + 哈希校验 + 日志脱敏 + URL 校验

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 1 个逻辑 bug、添加模型下载完整性校验、日志脱敏、前端 URL 格式校验。

**Architecture:** 四个独立的小修改，分布在 `audio_control.rs`、`sensevoice.rs`、`ai.rs`、`models/+page.svelte` 中，互不依赖。

**Tech Stack:** Rust / SvelteKit / sha2 crate

**前置依赖：** 无

---

## File Structure

| 文件 | 操作 |
|------|------|
| `src-tauri/src/audio_control.rs` | 修改 — 修复 `cleanup_stale_state` 逻辑 bug |
| `src-tauri/src/sensevoice.rs` | 修改 — 添加 SHA-256 校验 |
| `src-tauri/Cargo.toml` | 修改 — 添加 sha2 依赖 |
| `src-tauri/src/ai.rs` | 修改 — 日志脱敏 |
| `src/routes/models/+page.svelte` | 修改 — 前端 URL 校验 |

---

### Task 1: 修复 `audio_control.rs` 逻辑 bug

**Files:**
- Modify: `src-tauri/src/audio_control.rs:118-143`

**Context:** `cleanup_stale_state` 中，无论过期判断结果如何，都执行了相同的删除文件+恢复音量操作。过期判断形同虚设。

- [ ] **Step 1: 修复逻辑**

将 `audio_control.rs` 中的 `cleanup_stale_state` 函数替换为：

```rust
pub fn cleanup_stale_state(app_data_dir: &std::path::Path) -> Result<(), String> {
    let path = state_file_path(app_data_dir);

    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let state: MuteState = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if now - state.timestamp > MAX_STALE_SECONDS {
        let _ = fs::remove_file(&path);
        set_volume(state.volume)?;
    }

    Ok(())
}
```

- [ ] **Step 2: 运行测试**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/audio_control.rs
git commit -m "fix: cleanup_stale_state logic bug — only restore volume when stale (L-6)"
```

---

### Task 2: 模型下载添加 SHA-256 完整性校验

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/sensevoice.rs:68-74, 494-544`

**Context:** 模型文件完整性验证仅比较文件大小，中间人攻击者可用相同大小的恶意文件替换。需要添加 SHA-256 哈希校验。

- [ ] **Step 1: 添加 sha2 依赖**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 中添加：

```toml
sha2 = "0.10"
```

- [ ] **Step 2: 计算模型文件 SHA-256 哈希值**

**这一步需要实际下载文件计算哈希。** 由于文件较大（model_quant.onnx 241MB），先准备占位符，后续手动填充：

```bash
# 在有网络的环境下执行（按文件从小到大排列）：
curl -sL https://huggingface.co/haixuantao/SenseVoiceSmall-onnx/resolve/main/config.yaml | shasum -a 256
curl -sL https://huggingface.co/haixuantao/SenseVoiceSmall-onnx/resolve/main/am.mvn | shasum -a 256
curl -sL https://huggingface.co/haixuantao/SenseVoiceSmall-onnx/resolve/main/chn_jpn_yue_eng_ko_spectok.bpe.model | shasum -a 256
curl -sL https://huggingface.co/haixuantao/SenseVoiceSmall-onnx/resolve/main/tokens.json | shasum -a 256
# 最大的文件最后下载：
curl -sL https://huggingface.co/haixuantao/SenseVoiceSmall-onnx/resolve/main/model_quant.onnx | shasum -a 256
```

将计算出的哈希值填入下一步的常量中。

- [ ] **Step 3: 修改 `MODEL_FILES` 常量**

将 `sensevoice.rs:68-74`：

```rust
const MODEL_FILES: &[(&str, u64)] = &[
    ("model_quant.onnx", 241_216_270),
    ("config.yaml", 1_855),
    ("am.mvn", 11_203),
    ("chn_jpn_yue_eng_ko_spectok.bpe.model", 377_341),
    ("tokens.json", 352_064),
];
```

改为（`<HASH>` 替换为 Step 2 计算出的实际值）：

```rust
const MODEL_FILES: &[(&str, u64, &str)] = &[
    ("model_quant.onnx", 241_216_270, "<HASH>"),
    ("config.yaml", 1_855, "<HASH>"),
    ("am.mvn", 11_203, "<HASH>"),
    ("chn_jpn_yue_eng_ko_spectok.bpe.model", 377_341, "<HASH>"),
    ("tokens.json", 352_064, "<HASH>"),
];
```

- [ ] **Step 4: 添加哈希校验函数**

在 `sensevoice.rs` 中（MODEL_FILES 常量之后）添加：

```rust
use std::io::Read as _;

fn verify_file_hash(file_path: &std::path::Path, expected_hash: &str) -> Result<bool, String> {
    let mut file = std::fs::File::open(file_path).map_err(|e| e.to_string())?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    let result = format!("{:x}", hasher.finalize());
    Ok(result == expected_hash)
}
```

- [ ] **Step 5: 修改下载逻辑的解构和校验**

修改 `download_sensevoice_model` 函数中的循环：

将 `for &(filename, expected_size) in MODEL_FILES` 改为 `for &(filename, expected_size, expected_hash) in MODEL_FILES`。

修改 `is_valid` 检查（约第 496-498 行）：

```rust
let is_valid = match std::fs::metadata(&file_path) {
    Ok(meta) => {
        if meta.len() != expected_size {
            false
        } else {
            verify_file_hash(&file_path, expected_hash).unwrap_or(true)
        }
    }
    Err(_) => false,
};
```

在 `async_file.flush()` 之后、`std::fs::rename` 之前添加：

```rust
if let Ok(valid) = verify_file_hash(&tmp_path, expected_hash) {
    if !valid {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("SHA-256 hash mismatch for {}", filename));
    }
}
```

- [ ] **Step 6: 运行 cargo check**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

- [ ] **Step 7: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/sensevoice.rs
git commit -m "fix(security): add SHA-256 integrity check for model downloads (M-1)"
```

---

### Task 3: 日志脱敏

**Files:**
- Modify: `src-tauri/src/ai.rs`

**Context:** `ai.rs` 中的日志记录了完整的 API 端点 URL、prompt 等敏感信息。日志可通过前端 `get_log_content` 命令读取。需要移除不必要的敏感字段。

- [ ] **Step 1: 搜索 `ai.rs` 中所有包含敏感信息的日志**

搜索 `ai.rs` 中所有 `logger.info` 和 `logger.error` 调用，找出包含 `url`、`endpoint`、`prompt`、`api_key` 等字段的日志。

重点位置：
- 约 207-217 行：transcription 请求日志（包含 `url` 和 `prompt`）
- 约 380-390 行：其他 AI 请求日志（可能包含 `url`）

- [ ] **Step 2: 移除日志中的敏感字段**

对于 transcription 请求日志，将包含 `"url"` 和 `"prompt"` 的字段移除，只保留 `"model"`、`"media_type"`、`"audio_size_bytes"`。

对于其他 AI 请求日志，移除 `"url"` 字段，保留 `"model"`。

**原则：** 日志应足以调试问题，但不应泄露敏感数据。模型名、文件大小、媒体类型是安全的；完整 URL、prompt 内容不是。

- [ ] **Step 3: 运行测试**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/ai.rs
git commit -m "fix(security): remove sensitive data from logs (M-2)"
```

---

### Task 4: 前端 Endpoint URL 格式校验

**Files:**
- Modify: `src/routes/models/+page.svelte`

**Context:** 用户添加 openai-compatible 类型 Provider 时可输入任意字符串作为 endpoint，前端和后端均未验证 URL 格式。后端校验已在 Batch 1 Task 3 中添加，此处添加前端即时反馈。

- [ ] **Step 1: 在 `models/+page.svelte` 中添加 URL 校验函数**

在 script 区域添加：

```typescript
function validateEndpointUrl(url: string): string | null {
    if (!url.trim()) return null; // 空值允许（vertex/sensevoice）
    try {
        const parsed = new URL(url);
        if (!['http:', 'https:'].includes(parsed.protocol)) {
            return 'Endpoint 必须以 http:// 或 https:// 开头';
        }
    } catch {
        return 'Endpoint 格式无效';
    }
    return null;
}
```

- [ ] **Step 2: 在 Provider 添加/保存时调用校验**

找到 `models/+page.svelte` 中添加或编辑 Provider 的保存逻辑，在校验链中添加 `validateEndpointUrl` 调用。

如果已有校验函数（如检查 name/id 是否为空），将 URL 校验添加到同一处。如果 provider_type 为 `openai-compatible`，才执行 URL 校验。

- [ ] **Step 3: 手动验证**

Run: `npm run tauri dev`
验证：
- 添加 openai-compatible Provider 时输入 `ftp://evil.com`，显示错误
- 输入 `https://api.example.com/v1`，正常保存
- vertex 和 sensevoice 类型不要求 endpoint

- [ ] **Step 4: Commit**

```bash
git add src/routes/models/+page.svelte
git commit -m "fix(security): add endpoint URL format validation in frontend (M-4)"
```
