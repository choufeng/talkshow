# 移除选中内容检测/替换模式功能 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 移除转写流程中的选中文字检测、替换模式状态和 AI prompt 上下文注入，保留剪贴板写入+粘贴功能。

**Architecture:** 逐文件精简，从底层模块（clipboard.rs）开始向上层（lib.rs、skills.rs、前端）逐层清理，最后更新测试。

**Tech Stack:** Rust (Tauri), Svelte (前端), arboard crate

---

### Task 1: 精简 clipboard.rs

**Files:**
- Modify: `src-tauri/src/clipboard.rs`

- [ ] **Step 1: 重写 clipboard.rs，移除选中文字相关代码**

删除 `SELECTED_TEXT` 静态变量、`save_selected_text()`、`get_saved_selected_text()`、`get_replace_mode_state()`、`detect_selected_text()` 两个平台实现。保留 `TARGET_APP`、`save_target_app()`、`write_and_paste()`、`escape_applescript_string()`、`simulate_paste()`。

文件最终内容：

```rust
use std::sync::Mutex;

static TARGET_APP: Mutex<Option<String>> = Mutex::new(None);

pub fn save_target_app(app_name: &str) {
    if let Ok(mut guard) = TARGET_APP.lock() {
        *guard = Some(app_name.to_string());
    }
}

pub fn write_and_paste(text: &str) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to write to clipboard: {}", e))?;
    simulate_paste();
    Ok(())
}

#[cfg(target_os = "macos")]
fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(target_os = "macos")]
fn simulate_paste() {
    let target_app = TARGET_APP.lock().ok().and_then(|g| g.clone());
    if let Some(app) = target_app {
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!(
                "tell application \"{}\" to activate",
                escape_applescript_string(&app)
            ))
            .output();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"v\" using command down")
        .output();
}

#[cfg(not(target_os = "macos"))]
fn simulate_paste() {
    eprintln!("[TalkShow] Paste simulation not supported on this platform");
}
```

- [ ] **Step 2: 验证编译**

Run: `cd src-tauri && cargo check 2>&1 | head -30`
Expected: 多个编译错误（因为 lib.rs 仍引用已删除函数），这是预期的。确认没有 clipboard.rs 内部的语法错误即可。

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/clipboard.rs
git commit -m "refactor: remove selection detection from clipboard module"
```

---

### Task 2: 更新 skills.rs — 移除 selected_text 参数

**Files:**
- Modify: `src-tauri/src/skills.rs`

- [ ] **Step 1: 移除 assemble_skills_prompt 的 selected_text 参数**

将第 54-74 行从：

```rust
pub fn assemble_skills_prompt(
    skills: &[Skill],
    transcription: &str,
    app_name: &str,
    bundle_id: &str,
    selected_text: Option<&str>,
) -> (String, String) {
    let mut system_prompt = String::from(
        "你是一个语音转文字的文本处理助手。请根据以下规则处理用户的输入文本。\n\n基础规则：\n1. 如果输入是短内容（如单词、短语、简短回答）或非完整句子，不要添加句尾标点符号。只有当输入明显构成完整句子时才添加标点。\n2. 当输入包含多种语言（如中英文混用）时，保留各语言的原文表达，不要尝试翻译或统一为某一种语言。\n",
    );

    system_prompt.push_str(&format!(
        "当前用户正在使用的应用是：{} ({})\n",
        app_name, bundle_id
    ));
    if let Some(selected) = selected_text {
        system_prompt.push_str(&format!(
            "用户选中了以下文字，准备用语音替换它。请在处理转写结果时考虑这个上下文，使替换后的文本自然衔接。\n选中的原文：「{}」\n",
            selected
        ));
    }
    system_prompt.push_str("请仅应用与当前场景相关的规则，跳过不适用的规则。");
```

改为：

```rust
pub fn assemble_skills_prompt(
    skills: &[Skill],
    transcription: &str,
    app_name: &str,
    bundle_id: &str,
) -> (String, String) {
    let mut system_prompt = String::from(
        "你是一个语音转文字的文本处理助手。请根据以下规则处理用户的输入文本。\n\n基础规则：\n1. 如果输入是短内容（如单词、短语、简短回答）或非完整句子，不要添加句尾标点符号。只有当输入明显构成完整句子时才添加标点。\n2. 当输入包含多种语言（如中英文混用）时，保留各语言的原文表达，不要尝试翻译或统一为某一种语言。\n",
    );

    system_prompt.push_str(&format!(
        "当前用户正在使用的应用是：{} ({})\n",
        app_name, bundle_id
    ));
    system_prompt.push_str("请仅应用与当前场景相关的规则，跳过不适用的规则。");
```

- [ ] **Step 2: 移除 process_with_skills 的 selected_text 参数**

将第 88-96 行从：

```rust
pub async fn process_with_skills(
    logger: &Logger,
    skills_config: &SkillsConfig,
    transcription_config: &crate::config::TranscriptionConfig,
    providers: &[ProviderConfig],
    transcription: &str,
    vertex_cache: &ProviderContext,
    selected_text: Option<&str>,
) -> Result<String, String> {
```

改为：

```rust
pub async fn process_with_skills(
    logger: &Logger,
    skills_config: &SkillsConfig,
    transcription_config: &crate::config::TranscriptionConfig,
    providers: &[ProviderConfig],
    transcription: &str,
    vertex_cache: &ProviderContext,
) -> Result<String, String> {
```

将第 145-151 行从：

```rust
    let (system_prompt, user_message) = assemble_skills_prompt(
        &skills_owned,
        transcription,
        &app_name,
        &bundle_id,
        selected_text,
    );
```

改为：

```rust
    let (system_prompt, user_message) = assemble_skills_prompt(
        &skills_owned,
        transcription,
        &app_name,
        &bundle_id,
    );
```

- [ ] **Step 3: 移除 process_with_skills_client 的 selected_text 参数**

将第 225-232 行从：

```rust
pub async fn process_with_skills_client(
    logger: &Logger,
    skills_config: &SkillsConfig,
    transcription_config: &crate::config::TranscriptionConfig,
    providers: &[ProviderConfig],
    transcription: &str,
    client: &mut dyn LlmClient,
    selected_text: Option<&str>,
) -> Result<String, String> {
```

改为：

```rust
pub async fn process_with_skills_client(
    logger: &Logger,
    skills_config: &SkillsConfig,
    transcription_config: &crate::config::TranscriptionConfig,
    providers: &[ProviderConfig],
    transcription: &str,
    client: &mut dyn LlmClient,
) -> Result<String, String> {
```

将第 277-283 行从：

```rust
    let (system_prompt, user_message) = assemble_skills_prompt(
        &skills_owned,
        transcription,
        &app_name,
        &bundle_id,
        selected_text,
    );
```

改为：

```rust
    let (system_prompt, user_message) = assemble_skills_prompt(
        &skills_owned,
        transcription,
        &app_name,
        &bundle_id,
    );
```

- [ ] **Step 4: 验证 skills.rs 编译**

Run: `cd src-tauri && cargo check 2>&1 | head -30`
Expected: lib.rs 和测试文件有编译错误（因为调用签名变了），skills.rs 本身无错误。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/skills.rs
git commit -m "refactor: remove selected_text param from skills functions"
```

---

### Task 3: 更新 lib.rs — 移除选中文字检测流程

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 移除 get_replace_mode_state Tauri Command 注册**

在第 1075 行附近，从 invoke_handler 列表中删除 `clipboard::get_replace_mode_state`：

将：
```rust
            logger::get_log_sessions,
            logger::get_log_content,
            clipboard::get_replace_mode_state
        ])
```

改为：
```rust
            logger::get_log_sessions,
            logger::get_log_content
        ])
```

- [ ] **Step 2: 简化 show_indicator 函数**

将第 585-589 行从：

```rust
fn show_indicator(app_handle: &tauri::AppHandle, selected_text: Option<&str>) {
    let payload = serde_json::json!({
        "replaceMode": selected_text.is_some(),
        "selectedPreview": selected_text.map(|t| t.chars().take(50).collect::<String>()).unwrap_or_default()
    });
```

改为：

```rust
fn show_indicator(app_handle: &tauri::AppHandle) {
    let payload = serde_json::json!({
        "replaceMode": false,
        "selectedPreview": ""
    });
```

搜索所有 `show_indicator(` 调用点，移除第二个参数。调用点应为 `show_indicator(&app_handle, None)` 或 `show_indicator(&app_handle, Some(...))`，统一改为 `show_indicator(&app_handle)`。

- [ ] **Step 3: 移除转写完成后获取 selected_text 的逻辑**

将第 306-316 行从：

```rust
                                    let skills_start = Instant::now();
                                    let selected_text = clipboard::get_saved_selected_text();
                                    let selected_text_ref = selected_text.as_deref();
                                    let mut final_text = skills::process_with_skills(
                                        &logger,
                                        &skills_config,
                                        &app_config.features.transcription,
                                        &skills_providers,
                                        &text,
                                        &h.state::<ProviderContext>(),
                                        selected_text_ref,
                                    )
```

改为：

```rust
                                    let skills_start = Instant::now();
                                    let mut final_text = skills::process_with_skills(
                                        &logger,
                                        &skills_config,
                                        &app_config.features.transcription,
                                        &skills_providers,
                                        &text,
                                        &h.state::<ProviderContext>(),
                                    )
```

- [ ] **Step 4: 简化 Phase 2 后台线程**

将第 1306-1357 行的后台线程从：

```rust
                                                std::thread::spawn(move || {
                                                    // Get frontmost app name
                                                    let frontmost = std::process::Command::new("osascript")
                                                        .arg("-e")
                                                        .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
                                                        .output()
                                                        .ok()
                                                        .filter(|o| o.status.success())
                                                        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

                                                    // Detect selected text
                                                    let selected_text = clipboard::detect_selected_text(frontmost.as_deref().unwrap_or(""));

                                                    // Save target app and selected text
                                                    if let Some(ref app) = frontmost {
                                                        clipboard::save_target_app(app);
                                                    }
                                                    if let Some(ref text) = selected_text {
                                                        clipboard::save_selected_text(text);
                                                        if let Some(logger) = app_handle_bg.try_state::<Logger>() {
                                                            logger.info("recording", "检测到选中文本，进入替换模式", Some(serde_json::json!({
                                                                "selected_length": text.len(),
                                                                "selected_preview": text.chars().take(100).collect::<String>()
                                                            })));
                                                        }
                                                        // Update indicator with replaceMode
                                                        let payload = serde_json::json!({
                                                            "replaceMode": true,
                                                            "selectedPreview": text.chars().take(50).collect::<String>()
                                                        });
                                                        let _ = app_handle_bg.emit_to(INDICATOR_LABEL, "indicator:recording", &payload);
                                                    }

                                                    // Auto mute
                                                    ...
```

改为：

```rust
                                                std::thread::spawn(move || {
                                                    // Get frontmost app name
                                                    let frontmost = std::process::Command::new("osascript")
                                                        .arg("-e")
                                                        .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
                                                        .output()
                                                        .ok()
                                                        .filter(|o| o.status.success())
                                                        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

                                                    // Save target app for later paste
                                                    if let Some(ref app) = frontmost {
                                                        clipboard::save_target_app(app);
                                                    }

                                                    // Auto mute
                                                    ...
```

注意：保留自动静音和 ESC 快捷键注册部分不变，只删除 `detect_selected_text`、`save_selected_text`、替换模式指示器更新、相关日志的代码块。

同时将第 1298 行的注释：
```rust
                                                // Show indicator immediately (without selected_text, will update later)
```
改为：
```rust
                                                // Show indicator immediately
```

- [ ] **Step 5: 验证编译**

Run: `cd src-tauri && cargo check 2>&1 | head -30`
Expected: 仅测试文件可能有编译错误，lib.rs 和 clipboard.rs 无错误。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "refactor: remove selection detection from recording pipeline"
```

---

### Task 4: 更新测试文件

**Files:**
- Modify: `src-tauri/tests/skills_pipeline.rs`

- [ ] **Step 1: 删除 test_skills_with_selected_text 测试**

删除第 222-247 行（含测试上方的空行）：

```rust
#[tokio::test]
async fn test_skills_with_selected_text() {
    let (logger, _dir) = create_test_logger();
    let config = enabled_skills_config();
    let tc = test_transcription_config();
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, _, _| {
        assert!(prompt.contains("选中的文本"));
        Ok("处理结果".to_string())
    });

    let result = process_with_skills_client(
        &logger,
        &config,
        &tc,
        &providers,
        "转录文本",
        &mut mock,
        Some("选中的文本"),
    )
    .await;

    assert_eq!(result.unwrap(), "处理结果");
    assert_eq!(mock.send_text_call_count(), 1);
}
```

- [ ] **Step 2: 更新 test_assemble_skills_prompt_with_app_context**

将第 314-326 行从：

```rust
    let (system, user) = assemble_skills_prompt(
        &skills,
        "转录文本",
        "Finder",
        "com.apple.finder",
        Some("选中文本"),
    );

    assert!(system.contains("测试技能"));
    assert!(system.contains("测试 prompt"));
    assert!(system.contains("Finder"));
    assert!(system.contains("com.apple.finder"));
    assert!(system.contains("选中文本"));
    assert_eq!(user, "转录文本");
```

改为：

```rust
    let (system, user) = assemble_skills_prompt(
        &skills,
        "转录文本",
        "Finder",
        "com.apple.finder",
    );

    assert!(system.contains("测试技能"));
    assert!(system.contains("测试 prompt"));
    assert!(system.contains("Finder"));
    assert!(system.contains("com.apple.finder"));
    assert_eq!(user, "转录文本");
```

- [ ] **Step 3: 更新 test_assemble_skills_prompt_without_selected_text**

将第 331-337 行从：

```rust
fn test_assemble_skills_prompt_without_selected_text() {
    let skills: Vec<Skill> = vec![];
    let (system, user) = assemble_skills_prompt(&skills, "你好", "App", "com.app", None);

    assert!(!system.contains("选中文本"));
    assert_eq!(user, "你好");
}
```

改为：

```rust
fn test_assemble_skills_prompt_without_selected_text() {
    let skills: Vec<Skill> = vec![];
    let (system, user) = assemble_skills_prompt(&skills, "你好", "App", "com.app");

    assert_eq!(user, "你好");
}
```

- [ ] **Step 4: 搜索并更新其他测试中的 process_with_skills_client 调用**

搜索测试文件中所有 `process_with_skills_client(` 调用，移除最后一个 `None` 或 `Some(...)` 参数。同样搜索所有 `process_with_skills(` 调用（如有），移除 `selected_text` 相关参数。

- [ ] **Step 5: 验证编译和测试**

Run: `cd src-tauri && cargo test 2>&1`
Expected: 所有测试编译通过并通过。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/tests/skills_pipeline.rs
git commit -m "test: update tests after removing selected_text param"
```

---

### Task 5: 更新前端 recording/+page.svelte

**Files:**
- Modify: `src/routes/recording/+page.svelte`

- [ ] **Step 1: 移除 replaceMode/selectedPreview 状态和 fetchReplaceModeState**

删除第 16-28 行：

```typescript
  let replaceMode = $state(false);
  let selectedPreview = $state("");
  let closeTimeoutId: ReturnType<typeof setTimeout> | null = null;

  async function fetchReplaceModeState() {
    const state = await invokeWithError<{ replaceMode: boolean; selectedPreview: string }>("get_replace_mode_state");
    if (state) {
      replaceMode = state.replaceMode;
      selectedPreview = state.selectedPreview;
    }
  }

  fetchReplaceModeState();
```

替换为：

```typescript
  let closeTimeoutId: ReturnType<typeof setTimeout> | null = null;
```

同时移除文件顶部不再需要的 `invokeWithError` import：

将第 6 行 `import { invokeWithError } from '$lib/ai/shared';` 删除（如果没有其他地方使用的话，需检查文件中是否还有其他 `invokeWithError` 调用）。

- [ ] **Step 2: 移除 indicator:recording 监听中的 replaceMode 重置**

将第 63-68 行从：

```typescript
        await listen("indicator:recording", () => {
          phase = "recording";
          visible = true;
          replaceMode = false;
          selectedPreview = "";
          startTimer();
        }),
```

改为：

```typescript
        await listen("indicator:recording", () => {
          phase = "recording";
          visible = true;
          startTimer();
        }),
```

- [ ] **Step 3: 简化 style 属性，移除橙色分支**

将第 93 行从：

```svelte
    style="--accent-color: {replaceMode ? '#f59e0b' : '#ff0055'}; --accent-shadow: {replaceMode ? 'rgba(245, 158, 11, 0.6)' : 'rgba(255, 0, 85, 0.6)'}; --accent-text-shadow: {replaceMode ? 'rgba(245, 158, 11, 0.4)' : 'rgba(255, 0, 85, 0.4)'}"
```

改为：

```svelte
    style="--accent-color: #ff0055; --accent-shadow: rgba(255, 0, 85, 0.6); --accent-text-shadow: rgba(255, 0, 85, 0.4)"
```

- [ ] **Step 4: 验证前端构建**

Run: `npm run build 2>&1 | tail -20`（或项目对应的前端构建命令）
Expected: 构建成功，无类型错误。

- [ ] **Step 5: Commit**

```bash
git add src/routes/recording/+page.svelte
git commit -m "refactor: remove replace mode UI from recording indicator"
```

---

### Task 6: 最终验证

- [ ] **Step 1: 完整编译检查**

Run: `cd src-tauri && cargo check 2>&1`
Expected: 无错误。

- [ ] **Step 2: 运行所有 Rust 测试**

Run: `cd src-tauri && cargo test 2>&1`
Expected: 所有测试通过。

- [ ] **Step 3: 前端构建检查**

Run: `npm run build 2>&1`（或 `pnpm build`）
Expected: 构建成功。

- [ ] **Step 4: 搜索确认无残留引用**

搜索以下关键词确认无遗漏：
- `selected_text` / `selectedPreview` / `replaceMode` / `get_replace_mode_state` / `save_selected_text` / `get_saved_selected_text` / `detect_selected_text` / `SELECTED_TEXT`

Run: `rg "selected_text|selectedPreview|replaceMode|get_replace_mode_state|save_selected_text|get_saved_selected_text|detect_selected_text|SELECTED_TEXT" --type rust --type ts --type svelte`
Expected: 仅在设计文档中匹配，源代码无匹配。
