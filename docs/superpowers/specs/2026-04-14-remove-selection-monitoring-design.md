# 移除选中内容检测/替换模式功能

## 背景

转写流程中有一个"监控选中内容"的环节：录音开始时模拟 Cmd+C 检测用户是否选中了文字，如果选中则进入"替换模式"，转写完成后替换该选中文字。此功能不再需要，需移除。

## 决策边界

- **移除**：选中文字检测、替换模式状态、AI prompt 中的选中文字上下文注入
- **保留**：剪贴板写入 + 模拟 Cmd+V 粘贴、目标应用激活、`arboard` crate 依赖
- **不动**：日志页面的复制功能（`src/routes/logs/+page.svelte`）是独立的 UI 功能，与转写流程无关

## 变更清单

### 1. `src-tauri/src/clipboard.rs`

移除以下函数/变量：
- `static SELECTED_TEXT: Mutex<Option<String>>`（第 4 行）
- `save_selected_text()`（第 12-16 行）
- `get_saved_selected_text()`（第 18-20 行）
- `get_replace_mode_state()` Tauri Command（第 22-29 行）
- `detect_selected_text()` macOS 版（第 70-98 行）
- `detect_selected_text()` 非 macOS 版（第 100-103 行）

保留不变：
- `TARGET_APP`、`save_target_app()`、`write_and_paste()`、`escape_applescript_string()`、`simulate_paste()`

文件从 103 行缩减到约 40 行。

### 2. `src-tauri/src/lib.rs`

**第 307-308 行**：移除 `clipboard::get_saved_selected_text()` 调用和 `selected_text_ref` 变量，`process_with_skills()` 调用中移除 `selected_text_ref` 参数。

**第 585-589 行**：`show_indicator()` 函数移除 `selected_text: Option<&str>` 参数，payload 固定为 `replaceMode: false, selectedPreview: ""`。

**第 1075 行**：从 `invoke_handler` 列表中删除 `clipboard::get_replace_mode_state`。

**第 1306-1357 行**：简化 Phase 2 后台线程：
- 保留：获取前台应用名、`save_target_app()`、自动静音、注册 ESC 快捷键
- 删除：`detect_selected_text()` 调用、`save_selected_text()` 调用、替换模式指示器更新事件、相关日志

**第 1299 行**：`show_indicator(&app_handle, None)` 保留原样（参数已移除，永远不传选中文字）。

### 3. `src-tauri/src/skills.rs`

**`assemble_skills_prompt()`（第 54-59 行）**：
- 移除 `selected_text: Option<&str>` 参数
- 删除第 69-74 行的 `if let Some(selected)` 块（AI prompt 中"用户选中了以下文字"的注入）

**`process_with_skills()`（第 88-96 行）**：
- 移除 `selected_text: Option<&str>` 参数
- 第 150 行调用 `assemble_skills_prompt()` 时移除该参数

**`process_with_skills_client()`（第 225-232 行）**：
- 移除 `selected_text: Option<&str>` 参数
- 第 282 行调用 `assemble_skills_prompt()` 时移除该参数

### 4. `src-tauri/tests/skills_pipeline.rs`

- **删除** `test_skills_with_selected_text()` 测试（第 223-247 行）
- **简化** `test_assemble_skills_prompt_with_skill()`（第 314-320 行）：移除 `Some("选中文本")` 参数
- **更新** 断言：移除第 326 行 `assert!(system.contains("选中文本"))`
- **简化** `test_assemble_skills_prompt_without_selected_text()`（第 331-337 行）：移除 `None` 参数

### 5. `src/routes/recording/+page.svelte`

- 删除 `replaceMode` 和 `selectedPreview` 状态变量（第 16-17 行）
- 删除 `fetchReplaceModeState()` 函数及其调用（第 20-28 行）
- 删除 `indicator:recording` 监听中的 `replaceMode = false; selectedPreview = "";`（第 66-67 行）
- 简化 `style` 属性（第 93 行）：颜色固定为 `#ff0055`，移除橙色替换模式分支

## 不涉及的文件

- `src/routes/logs/+page.svelte` — 日志页面的复制功能独立于转写流程
- `src/lib/components/onboarding/steps/TryTranscriptionStep.svelte` — 仅消费 `pipeline:complete` 事件，无选中文字依赖
- `src/lib/components/onboarding/steps/TryTranslationStep.svelte` — 同上
- `Cargo.toml` — `arboard` 依赖保留
