# AI 翻译功能实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 TalkShow 新增 AI 翻译功能，用户通过独立的翻译快捷键 (Ctrl+Shift+T) 触发录音，录音内容经转录和润色后自动翻译为目标语言。

**Architecture:** 在现有流水线（转录 → 润色 → 粘贴）基础上，当录音由翻译快捷键触发时，在润色后增加独立的 LLM 翻译步骤。翻译使用复用润色模型，通过内置可编辑的翻译 Skill 提供自定义提示词。录音模式通过 `AtomicU8` 枚举追踪。

**Tech Stack:** Rust (Tauri v2, rig, tokio), TypeScript (SvelteKit 5, Svelte runes)

---

## File Structure

| Action | File | Responsibility |
|---|---|---|
| Modify | `src-tauri/src/config.rs` | 新增 `TranslationConfig`、`translate_shortcut`、内置翻译 Skill、迁移逻辑 |
| Modify | `src-tauri/src/lib.rs` | `RECORDING` 改为 `AtomicU8`、新增翻译快捷键注册、流水线中插入翻译步骤 |
| Create | `src-tauri/src/translation.rs` | 翻译 LLM 调用、prompt 组装逻辑 |
| Modify | `src/lib/stores/config.ts` | 新增 `TranslationConfig`、`translate_shortcut`、语言常量 |
| Modify | `src/routes/settings/+page.svelte` | 新增翻译快捷键配置 UI |
| Modify | `src/routes/models/+page.svelte` | 新增目标语言选择 UI |
| Modify | `src/routes/skills/+page.svelte` | 处理 `builtin-translation` 可编辑标记 |

---

### Task 1: 配置层 — 后端 Rust

**Files:**
- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: 新增 TranslationConfig 结构体**

在 `SkillsConfig` 的 `impl Default` 块之后、`FeaturesConfig` 之前，新增：

```rust
const DEFAULT_TRANSLATE_SHORTCUT: &str = "Control+Shift+T";

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct TranslationConfig {
    pub target_lang: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub builtin: bool,
    pub editable: bool,
    pub enabled: bool,
}
```

注意：`Skill` 结构体新增了 `editable: bool` 字段。

- [ ] **Step 2: 更新 Skill 默认值，新增 builtin-translation**

在 `SkillsConfig` 的 `impl Default` 中，给所有现有 Skill 的 `editable` 设为 `false`，并在末尾新增 `builtin-translation`：

```rust
impl Default for SkillsConfig {
    fn default() -> Self {
        SkillsConfig {
            enabled: true,
            skills: vec![
                Skill {
                    id: "builtin-fillers".to_string(),
                    name: "语气词剔除".to_string(),
                    description: "去除嗯、啊、那个、就是等无意义口头语气词".to_string(),
                    prompt: "去除中文口语中常见的无意义语气词和填充词，包括但不限于：\n\"嗯\"、\"啊\"、\"额\"、\"呃\"、\"那个\"、\"就是\"、\"然后\"、\"对吧\"、\"的话\"、\"怎么说呢\"。\n注意保留有实际语义的词语，例如\"然后\"在表示时间顺序时应保留。不要改变原文的语义和语气。".to_string(),
                    builtin: true,
                    editable: false,
                    enabled: true,
                },
                Skill {
                    id: "builtin-typos".to_string(),
                    name: "错别字修正".to_string(),
                    description: "修正错别字、同音错误和输入法错误".to_string(),
                    prompt: "识别并修正文本中的错别字、同音错误和常见输入法导致的文字错误。\n只修正明确的错误，不要对有歧义的内容做主观改动。\n常见的同音错误示例：\"的/地/得\"、\"做/作\"、\"在/再\"、\"已/以\"、\"即/既\"。".to_string(),
                    builtin: true,
                    editable: false,
                    enabled: true,
                },
                Skill {
                    id: "builtin-polish".to_string(),
                    name: "口语润色".to_string(),
                    description: "保持口语化风格，使表达更流畅自然".to_string(),
                    prompt: "保持口语化的表达风格，但使语句更流畅自然。\n具体做法：去除重复表达、调整语序使其更通顺、适当添加标点使句子结构更清晰。\n不要改变原文的口语化特征，不要转换为书面语。".to_string(),
                    builtin: true,
                    editable: false,
                    enabled: false,
                },
                Skill {
                    id: "builtin-formal".to_string(),
                    name: "书面格式化".to_string(),
                    description: "口语转书面表达，适合邮件和文档场景".to_string(),
                    prompt: "将口语化的表达转换为规范的书面表达，适合邮件、文档、报告等正式场景。\n\n具体做法：\n- 词汇替换：将口语化词汇替换为正式表达（如"搞定了"→"已完成"）\n- 列表结构化：将"第一/第二/第三"、"首先/其次/最后"、"一二三"等序列词转换为规范的有序列表格式\n- 段落重组：识别话题转换，合理分段；将碎片化短句合并为完整句子\n- 标点规范：统一使用全角标点，消除重复标点，合理使用冒号、分号等结构化标点\n- 句子结构：调整语序使其符合书面语法，消除冗余和重复表达\n- 层级关系：识别"总-分"、因果、递进等逻辑关系，用合适的连接词明确表达\n\n约束：\n- 保持原文的完整语义，不添加或删除信息\n- 输出纯文本，可使用 Markdown 列表格式\n- 不要添加解释性文字".to_string(),
                    builtin: true,
                    editable: false,
                    enabled: false,
                },
                Skill {
                    id: "builtin-translation".to_string(),
                    name: "翻译优化".to_string(),
                    description: "自定义翻译规则，如术语、风格和行业特定要求".to_string(),
                    prompt: "保持原文的语气和风格。确保技术术语翻译准确。如果某个术语没有标准翻译，保留原文。".to_string(),
                    builtin: true,
                    editable: true,
                    enabled: true,
                },
            ],
        }
    }
```

- [ ] **Step 3: 更新 FeaturesConfig 和 AppConfig**

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct FeaturesConfig {
    pub transcription: TranscriptionConfig,
    pub translation: TranslationConfig,
    pub skills: SkillsConfig,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub shortcut: String,
    pub recording_shortcut: String,
    pub translate_shortcut: String,
    pub ai: AiConfig,
    pub features: FeaturesConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            shortcut: DEFAULT_SHORTCUT.to_string(),
            recording_shortcut: DEFAULT_RECORDING_SHORTCUT.to_string(),
            translate_shortcut: DEFAULT_TRANSLATE_SHORTCUT.to_string(),
            ai: AiConfig {
                providers: builtin_providers(),
            },
            features: FeaturesConfig {
                transcription: TranscriptionConfig {
                    provider_id: "vertex".to_string(),
                    model: "gemini-2.0-flash".to_string(),
                    polish_enabled: true,
                    polish_provider_id: String::new(),
                    polish_model: String::new(),
                },
                translation: TranslationConfig {
                    target_lang: "English".to_string(),
                },
                skills: SkillsConfig::default(),
            },
        }
    }
}
```

- [ ] **Step 4: 更新 migrate_builtin_skills 函数**

在 `migrate_builtin_skills` 函数中，对 `builtin-translation` 跳过 prompt 强制同步（因为它是 editable 的）：

```rust
fn migrate_builtin_skills(value: &mut serde_json::Value) {
    if let Some(skills) = value
        .get_mut("features")
        .and_then(|f| f.get_mut("skills"))
        .and_then(|s| s.get_mut("skills"))
        .and_then(|s| s.as_array_mut())
    {
        let default_skills = SkillsConfig::default().skills;
        for skill in skills.iter_mut() {
            if let Some(id) = skill.get("id").and_then(|v| v.as_str()) {
                if let Some(builtin) = skill.get("builtin").and_then(|v| v.as_bool()) {
                    if builtin {
                        if let Some(default) = default_skills.iter().find(|s| s.id == id) {
                            if let Some(editable) = skill.get("editable").and_then(|v| v.as_bool()) {
                                if editable {
                                    continue;
                                }
                            }
                            if let Some(current_prompt) =
                                skill.get("prompt").and_then(|v| v.as_str())
                            {
                                if current_prompt != default.prompt {
                                    *skill.get_mut("prompt").unwrap() =
                                        serde_json::json!(default.prompt);
                                }
                            }
                        }
                    }
                }
            }
        }

        let builtin_ids: std::collections::HashSet<String> = skills
            .iter()
            .filter_map(|s| {
                if s.get("builtin").and_then(|v| v.as_bool()).unwrap_or(false) {
                    s.get("id").and_then(|v| v.as_str()).map(String::from)
                } else {
                    None
                }
            })
            .collect();

        for default in &default_skills {
            if !builtin_ids.contains(&default.id) {
                skills.push(serde_json::to_value(default).unwrap_or_default());
            }
        }
    }
}
```

- [ ] **Step 5: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 编译通过（注意：`lib.rs` 中引用了 `config::Skill` 的 `builtin` 字段，新增的 `editable` 字段由于有 `#[serde(default)]` 所以向后兼容）

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "feat(config): add TranslationConfig, builtin-translation skill, and editable field"
```

---

### Task 2: 翻译模块 — 后端 Rust

**Files:**
- Create: `src-tauri/src/translation.rs`

- [ ] **Step 1: 创建 translation.rs**

```rust
use crate::config::{ProviderConfig, Skill, SkillsConfig};
use crate::logger::Logger;
use std::sync::{Arc, Mutex};

type VertexClientCache = Arc<Mutex<Option<rig_vertexai::Client>>>;

const TRANSLATION_TIMEOUT_SECS: u64 = 15;

const TRANSLATION_BASE_PROMPT: &str = "You are a professional translator. Translate the following text to {target_lang}. Output only the translation, nothing else.";

fn get_translation_skill_prompt(skills_config: &SkillsConfig) -> Option<String> {
    skills_config
        .skills
        .iter()
        .find(|s| s.id == "builtin-translation" && s.enabled)
        .map(|s| s.prompt.clone())
}

pub async fn translate_text(
    logger: &Logger,
    text: &str,
    target_lang: &str,
    skills_config: &SkillsConfig,
    provider_id: &str,
    model_name: &str,
    providers: &[ProviderConfig],
    vertex_cache: &VertexClientCache,
) -> Result<String, String> {
    let provider = providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Translation provider not found: {}", provider_id))?;

    let mut system_prompt = TRANSLATION_BASE_PROMPT.replace("{target_lang}", target_lang);

    if let Some(skill_prompt) = get_translation_skill_prompt(skills_config) {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&skill_prompt);
    }

    let full_prompt = format!("{}\n\n{}", system_prompt, text);

    logger.info(
        "translation",
        "翻译开始",
        Some(serde_json::json!({
            "target_lang": target_lang,
            "provider_id": provider_id,
            "model": model_name,
            "text_length": text.len(),
            "text_preview": text.chars().take(50).collect::<String>(),
        })),
    );

    let start = std::time::Instant::now();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(TRANSLATION_TIMEOUT_SECS),
        crate::ai::send_text_prompt(
            logger,
            &full_prompt,
            model_name,
            provider,
            vertex_cache,
            crate::ai::ThinkingMode::Disabled,
        ),
    )
    .await;

    let elapsed_ms = start.elapsed().as_millis();

    match result {
        Ok(Ok(translated)) => {
            logger.info(
                "translation",
                "翻译成功",
                Some(serde_json::json!({
                    "elapsed_ms": elapsed_ms,
                    "original_length": text.len(),
                    "translated_length": translated.len(),
                    "translated_preview": translated.chars().take(50).collect::<String>(),
                })),
            );
            Ok(translated)
        }
        Ok(Err(e)) => {
            logger.error(
                "translation",
                "翻译失败",
                Some(serde_json::json!({
                    "elapsed_ms": elapsed_ms,
                    "error": e.to_string(),
                })),
            );
            Err(format!("翻译失败: {}", e))
        }
        Err(_) => {
            logger.error(
                "translation",
                "翻译超时",
                Some(serde_json::json!({
                    "elapsed_ms": elapsed_ms,
                    "timeout_secs": TRANSLATION_TIMEOUT_SECS,
                })),
            );
            Err(format!("翻译超时 ({}s)", TRANSLATION_TIMEOUT_SECS))
        }
    }
}
```

- [ ] **Step 2: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 编译通过（`translation` 模块尚未被 `lib.rs` 引用，但自身可以编译）

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/translation.rs
git commit -m "feat(translation): add translation module with LLM call and prompt assembly"
```

---

### Task 3: 录音模式追踪与快捷键注册 — 后端 lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 新增 mod translation 并修改 RECORDING 为 AtomicU8**

在文件顶部，将 `use std::sync::atomic::{AtomicBool, Ordering};` 改为：

```rust
use std::sync::atomic::{AtomicU8, Ordering};
```

新增模块声明：

```rust
mod translation;
```

将 `RECORDING` 从 `AtomicBool` 改为 `AtomicU8`：

```rust
const RECORDING_MODE_NONE: u8 = 0;
const RECORDING_MODE_TRANSCRIPTION: u8 = 1;
const RECORDING_MODE_TRANSLATION: u8 = 2;

static RECORDING: AtomicU8 = AtomicU8::new(RECORDING_MODE_NONE);
```

- [ ] **Step 2: 更新 ShortcutIds 增加 translate 字段**

```rust
struct ShortcutIds {
    toggle: u32,
    recording: u32,
    translate: u32,
}

static SHORTCUT_IDS: RwLock<ShortcutIds> = RwLock::new(ShortcutIds {
    toggle: 0,
    recording: 0,
    translate: 0,
});
```

- [ ] **Step 3: 更新所有 RECORDING.load() 调用**

将所有 `RECORDING.load(Ordering::Relaxed)` 调用从布尔检查改为：

- `RECORDING.load(Ordering::Relaxed) != RECORDING_MODE_NONE` （用于"是否在录音"检查）
- `RECORDING.load(Ordering::Relaxed) == RECORDING_MODE_NONE` （用于"是否不在录音"检查）

将所有 `RECORDING.store(true/false, ...)` 调用改为存储对应模式值或 `RECORDING_MODE_NONE`。

具体需要修改的位置（按行号参考，实际以代码为准）：

1. `stop_recording` 函数第一行：
```rust
let recording_mode = RECORDING.load(Ordering::Relaxed);
RECORDING.store(RECORDING_MODE_NONE, Ordering::Relaxed);
```

2. `stop_recording` 内部检查 `RECORDING.load()` 的地方：
```rust
// 原来是 !RECORDING.load(Ordering::Relaxed)
// 改为：
RECORDING.load(Ordering::Relaxed) == RECORDING_MODE_NONE
```

3. `indicator:cancel` 事件监听中：
```rust
let is_recording = RECORDING.load(Ordering::Relaxed) != RECORDING_MODE_NONE;
```

4. 全局快捷键 handler 中所有 `RECORDING.load()` / `RECORDING.store()` 的地方。

5. `RECORDING.store(true, Ordering::Relaxed)` 改为 `RECORDING.store(RECORDING_MODE_TRANSCRIPTION, Ordering::Relaxed)` 或 `RECORDING.store(RECORDING_MODE_TRANSLATION, Ordering::Relaxed)`（根据触发快捷键决定）。

- [ ] **Step 4: 修改 stop_recording 函数签名，增加 recording_mode 参数**

```rust
fn stop_recording(
    app_handle: &tauri::AppHandle,
    recorder: &Arc<std::sync::Mutex<AudioRecorder>>,
    recording_start: &Arc<std::sync::Mutex<Option<Instant>>>,
    event_name: &str,
    recording_mode: u8,
)
```

在 `stop_recording` 函数开头删除 `RECORDING.store(...)` 行（因为调用者已经通过参数传入模式）。但保留 `recording_mode` 参数传递到下游流水线中。

- [ ] **Step 5: 在流水线中插入翻译步骤**

在 `stop_recording` 函数的异步任务中，在 Skills 润色之后、剪贴板写入之前，插入翻译步骤。找到以下代码位置（在 `let skills_elapsed = skills_start.elapsed().as_millis();` 之后、`if CANCELLED.load(...)` 之前）：

```rust
                                let skills_elapsed = skills_start.elapsed().as_millis();

                                // --- Translation step ---
                                let final_text = if recording_mode == RECORDING_MODE_TRANSLATION {
                                    if transcription.polish_enabled
                                        && !transcription.polish_provider_id.is_empty()
                                        && !transcription.polish_model.is_empty()
                                    {
                                        let translate_config = app_config.features.translation.clone();
                                        match translation::translate_text(
                                            &logger,
                                            &final_text,
                                            &translate_config.target_lang,
                                            &skills_config,
                                            &transcription.polish_provider_id,
                                            &transcription.polish_model,
                                            &skills_providers,
                                            &h.state::<VertexClientState>().client,
                                        )
                                        .await
                                        {
                                            Ok(translated) => translated,
                                            Err(e) => {
                                                logger.error("translation", &e, None);
                                                show_notification(&h, "翻译失败", &e);
                                                destroy_indicator(&h);
                                                if RECORDING.load(Ordering::Relaxed) == RECORDING_MODE_NONE {
                                                    let _ = h.global_shortcut().unregister(Shortcut::new(None, Code::Escape));
                                                }
                                                return;
                                            }
                                        }
                                    } else {
                                        show_notification(&h, "翻译失败", "请先启用润色并配置润色模型");
                                        destroy_indicator(&h);
                                        if RECORDING.load(Ordering::Relaxed) == RECORDING_MODE_NONE {
                                            let _ = h.global_shortcut().unregister(Shortcut::new(None, Code::Escape));
                                        }
                                        return;
                                    }
                                } else {
                                    final_text
                                };
```

注意：需要将 Skills 润色结果的变量名改为可变的（`let final_text` → 先赋值为润色结果，然后翻译步骤可能覆盖）。具体做法是在 Skills 处理后：

```rust
                                let mut final_text = skills::process_with_skills(...)
                                    .await
                                    .unwrap_or_else(|e| { ... text });

                                // Translation step (override final_text if translation mode)
                                if recording_mode == RECORDING_MODE_TRANSLATION {
                                    // ... translation logic that assigns to final_text
                                }
```

- [ ] **Step 6: 更新所有 stop_recording 调用点**

所有调用 `stop_recording` 的地方需要传入当前录音模式：

1. `indicator:cancel` 事件：
```rust
let mode = RECORDING.load(Ordering::Relaxed);
stop_recording(&app_handle_cancel, &recorder_cancel, &recording_start_cancel, "recording:cancel", mode);
```

2. ESC handler 中（录音中取消）：
```rust
let mode = RECORDING.load(Ordering::Relaxed);
stop_recording(&app_handle, &recorder_handler, &recording_start_handler, "recording:cancel", mode);
```

3. 录音快捷键 handler 中（结束录音）：
```rust
let mode = RECORDING.load(Ordering::Relaxed);
stop_recording(&app_handle, &recorder_handler, &recording_start_handler, "recording:complete", mode);
```

- [ ] **Step 7: 新增翻译快捷键注册**

在 `setup` 闭包中，加载 `translate_shortcut_str`：

```rust
let translate_shortcut_str = app_config.translate_shortcut.clone();
```

解析并注册：

```rust
let translate_shortcut = parse_shortcut(&translate_shortcut_str);
let translate_id = translate_shortcut.as_ref().map(|s| s.id()).unwrap_or(0);
```

在 `SHORTCUT_IDS` 写入中增加：

```rust
{
    let mut ids = SHORTCUT_IDS.write().unwrap();
    ids.toggle = toggle_id;
    ids.recording = rec_id;
    ids.translate = translate_id;
}
```

在全局快捷键 handler 中，读取 translate_id：

```rust
let (_current_toggle_id, current_rec_id, current_translate_id) = {
    let ids = SHORTCUT_IDS.read().unwrap();
    (ids.toggle, ids.recording, ids.translate)
};
```

在录音快捷键处理逻辑之后、toggle 窗口逻辑之前，新增翻译快捷键处理：

```rust
                        if current_translate_id != 0 && id == current_translate_id {
                            let now = Instant::now();
                            let should_ignore = LAST_REC_PRESS
                                .lock()
                                .ok()
                                .and_then(|mut last| {
                                    if let Some(t) = *last {
                                        if now.duration_since(t) < Duration::from_millis(500) {
                                            return Some(true);
                                        }
                                    }
                                    *last = Some(now);
                                    Some(false)
                                })
                                .unwrap_or(false);
                            if should_ignore {
                                return;
                            }
                            let is_recording = RECORDING.load(Ordering::Relaxed) != RECORDING_MODE_NONE;
                            if is_recording {
                                let mode = RECORDING.load(Ordering::Relaxed);
                                stop_recording(
                                    &app_handle,
                                    &recorder_handler,
                                    &recording_start_handler,
                                    "recording:complete",
                                    mode,
                                );
                                play_sound("Submarine.aiff");
                                restore_default_tray(&app_handle, default_icon_owned.clone());
                            } else {
                                let start_result =
                                    recorder_handler.lock().ok().and_then(|mut r| {
                                        let result = r.start();
                                        if result.is_ok() {
                                            Some(())
                                        } else {
                                            None
                                        }
                                    });
                                match start_result {
                                    Some(()) => {
                                        RECORDING.store(RECORDING_MODE_TRANSLATION, Ordering::Relaxed);
                                        if let Ok(mut start) = recording_start_handler.lock() {
                                            *start = Some(Instant::now());
                                        }
                                        if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
                                            let _ =
                                                tray.set_icon(Some(recording_icon_owned.clone()));
                                        }
                                        let frontmost = std::process::Command::new("osascript")
                                            .arg("-e")
                                            .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
                                            .output()
                                            .ok()
                                            .filter(|o| o.status.success())
                                            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
                                        if let Some(ref app) = frontmost {
                                            clipboard::save_target_app(app);
                                        }
                                        play_sound("Ping.aiff");
                                        show_indicator(&app_handle);
                                        if let Some(mw) = app_handle.get_webview_window("main") {
                                            if mw.is_visible().unwrap_or(false) {
                                                let _ = mw.hide();
                                            }
                                        }
                                        if let Some(logger) = app_handle.try_state::<Logger>() {
                                            logger.info("recording", "录音开始 (翻译模式)", None);
                                        }
                                        let h = app_handle.clone();
                                        let esc = esc_shortcut_handler.clone();
                                        std::thread::spawn(move || {
                                            let _ = h.global_shortcut().register(esc);
                                        });
                                    }
                                    None => {
                                        let err_detail = recorder_handler
                                            .lock()
                                            .ok()
                                            .and_then(|mut r| r.start().err())
                                            .map(|e| e.to_string())
                                            .unwrap_or_else(|| "Unknown error".into());
                                        eprintln!("Failed to start recording: {}", err_detail);
                                        if let Some(logger) = app_handle.try_state::<Logger>() {
                                            logger.error("recording", "录音启动失败", Some(serde_json::json!({ "error": err_detail })));
                                        }
                                        show_notification(&app_handle, "录音失败", &err_detail);
                                    }
                                }
                            }
                            return;
                        }
```

在 setup 末尾注册翻译快捷键：

```rust
if let Some(sc) = translate_shortcut {
    if let Err(e) = app.global_shortcut().register(sc) {
        eprintln!("Failed to register translate shortcut: {}", e);
    }
}
```

- [ ] **Step 8: 更新 update_shortcut 命令支持 translate 类型**

在 `update_shortcut` 函数中：

```rust
#[tauri::command]
fn update_shortcut(
    app_handle: tauri::AppHandle,
    shortcut_type: String,
    shortcut: String,
) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);

    let old_toggle = app_config.shortcut.clone();
    let old_recording = app_config.recording_shortcut.clone();
    let old_translate = app_config.translate_shortcut.clone();

    match shortcut_type.as_str() {
        "toggle" => app_config.shortcut = shortcut,
        "recording" => app_config.recording_shortcut = shortcut,
        "translate" => app_config.translate_shortcut = shortcut,
        _ => return Err("Invalid shortcut type".to_string()),
    }

    config::save_config(&app_data_dir, &app_config)?;

    if let Some(sc) = parse_shortcut(&old_toggle) {
        let _ = app_handle.global_shortcut().unregister(sc);
    }
    if let Some(sc) = parse_shortcut(&old_recording) {
        let _ = app_handle.global_shortcut().unregister(sc);
    }
    if let Some(sc) = parse_shortcut(&old_translate) {
        let _ = app_handle.global_shortcut().unregister(sc);
    }

    let new_toggle = parse_shortcut(&app_config.shortcut);
    let new_rec = parse_shortcut(&app_config.recording_shortcut);
    let new_translate = parse_shortcut(&app_config.translate_shortcut);

    {
        let mut ids = SHORTCUT_IDS.write().unwrap();
        ids.toggle = new_toggle.map(|s| s.id()).unwrap_or(0);
        ids.recording = new_rec.map(|s| s.id()).unwrap_or(0);
        ids.translate = new_translate.map(|s| s.id()).unwrap_or(0);
    }

    if let Some(sc) = new_toggle {
        app_handle
            .global_shortcut()
            .register(sc)
            .map_err(|e| e.to_string())?;
    }
    if let Some(sc) = new_rec {
        app_handle
            .global_shortcut()
            .register(sc)
            .map_err(|e| e.to_string())?;
    }
    if let Some(sc) = new_translate {
        app_handle
            .global_shortcut()
            .register(sc)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
```

- [ ] **Step 9: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/translation.rs
git commit -m "feat(backend): add translation shortcut, recording mode tracking, and translation pipeline step"
```

---

### Task 4: 前端配置层

**Files:**
- Modify: `src/lib/stores/config.ts`

- [ ] **Step 1: 新增 TranslationConfig 接口和语言常量**

在 `SkillsConfig` 接口之后新增：

```typescript
export interface TranslationConfig {
  target_lang: string;
}

export const TRANSLATE_LANGUAGES = [
  'English',
  '中文',
  '日本語',
  '한국어',
  'Français',
  'Deutsch',
  'Español',
  'Português',
  'Italiano',
  'Русский',
  'العربية',
  'हिन्दी',
];
```

- [ ] **Step 2: 更新 Skill 接口**

```typescript
export interface Skill {
  id: string;
  name: string;
  description: string;
  prompt: string;
  builtin: boolean;
  editable: boolean;
  enabled: boolean;
}
```

- [ ] **Step 3: 更新 FeaturesConfig 和 AppConfig**

```typescript
export interface FeaturesConfig {
  transcription: TranscriptionConfig;
  translation: TranslationConfig;
  skills: SkillsConfig;
}

export interface AppConfig {
  shortcut: string;
  recording_shortcut: string;
  translate_shortcut: string;
  ai: AiConfig;
  features: FeaturesConfig;
}
```

- [ ] **Step 4: 更新 createConfigStore 默认值**

```typescript
function createConfigStore() {
  const { subscribe, set, update } = writable<AppConfig>({
    shortcut: 'Control+Shift+Quote',
    recording_shortcut: 'Control+Backslash',
    translate_shortcut: 'Control+Shift+T',
    ai: {
      providers: BUILTIN_PROVIDERS.map((p) => ({ ...p }))
    },
    features: {
      transcription: {
        provider_id: 'vertex',
        model: 'gemini-2.0-flash',
        polish_enabled: true,
        polish_provider_id: '',
        polish_model: ''
      },
      translation: {
        target_lang: 'English'
      },
      skills: {
        enabled: true,
        skills: []
      }
    }
  });
```

- [ ] **Step 5: 更新 updateShortcut 支持 translate 类型**

```typescript
    updateShortcut: async (type: 'toggle' | 'recording' | 'translate', shortcut: string) => {
      try {
        await invoke('update_shortcut', { shortcutType: type, shortcut });
        update(config => {
          if (type === 'toggle') {
            return { ...config, shortcut };
          } else if (type === 'recording') {
            return { ...config, recording_shortcut: shortcut };
          } else {
            return { ...config, translate_shortcut: shortcut };
          }
        });
      } catch (error) {
        console.error('Failed to update shortcut:', error);
        throw error;
      }
    },
```

- [ ] **Step 6: Commit**

```bash
git add src/lib/stores/config.ts
git commit -m "feat(frontend): add TranslationConfig, translate_shortcut, and editable field to config store"
```

---

### Task 5: 设置页面 — 翻译快捷键 UI

**Files:**
- Modify: `src/routes/settings/+page.svelte`

- [ ] **Step 1: 新增翻译快捷键处理函数**

在 `handleUpdateRecording` 之后新增：

```typescript
  async function handleUpdateTranslate(shortcut: string) {
    await config.updateShortcut('translate', shortcut);
  }
```

- [ ] **Step 2: 在录音快捷键之后新增翻译快捷键 UI**

在录音控制 `ShortcutRecorder` 之后、提示区块之前新增：

```svelte
    <ShortcutRecorder
      label="AI 翻译"
      description="录音并翻译为目标语言"
      value={$config.translate_shortcut}
      onUpdate={handleUpdateTranslate}
    />
```

- [ ] **Step 3: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(settings): add translate shortcut configuration UI"
```

---

### Task 6: 模型页面 — 目标语言选择 UI

**Files:**
- Modify: `src/routes/models/+page.svelte`

- [ ] **Step 1: 导入 TRANSLATE_LANGUAGES**

在 import 中新增：

```typescript
import { config, isBuiltinProvider, BUILTIN_PROVIDERS, MODEL_CAPABILITIES, TRANSLATE_LANGUAGES } from '$lib/stores/config';
```

- [ ] **Step 2: 新增目标语言变更处理函数**

在 `handlePolishEnabled` 函数之后新增：

```typescript
  function handleTargetLangChange(lang: string) {
    const newConfig: AppConfig = {
      ...$config,
      features: {
        ...$config.features,
        translation: { target_lang: lang }
      }
    };
    config.save(newConfig);
  }
```

- [ ] **Step 3: 在润色模型之后新增翻译配置 UI**

在 `{#if $config.features.transcription.polish_enabled}` 块的 `{/if}` 之后、`</div>` 闭合之前（在 `转写服务` 区块的 `flex flex-col gap-5` 容器内），新增：

```svelte
        <div>
          <div class="text-xs text-muted-foreground uppercase tracking-wider mb-3 mt-5">AI 翻译</div>
          <div>
            <label class="block text-sm text-foreground-alt mb-1.5">目标语言</label>
            <select
              class="flex h-9 w-full rounded-md border border-border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20"
              value={$config.features.translation?.target_lang || 'English'}
              onchange={(e) => handleTargetLangChange((e.target as HTMLSelectElement).value)}
            >
              {#each TRANSLATE_LANGUAGES as lang}
                <option value={lang}>{lang}</option>
              {/each}
            </select>
          </div>
          <div class="text-xs text-muted-foreground mt-1.5">
            翻译模型复用润色模型：{$config.features.transcription.polish_model || '未配置'}
          </div>
        </div>
```

- [ ] **Step 4: Commit**

```bash
git add src/routes/models/+page.svelte
git commit -m "feat(models): add target language selector for AI translation"
```

---

### Task 7: Skills 页面 — 支持可编辑内置 Skill

**Files:**
- Modify: `src/routes/skills/+page.svelte`

- [ ] **Step 1: 允许编辑 builtin 但 editable 为 true 的 Skill**

在 Skills 列表中，编辑按钮的显示条件从 `onclick={() => openEditDialog(skill)}` 无条件显示（已有逻辑），但确认删除按钮的 `{:else}` 条件改为同时检查 `!skill.builtin || skill.editable`：

实际上，当前代码中编辑按钮对所有 Skill 都显示了，这已经可以工作。需要确保的是：

1. 编辑保存时，`builtin-translation` 的 `editable` 字段被保留。
2. 删除按钮仍然只对非 builtin 显示（即 `builtin-translation` 不可删除，但可编辑）。

当前代码逻辑已经满足这些需求，不需要修改。但需要确认 `handleSave` 中保存了 `editable` 字段：

在 `handleSave` 函数的 `update_skill` 分支中，确保 `editable` 被传递：

```typescript
      const updated: Skill = {
        ...editingSkill,
        name: editForm.name.trim(),
        description: editForm.description.trim(),
        prompt: editForm.prompt.trim()
      };
```

这里 `...editingSkill` 已经包含了 `editable` 字段，所以无需修改。

- [ ] **Step 2: 验证无需修改，Commit（如有调整）**

如果代码无需改动，跳过此 task 的 commit。如果有任何微调：

```bash
git add src/routes/skills/+page.svelte
git commit -m "fix(skills): ensure editable builtin skills are properly handled"
```

---

### Task 8: 集成测试与验证

- [ ] **Step 1: 编译 Rust 后端**

Run: `cd src-tauri && cargo build`
Expected: 编译成功

- [ ] **Step 2: 编译前端**

Run: `npm run build`
Expected: 编译成功

- [ ] **Step 3: 启动应用进行手动验证**

Run: `npm run tauri dev`

验证清单：
1. 设置页面显示翻译快捷键配置，可以修改
2. 模型页面显示目标语言下拉框
3. Skills 页面显示"翻译优化"内置 Skill，可以编辑其 prompt
4. 按下翻译快捷键开始录音，托盘图标变为录音图标
5. 再次按下翻译快捷键结束录音，等待流水线完成
6. 转录 → 润色 → 翻译 → 粘贴的完整流程
7. 普通录音快捷键仍然正常工作（不翻译）
8. 翻译失败时不粘贴，弹出系统通知

- [ ] **Step 4: Final Commit**

```bash
git add -A
git commit -m "feat: complete AI translation feature with shortcut trigger and target language config"
```
