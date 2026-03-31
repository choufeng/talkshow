# 语音替换选中文本

## 背景

Talkshow 的转写功能通过"录音 → 语音识别 → Skills 润色 → 剪贴板粘贴"的流程，将语音转为文字插入到当前光标位置。粘贴机制是模拟 Cmd+V，这在 macOS 上天然会替换当前选中的文本。

本设计在此基础上增加"选中文字 → 语音替换"能力：用户在按下录音快捷键前先选中一段文字，Talkshow 检测到选中内容后进入替换模式，并将原文作为上下文传给 Skills，使 AI 能理解替换意图。

## 需求

- 用户选中文字后按下录音快捷键，Talkshow 自动进入替换模式
- 替换模式下，录音指示器用琥珀色区分普通录音模式
- 选中的原文作为上下文传给 Skills 处理环节
- 录音结束后，转写结果替换选中的原文（利用现有 Cmd+V 行为，无需改动粘贴逻辑）
- 如果无法获取选中文本（应用不支持），静默降级为普通插入模式

## 技术方案

### 1. 获取选中文本

在 `clipboard.rs` 中新增函数：

```rust
pub fn get_selected_text(app_name: &str) -> Option<String>
```

通过 AppleScript 从当前前台应用获取选中文本：

```applescript
tell application "{app_name}" to get selection
```

返回值为 `Option<String>`：成功时返回选中文本，失败时返回 `None`。

### 2. 录音启动时检测

在 `lib.rs` 录音快捷键 handler 中（约第1016行，获取 `frontmost` 应用名之后），增加一步：

1. 调用 `get_selected_text(app_name)` 获取选中文本
2. 将结果存入全局变量 `static SELECTED_TEXT: Mutex<Option<String>>`
3. 如果有选中文本，通过 Tauri 事件通知前端：`emit("indicator:replace-mode", { text: selected_text })`

### 3. Skills prompt 增加替换上下文

修改 `skills.rs` 的 `assemble_skills_prompt()` 函数，增加可选参数 `selected_text: Option<&str>`。

当有选中文本时，在 system prompt 中追加：

```
用户选中了以下文字，准备用语音替换它。请在处理转写结果时考虑这个上下文，使替换后的文本自然衔接。

选中的原文：「{selected_text}」
```

同步修改 `process_with_skills()` 的函数签名，增加 `selected_text: Option<&str>` 参数并透传。

### 4. 前端指示器颜色区分

修改 `recording/+page.svelte`：

- 新增状态 `replaceMode: boolean`
- 监听 `indicator:replace-mode` 事件，设置 `replaceMode = true`
- 使用 CSS 变量 `--accent-color` 控制录音阶段的主色调：
  - 普通模式：`#ff0055`（红色）
  - 替换模式：`#f59e0b`（琥珀色）
- 受影响元素：呼吸环 `stroke`/`fill`、中心圆点、`REC` 标签、计时器文字、`drop-shadow`、`text-shadow`

### 5. 粘贴逻辑

无需改动。现有 `clipboard.rs` 的 `simulate_paste()` 通过模拟 Cmd+V 实现粘贴，macOS 的 Cmd+V 在有选中文字时会自动替换选中内容。

### 6. 数据流

```
用户选中文字 → 按下录音快捷键
       │
       ▼
lib.rs: 检测选中文本 → get_selected_text()
       ├── 有选中 → 存入全局变量 + emit("indicator:replace-mode")
       └── 无选中 → 正常流程
       │
       ▼
[录音 → 转写 → Skills(selected_text 作为上下文)]
       │
       ▼
[粘贴] clipboard::write_and_paste()
       └── Cmd+V 自动替换选中内容（无需改动）
```

## AppleScript 可靠性

`get selection` 在不同应用中的支持情况：

- **可靠**：TextEdit, Pages, Keynote, Notes, Xcode, VS Code, Sublime Text
- **部分支持**：Chrome/Safari（仅富文本区域）、Microsoft Office
- **不支持**：Terminal.app、部分非标准 Cocoa 应用

不支持时返回 `None`，静默降级为普通插入模式。

## 涉及文件

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src-tauri/src/clipboard.rs` | 新增函数 | `get_selected_text()`、`save_selected_text()` |
| `src-tauri/src/lib.rs` | 修改 | 录音启动时调用检测并 emit 事件 |
| `src-tauri/src/skills.rs` | 修改 | `assemble_skills_prompt()` 和 `process_with_skills()` 增加替换上下文参数 |
| `src/routes/recording/+page.svelte` | 修改 | 监听 replace-mode 事件，CSS 变量切换颜色 |

## 开发调试

录音指示器上临时显示获取到的选中文本内容（截断显示），方便开发期间验证 `get_selected_text()` 是否正常工作。正式发布前移除该调试信息。

## 不做什么

- 不使用 Accessibility API（需要用户授权，复杂度过高）
- 不增加确认对话框（增加交互步骤）
- 不修改翻译模式（翻译有独立流程）
- 不修改粘贴逻辑（现有 Cmd+V 行为已满足需求）
