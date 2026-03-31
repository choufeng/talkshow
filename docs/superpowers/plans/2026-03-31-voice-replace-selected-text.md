# 语音替换选中文本 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用户选中文字后按下录音快捷键，Talkshow 自动进入替换模式，指示器变为琥珀色，选中的原文作为上下文传给 Skills。

**Architecture:** 在现有转写流水线中插入"选中文本检测"环节。录音启动时通过 AppleScript 获取选中文本并存入全局变量，传递给 Skills prompt 构造和前端指示器。粘贴逻辑无需改动（Cmd+V 天然替换选中内容）。

**Tech Stack:** Rust (Tauri), AppleScript (osascript), Svelte 5 (CSS 变量)

---

### Task 1: 新增选中文本检测函数

**Files:**
- Modify: `src-tauri/src/clipboard.rs`

- [ ] **Step 1: 在 `clipboard.rs` 中添加全局变量和函数**

在 `TARGET_APP` 全局变量下方添加 `SELECTED_TEXT` 全局变量，以及 `save_selected_text()` 和 `get_selected_text()` 两个函数：

```rust
static SELECTED_TEXT: Mutex<Option<String>> = Mutex::new(None);

pub fn save_selected_text(text: &str) {
    if let Ok(mut guard) = SELECTED_TEXT.lock() {
        *guard = Some(text.to_string());
    }
}

pub fn get_saved_selected_text() -> Option<String> {
    SELECTED_TEXT.lock().ok().and_then(|g| g.clone())
}

pub fn clear_selected_text() {
    if let Ok(mut guard) = SELECTED_TEXT.lock() {
        *guard = None;
    }
}
```

- [ ] **Step 2: 添加 macOS 上的 AppleScript 检测函数**

```rust
#[cfg(target_os = "macos")]
pub fn detect_selected_text(app_name: &str) -> Option<String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(format!("tell application \"{}\" to get selection", app_name))
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return None;
    }

    Some(text)
}

#[cfg(not(target_os = "macos"))]
pub fn detect_selected_text(_app_name: &str) -> Option<String> {
    None
}
```

- [ ] **Step 3: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 4: Commit**

```
git add src-tauri/src/clipboard.rs
git commit -m "feat: add selected text detection via AppleScript"
```

---

### Task 2: 录音启动时检测选中文本并通知前端

**Files:**
- Modify: `src-tauri/src/lib.rs` (约第1016-1027行，录音启动逻辑)

- [ ] **Step 1: 在录音启动逻辑中调用检测**

在 `lib.rs` 第1023行（`if let Some(ref app) = frontmost { clipboard::save_target_app(app); }` 之后），添加选中文本检测逻辑：

```rust
                                        let selected_text = if let Some(ref app) = frontmost {
                                            clipboard::detect_selected_text(app)
                                        } else {
                                            None
                                        };
                                        if let Some(ref text) = selected_text {
                                            clipboard::save_selected_text(text);
                                            let _ = app_handle.emit("indicator:replace-mode", serde_json::json!({
                                                "text": text.chars().take(50).collect::<String>()
                                            }));
                                            if let Some(logger) = app_handle.try_state::<Logger>() {
                                                logger.info("recording", "检测到选中文本，进入替换模式", Some(serde_json::json!({
                                                    "selected_length": text.len(),
                                                    "selected_preview": text.chars().take(100).collect::<String>()
                                                })));
                                            }
                                        }
```

- [ ] **Step 2: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 3: Commit**

```
git add src-tauri/src/lib.rs
git commit -m "feat: detect selected text on recording start and emit replace-mode event"
```

---

### Task 3: Skills prompt 增加替换上下文

**Files:**
- Modify: `src-tauri/src/skills.rs` (第45-70行 `assemble_skills_prompt`，第72-79行 `process_with_skills`)
- Modify: `src-tauri/src/lib.rs` (约第245-252行，调用 `process_with_skills` 的地方)

- [ ] **Step 1: 修改 `assemble_skills_prompt` 函数签名和逻辑**

将 `skills.rs` 中的 `assemble_skills_prompt` 修改为接受 `selected_text` 参数：

```rust
fn assemble_skills_prompt(
    skills: &[Skill],
    transcription: &str,
    app_name: &str,
    bundle_id: &str,
    selected_text: Option<&str>,
) -> (String, String) {
    let mut system_prompt = String::from(
        "你是一个语音转文字的文本处理助手。请根据以下规则处理用户的输入文本。\n\n",
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

    for skill in skills {
        system_prompt.push_str(&format!("\n---\n【{}】\n{}", skill.name, skill.prompt));
    }

    system_prompt.push_str("\n\n请只输出处理后的纯文本。不要添加任何解释、标注或前缀。");

    let user_message = transcription.to_string();

    (system_prompt, user_message)
}
```

- [ ] **Step 2: 修改 `process_with_skills` 函数签名**

在 `process_with_skills` 的参数列表中添加 `selected_text: Option<&str>`，并透传给 `assemble_skills_prompt`：

函数签名变为：

```rust
pub async fn process_with_skills(
    logger: &Logger,
    skills_config: &SkillsConfig,
    transcription_config: &crate::config::TranscriptionConfig,
    providers: &[ProviderConfig],
    transcription: &str,
    vertex_cache: &VertexClientCache,
    selected_text: Option<&str>,
) -> Result<String, String> {
```

调用处（约第130行）变为：

```rust
    let (system_prompt, user_message) =
        assemble_skills_prompt(&skills_owned, transcription, &app_name, &bundle_id, selected_text);
```

- [ ] **Step 3: 修改 `lib.rs` 中调用 `process_with_skills` 的地方**

在 `lib.rs` 约第245行，将调用改为传入选中文本：

```rust
                                let selected_text = clipboard::get_saved_selected_text();
                                let selected_text_ref = selected_text.as_deref();
                                let mut final_text = skills::process_with_skills(
                                    &logger,
                                    &skills_config,
                                    &app_config.features.transcription,
                                    &skills_providers,
                                    &text,
                                    &h.state::<VertexClientState>().client,
                                    selected_text_ref,
                                )
```

- [ ] **Step 4: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 5: Commit**

```
git add src-tauri/src/skills.rs src-tauri/src/lib.rs
git commit -m "feat: pass selected text context to Skills for replace mode"
```

---

### Task 4: 前端指示器 — 替换模式颜色 + 调试文本显示

**Files:**
- Modify: `src/routes/recording/+page.svelte`

- [ ] **Step 1: 添加状态和事件监听**

在 `<script>` 中添加 `replaceMode` 和 `selectedPreview` 状态，以及事件监听：

在第10行 `let visible = $state(true);` 之后添加：

```typescript
  let replaceMode = $state(false);
  let selectedPreview = $state("");
```

在 `indicator:recording` 监听器中（约第54-58行），添加替换模式的重置：

```typescript
      unsubs.push(
        await listen("indicator:recording", () => {
          phase = "recording";
          visible = true;
          replaceMode = false;
          selectedPreview = "";
          startTimer();
        }),
      );
```

在 `indicator:processing` 监听器之后，添加 replace-mode 监听：

```typescript
      unsubs.push(
        await listen<{ text: string }>("indicator:replace-mode", (event) => {
          replaceMode = true;
          selectedPreview = event.payload.text;
        }),
      );
```

- [ ] **Step 2: 用 CSS 变量控制录音阶段颜色**

将录音阶段 SVG 和文字的颜色从硬编码 `#ff0055` 改为 CSS 变量。在 `<div class="indicator">` 上根据 `replaceMode` 设置 CSS 变量：

```svelte
  <div
    class="indicator"
    class:processing={phase === "processing"}
    class:fade-out={!visible}
    style="--accent-color: {replaceMode ? '#f59e0b' : '#ff0055'}; --accent-shadow: {replaceMode ? 'rgba(245, 158, 11, 0.6)' : 'rgba(255, 0, 85, 0.6)'}; --accent-text-shadow: {replaceMode ? 'rgba(245, 158, 11, 0.4)' : 'rgba(255, 0, 85, 0.4)'}"
  >
```

- [ ] **Step 3: 将录音阶段 SVG 颜色改为 CSS 变量引用**

将第84-88行的硬编码颜色替换：

```svelte
      <svg class="neon-ring" width="24" height="24" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="8" fill="none" stroke="var(--accent-color)" stroke-width="1.5">
          <animate attributeName="r" values="7;9;7" dur="1.2s" repeatCount="indefinite"/>
          <animate attributeName="opacity" values="1;0.5;1" dur="1.2s" repeatCount="indefinite"/>
        </circle>
        <circle cx="12" cy="12" r="3" fill="var(--accent-color)"/>
      </svg>
```

- [ ] **Step 4: 将 `.neon-ring`、`.timer`、`.rec-label` 的 CSS 改为使用变量**

```css
  .neon-ring {
    filter: drop-shadow(0 0 4px var(--accent-shadow));
    flex-shrink: 0;
  }

  .timer {
    font-family: -apple-system, BlinkMacSystemFont, "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 13px;
    color: var(--accent-color);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    min-width: 36px;
    text-align: center;
    text-shadow: 0 0 6px var(--accent-text-shadow);
  }

  .rec-label {
    font-family: -apple-system, BlinkMacSystemFont, "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 8px;
    color: var(--accent-text-shadow);
    letter-spacing: 2px;
  }
```

- [ ] **Step 5: 添加调试文本显示**

在录音阶段 `</div>` 之前（`<span class="rec-label">REC</span>` 后面），添加调试用的选中内容预览：

```svelte
      {#if replaceMode && selectedPreview}
        <span class="selected-preview">{selectedPreview}</span>
      {/if}
```

添加对应 CSS：

```css
  .selected-preview {
    font-family: -apple-system, BlinkMacSystemFont, "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 9px;
    color: var(--accent-text-shadow);
    max-width: 40px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex-shrink: 1;
  }
```

注意：这是临时调试信息，正式发布前需移除。

- [ ] **Step 6: 验证编译**

Run: `npm run check`（或项目中的等效命令）
Expected: 无类型错误

- [ ] **Step 7: Commit**

```
git add src/routes/recording/+page.svelte
git commit -m "feat: indicator amber color for replace mode with debug text preview"
```

---

### Task 5: 端到端验证

- [ ] **Step 1: 启动应用并手动测试**

1. `cargo tauri dev`
2. 打开任意文本编辑器（如 Notes），输入一些文字
3. 选中其中几个字
4. 按下录音快捷键（默认 `Control+\`）
5. 验证：
   - 指示器变为琥珀色
   - 指示器上显示选中文本的预览（调试信息）
   - 日志中输出"检测到选中文本，进入替换模式"
6. 说一段话，再次按下快捷键停止
7. 验证：选中的文字被转写结果替换

6. 不选中任何文字，按下录音快捷键
7. 验证：指示器保持红色（普通模式）

- [ ] **Step 2: 清理调试信息（可选，如果验证通过）**

如果一切正常，可以移除 `selectedPreview` 状态和 `.selected-preview` 元素，或者保留到正式发布前再清理。
