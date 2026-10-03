# 校对浮窗 · 智能候选按钮 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** review 浮窗按语义路由推荐智能生成按钮（主按钮+歧义次选+折叠其他去向），浮窗内语音纯转写插入，发送键可配置。

**Architecture:** intent.rs 扩展 alternatives 输出；pipeline 拦截点组装 primary/alternatives/available 下发；/review 页按候选渲染按钮与键盘；新增编辑模式分支（review 活跃时新录音纯转写插入浮窗）。

**Tech Stack:** Rust (Tauri v2) / Svelte 5 runes / vitest / cargo test

**Worktree:** `.worktrees/review-intercept`（所有路径相对仓库根）

**Spec:** `docs/superpowers/specs/2026-10-04-review-smart-buttons-design.md`

---

### Task 1: intent.rs — alternatives 解析与 prompt 扩展

**Files:**
- Modify: `src-tauri/src/intent.rs`

- [ ] **Step 1: 写失败测试（追加到 intent.rs `mod tests` 末尾）**

```rust
    #[test]
    fn parse_alternatives_field() {
        let d = parse_llm_output(
            r#"{"intent":"transcribe","text":"这条内容值得被记录下来","alternatives":["hook"]}"#,
        );
        assert_eq!(d.intent, Intent::Transcribe);
        assert_eq!(d.alternatives, vec!["hook".to_string()]);
    }

    #[test]
    fn parse_alternatives_filters_translate_and_unknown() {
        let d = parse_llm_output(
            r#"{"intent":"transcribe","text":"帮我看看","alternatives":["translate","open_url","ask","ask"]}"#,
        );
        // translate 缺语言参数不参与;未知 kind 忽略;去重
        assert_eq!(d.alternatives, vec!["ask".to_string()]);
    }

    #[test]
    fn parse_alternatives_drops_primary_kind() {
        let d = parse_llm_output(
            r#"{"intent":"hook","text":"明天开会","alternatives":["hook","ask"]}"#,
        );
        assert_eq!(d.intent, Intent::Hook);
        assert_eq!(d.alternatives, vec!["ask".to_string()]);
    }

    #[test]
    fn parse_without_alternatives_is_empty() {
        let d = parse_llm_output(r#"{"intent":"transcribe","text":"你好"}"#);
        assert!(d.alternatives.is_empty());
    }

    #[test]
    fn disabled_alternative_kind_is_dropped() {
        let d = parse_llm_output_disabled(
            r#"{"intent":"transcribe","text":"值得记录","alternatives":["hook"]}"#,
            &["hook".to_string()],
        );
        assert_eq!(d.intent, Intent::Transcribe);
        assert!(d.alternatives.is_empty());
    }

    #[test]
    fn intent_instruction_mentions_alternatives() {
        let s = intent_instruction();
        assert!(s.contains("alternatives"));
    }
```

- [ ] **Step 2: 运行确认失败**

Run: `cd src-tauri && cargo test intent:: 2>&1 | grep -E "alternatives|error\[" | head`
Expected: 编译错误（`alternatives` 字段不存在）

- [ ] **Step 3: 实现**

`RawDecision`（serde default 已有）加字段：

```rust
#[derive(Deserialize, Default)]
#[serde(default)]
struct RawDecision {
    intent: String,
    target_lang: String,
    text: String,
    result: Option<String>,
    alternatives: Vec<String>,
}
```

`IntentDecision` 加字段（`passthrough` 补空数组）：

```rust
#[derive(Debug, Clone)]
pub struct IntentDecision {
    pub intent: Intent,
    pub text: String,
    pub result: Option<String>,
    /// 歧义候选 kinds(仅 "hook"/"ask";已过滤主意图同名/translate/未知/被禁用)
    pub alternatives: Vec<String>,
}
```

`passthrough`：

```rust
    pub fn passthrough(text: impl Into<String>) -> Self {
        Self {
            intent: Intent::Transcribe,
            text: text.into(),
            result: None,
            alternatives: Vec::new(),
        }
    }
```

`parse_llm_output_disabled` 中所有 `IntentDecision { ... }` 字面量补 `alternatives` 字段，并在 `kind` 判定后统一过滤（替换函数尾部）：

```rust
    let kind = match &decision.intent {
        Intent::Translate { .. } => "translate",
        Intent::Ask => "ask",
        Intent::Hook => "hook",
        Intent::Transcribe => return decision,
    };
    if disabled.iter().any(|d| d == kind) {
        return IntentDecision::passthrough(decision.text);
    }
    decision
```

改为（新增 alternatives 过滤逻辑，函数体三个字面量先各自写 `alternatives: filter_alternatives(&d.alternatives, &decision.intent)`，函数尾部禁用回退保持 passthrough——被禁用时 alternatives 一并丢弃）：

```rust
    // 主意图构造处(translate/ask/hook 三个分支)均:
    //   alternatives: filter_alternatives(&d.alternatives, &decision.intent)
    // passthrough 分支天然空数组
```

新增私有函数：

```rust
/// 过滤 LLM 报告的歧义候选:只留 hook/ask,去重,剔除与主意图同 kind;
/// translate 缺语言参数不参与歧义候选,直接忽略
fn filter_alternatives(raw: &[String], primary: &Intent) -> Vec<String> {
    let primary_kind = match primary {
        Intent::Transcribe => "paste",
        Intent::Translate { .. } => "paste",
        Intent::Ask => "ask",
        Intent::Hook => "hook",
    };
    let mut out: Vec<String> = Vec::new();
    for a in raw {
        let a = a.trim();
        if (a == "hook" || a == "ask") && a != primary_kind && !out.iter().any(|x| x == a) {
            out.push(a.to_string());
        }
    }
    out
}
```

注意：`primary_kind` 对 Transcribe/Translate 用 "paste"（决策层 kind 是转写粘贴），hook/ask 同名。禁用回退分支保持：

```rust
    if disabled.iter().any(|d| d == kind) {
        return IntentDecision::passthrough(decision.text);
    }
```

（被禁用 → passthrough，alternatives 不保留；`disabled_alternative_kind_is_dropped` 测试覆盖 alternatives 内 kind 被禁用的情况：`filter_alternatives` 前先剔除 disabled——在 filter_alternatives 调用点改为先过滤 disabled 再过滤 kind：

```rust
fn filter_alternatives(raw: &[String], primary: &Intent, disabled: &[String]) -> Vec<String> {
    let primary_kind = match primary {
        Intent::Transcribe | Intent::Translate { .. } => "paste",
        Intent::Ask => "ask",
        Intent::Hook => "hook",
    };
    let mut out: Vec<String> = Vec::new();
    for a in raw {
        let a = a.trim();
        if (a == "hook" || a == "ask")
            && a != primary_kind
            && !disabled.iter().any(|d| d == a)
            && !out.iter().any(|x| x == a)
        {
            out.push(a.to_string());
        }
    }
    out
}
```

`parse_llm_output_disabled` 内 `parse_llm_output(raw)` 非测试版签名无 disabled——把 `#[cfg(test)] pub fn parse_llm_output` 改为直接调 `parse_llm_output_disabled(raw, &[])` 保持现状。）

- [ ] **Step 4: prompt 扩展**

`intent_instruction_disabled` 中 `rules` 列表（"有歧义或不确定时,intent 用 \"transcribe\"" 之前）插入：

```rust
        rules.push(
            "开头无明确指令、但句意可能指向记录或问询(如\"这个值得记一下\"),intent 判为 \"transcribe\",同时把可能的意图加入 alternatives 数组(如 [\"hook\"] 或 [\"ask\"]);开头有明确指令时 alternatives 为空数组或省略".to_string(),
        );
```

`outputs` 示例第一个改为：

```rust
    let mut outputs: Vec<&str> = vec![
        "{\"intent\":\"transcribe\",\"text\":\"润色后的正文\",\"alternatives\":[]}",
    ];
```

- [ ] **Step 5: 运行测试通过**

Run: `cd src-tauri && cargo test intent:: 2>&1 | grep "test result"`
Expected: `ok. N passed; 0 failed`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/intent.rs
git commit -m "feat: 语义路由输出歧义候选 alternatives"
```

---

### Task 2: config — ReviewConfig.send_key

**Files:**
- Modify: `src-tauri/src/config/mod.rs`
- Modify: `src/lib/stores/config.ts`

- [ ] **Step 1: Rust 加字段**

```rust
/// 发送前人工确认:拦截 pipeline 产物,浮窗内编辑文本+选择去向
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ReviewConfig {
    pub enabled: bool,
    /// 浮窗发送键: "enter" | "cmd_enter"
    pub send_key: String,
}
```

`Default` 派生给 `send_key` 空串——空串按 "enter" 处理（前端与拦截均不读此值，仅设置页与浮窗 UI 用；空串 fallback 前端做）。若要后端规范化，在 `ReviewConfig` 手写 Default：

```rust
impl Default for ReviewConfig {
    fn default() -> Self {
        ReviewConfig {
            enabled: false,
            send_key: "enter".to_string(),
        }
    }
}
```

（采用手写 Default，derive 改回非 Default derive。）

- [ ] **Step 2: 前端类型**

`src/lib/stores/config.ts`：

```ts
export interface ReviewConfig {
  enabled: boolean;
  send_key: string;
}
```

store 初始值与 `load()` 兜底处（`review: { enabled: false }` 两处）改为：

```ts
      review: {
        enabled: false,
        send_key: 'enter'
      }
```

load 兜底：

```ts
        config.features.review = config.features.review || { enabled: false, send_key: 'enter' };
        config.features.review.send_key = config.features.review.send_key || 'enter';
```

- [ ] **Step 3: 检查通过**

Run: `npm run check 2>&1 | tail -2`
Expected: `svelte-check found 0 errors`

（测试 fixture 中 `review: { enabled: false }` 会类型报错——5 个 fixture 文件与 `config.test.ts` 的 `review: { enabled: false }` 全部改为 `review: { enabled: false, send_key: 'enter' }`。）

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/config/mod.rs src/lib/stores/config.ts src/lib/ai/shared/config.test.ts src/routes/settings/settings-page.integration.test.ts src/routes/models/models-page.integration.test.ts src/routes/hooks/hooks-page.integration.test.ts src/routes/skills/skills-page.integration.test.ts
git commit -m "feat: ReviewConfig.send_key 发送键配置"
```

---

### Task 3: review.rs — ReviewRequest 改造 + is_active

**Files:**
- Modify: `src-tauri/src/review.rs`

- [ ] **Step 1: ReviewRequest 字段替换**

删除 `action: String` 与 `can_hook: bool`，新增：

```rust
#[derive(Clone, serde::Serialize)]
pub struct ReviewRequest {
    pub id: u64,
    /// 原始转写(只读参考)
    pub raw_text: String,
    /// 主文本(润色后/译文/hook 整理后),可编辑
    pub text: String,
    /// 翻译产物时的原文
    pub original_text: Option<String>,
    /// 译文语言(展示用)
    pub target_lang: Option<String>,
    /// 模型首选动作 "paste"|"ask"|"hook"(翻译模式固定 "paste")
    pub primary: String,
    /// 歧义次选 kinds(已过滤:去重/hook 可用性/主同名),仅 "hook"|"ask"
    pub alternatives: Vec<String>,
    /// 全量去向中当前可用的 kinds(前端禁用判断)
    pub available: Vec<String>,
    /// 粘贴目标应用
    pub target_app: Option<String>,
    /// hook 动作时的发送预览
    pub hook: Option<HookPreview>,
    /// 音频文件路径(history 回溯用)
    pub audio_path: String,
}
```

- [ ] **Step 2: is_active helper + 组装辅助**

```rust
impl ReviewState {
    /// 编辑模式判定:浮窗是否有待确认内容
    pub fn is_active(&self) -> bool {
        self.current.lock().unwrap().is_some()
    }
}
```

新增模块级组装函数（pipeline 三个拦截点共用过滤逻辑）：

```rust
/// 组装可用去向与过滤后的歧义候选
pub fn assemble_candidates(
    primary: &str,
    raw_alternatives: &[String],
    hook_configured: bool,
) -> (Vec<String>, Vec<String>) {
    let mut available = vec!["paste", "translate", "ask"];
    if hook_configured {
        available.push("hook");
    }
    let alternatives: Vec<String> = raw_alternatives
        .iter()
        .filter(|a| *a != primary && available.contains(a))
        .cloned()
        .collect();
    (available, alternatives)
}
```

`ReviewConfirm` 保持不变（`action` 字段 = 用户最终选择，dispatch 依据）。

- [ ] **Step 3: Rust 单测（追加 review.rs `mod tests`）**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_filters_hook_when_unconfigured() {
        let (available, alts) = assemble_candidates("paste", &["hook".into(), "ask".into()], false);
        assert_eq!(available, vec!["paste", "translate", "ask"]);
        assert_eq!(alts, vec!["ask".to_string()]);
    }

    #[test]
    fn assemble_drops_primary_duplicate() {
        let (_, alts) = assemble_candidates("hook", &["hook".into(), "ask".into()], true);
        assert_eq!(alts, vec!["ask".to_string()]);
    }

    #[test]
    fn assemble_hook_configured_available() {
        let (available, _) = assemble_candidates("paste", &[], true);
        assert_eq!(available, vec!["paste", "translate", "ask", "hook"]);
    }
}
```

- [ ] **Step 4: 运行测试（此时 pipeline 未改，编译会因字段缺失失败——本 Task 先改 review.rs，pipeline 修复在 Task 4；为保持每步可编译，本 Task 只加结构体字段而暂不删旧字段？不——采用合并方案：Task 3 与 Task 4 合并提交）**

> **注意：** ReviewRequest 字段变化会让 pipeline.rs 三处拦截点编译失败。Task 3 与 Task 4 必须一起编译提交；执行时可先写 Task 3 代码，暂不运行全量 cargo，Task 4 完成后统一跑。

Run: `cd src-tauri && cargo test review:: 2>&1 | grep "test result"`
Expected（Task 4 完成后）: `ok. N passed; 0 failed`

- [ ] **Step 5: Commit（与 Task 4 合并提交）**

---

### Task 4: pipeline.rs — 拦截点组装 + 编辑模式分支

**Files:**
- Modify: `src-tauri/src/pipeline.rs`

- [ ] **Step 1: 编辑模式分支**

`text_result` `Ok(text)` 分支内、`logger.info("ai", "AI 转写成功"...)` 之后、`let skills_start = Instant::now();` 之前插入：

```rust
                                    // 编辑模式:浮窗有待确认内容时,新录音一律纯转写插入浮窗,
                                    // 跳过润色/语义路由/拦截(选区替换或光标插入由前端处理)
                                    if app_config.features.review.enabled
                                        && h.state::<crate::review::ReviewState>().is_active()
                                    {
                                        let _ = h.emit_to(
                                            "review",
                                            "review:insert",
                                            serde_json::json!({ "text": text }),
                                        );
                                        logger.info(
                                            "review",
                                            "编辑模式语音插入",
                                            Some(serde_json::json!({
                                                "text_length": text.len(),
                                            })),
                                        );
                                        crate::indicator::show_result(&h, "已插入");
                                        restore_tray_only(&h);
                                        cleanup_esc(&h);
                                        return;
                                    }
```

- [ ] **Step 2: 三个拦截点组装替换**

ask 拦截点（原 `action: "ask".to_string()` 的 ReviewRequest）：

```rust
                                        if app_config.features.review.enabled {
                                            let hook_configured = app_config.features.hooks.enabled
                                                && !app_config.features.hooks.url.trim().is_empty();
                                            let (available, alternatives) =
                                                crate::review::assemble_candidates(
                                                    "ask",
                                                    &decision.alternatives,
                                                    hook_configured,
                                                );
                                            crate::review::open_review(
                                                &h,
                                                crate::review::ReviewRequest {
                                                    id: session.id,
                                                    raw_text: raw_text_for_history.clone(),
                                                    text: final_text.clone(),
                                                    original_text: None,
                                                    target_lang: None,
                                                    primary: "ask".to_string(),
                                                    alternatives,
                                                    available,
                                                    target_app: saved_target_app.clone(),
                                                    hook: None,
                                                    audio_path: audio_path.display().to_string(),
                                                },
                                            );
                                            logger.info("review", "ask 拦截待确认", None);
                                            restore_tray_only(&h);
                                            cleanup_esc(&h);
                                            return;
                                        }
```

hook 拦截点（原 `action: "hook"` + `can_hook: true` 处）同构：`primary: "hook".to_string()`，`assemble_candidates("hook", &decision.alternatives, true)`（能走到 hook 拦截点 = hook 已配置）。

汇合点拦截（原 `action: "paste"` + `can_hook: ...` 处）：

```rust
                                        let primary = "paste";
                                        let hook_configured = app_config.features.hooks.enabled
                                            && !app_config.features.hooks.url.trim().is_empty();
                                        let (available, alternatives) =
                                            crate::review::assemble_candidates(
                                                primary,
                                                &decision.alternatives,
                                                hook_configured,
                                            );
                                        // ReviewRequest { ..., primary: primary.to_string(),
                                        //   alternatives, available, ... }
```

（保持其余字段与现有一致；`routed_lang` 的 display_lang 逻辑不变。）

- [ ] **Step 3: 编译 + 全量测试**

Run: `cd src-tauri && cargo test 2>&1 | grep -E "test result" | head -3`
Expected: 全部 `ok`

Run: `cd src-tauri && cargo fmt && cargo clippy --all-targets --all-features 2>&1 | grep -c "^error"`
Expected: `0`

- [ ] **Step 4: Commit（含 Task 3）**

```bash
git add src-tauri/src/review.rs src-tauri/src/pipeline.rs
git commit -m "feat: ReviewRequest 智能候选(primary/alternatives/available) + 编辑模式语音插入分支"
```

---

### Task 5: 前端 /review — 智能按钮 + 键盘 + 语音插入

**Files:**
- Modify: `src/routes/review/+page.svelte`
- Modify: `src/routes/review/review-page.test.ts`

- [ ] **Step 1: 重写 script 区候选与键盘逻辑**

替换 `<script>` 内状态与函数（关键变化，完整代码）：

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { X, ClipboardPaste, Languages, MessageCircle, Webhook, ChevronDown } from 'lucide-svelte';
  import '$lib/stores/theme';

  interface HookPreview {
    url: string;
    body: string;
    vars: Record<string, string>;
  }

  interface ReviewRequest {
    id: number;
    raw_text: string;
    text: string;
    original_text: string | null;
    target_lang: string | null;
    primary: 'paste' | 'ask' | 'hook';
    alternatives: string[];
    available: string[];
    target_app: string | null;
    hook: HookPreview | null;
    audio_path: string;
  }

  type ActionKind = 'paste' | 'translate' | 'ask' | 'hook';
  const ALL_KINDS: ActionKind[] = ['paste', 'translate', 'ask', 'hook'];
  const KIND_LABEL: Record<ActionKind, string> = {
    paste: '转写粘贴',
    translate: '翻译',
    ask: '问询',
    hook: '发送到 Hook'
  };

  const LANGUAGES = [
    'English', '中文', '日本語', '한국어', 'Français', 'Deutsch',
    'Español', 'Português', 'Italiano', 'Русский', 'العربية', 'हिन्दी'
  ];

  let req = $state<ReviewRequest | null>(null);
  let text = $state('');
  let currentAction = $state<ActionKind>('paste');
  let targetLang = $state('English');
  let busy = $state(false);
  let error = $state('');
  let showRaw = $state(false);
  let showMore = $state(false);
  let sendKey = $state<'enter' | 'cmd_enter'>('enter');
  let textareaEl = $state<HTMLTextAreaElement | null>(null);

  function load(r: ReviewRequest) {
    req = r;
    text = r.text;
    currentAction = r.primary === 'paste' || r.primary === 'ask' || r.primary === 'hook' ? r.primary : 'paste';
    if (r.target_lang) targetLang = r.target_lang;
    error = '';
    showMore = false;
  }

  async function confirm() {
    if (!req || busy) return;
    busy = true;
    error = '';
    try {
      await invoke('review_confirm', {
        confirm: {
          id: req.id,
          action: currentAction,
          text,
          target_lang: currentAction === 'translate' ? targetLang : null
        }
      });
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function dismiss() {
    if (busy) return;
    await invoke('review_dismiss').catch(() => {});
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      dismiss();
      return;
    }
    // 发送键: enter 或 cmd_enter(与配置互斥的另一变体在 textarea 中作换行,由 textarea handler 处理)
    if (e.key === 'Enter' && !e.shiftKey && e.metaKey === (sendKey === 'cmd_enter')) {
      e.preventDefault();
      confirm();
      return;
    }
    // ⌘1-4 恒映射固定四项
    if (e.metaKey && ['1', '2', '3', '4'].includes(e.key)) {
      const kind = ALL_KINDS[Number(e.key) - 1];
      if (req?.available.includes(kind)) {
        e.preventDefault();
        currentAction = kind;
        showMore = false;
      }
    }
  }

  function onTextareaKeydown(e: KeyboardEvent) {
    // 非发送键变体的 Enter/⌘Enter → 手动插入换行(native Cmd+Enter 不换行)
    const isSendCombo = e.metaKey === (sendKey === 'cmd_enter');
    if (e.key === 'Enter' && !e.shiftKey && !isSendCombo) {
      e.preventDefault();
      insertAtCursor('\n');
    }
    // 其余(Esc/⌘n/发送组合)冒泡到 window handler
  }

  function insertAtCursor(insert: string) {
    const ta = textareaEl;
    if (!ta) return;
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    text = text.slice(0, start) + insert + text.slice(end);
    requestAnimationFrame(() => {
      ta.selectionStart = ta.selectionEnd = start + insert.length;
      ta.focus();
    });
  }

  function insertVoice(payload: { text: string }) {
    const ta = textareaEl;
    if (!ta) {
      text += payload.text;
      return;
    }
    // 选区替换;无选区则光标插入
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    text = text.slice(0, start) + payload.text + text.slice(end);
    requestAnimationFrame(() => {
      ta.selectionStart = ta.selectionEnd = start + payload.text.length;
      ta.focus();
    });
  }

  onMount(() => {
    (async () => {
      try {
        const r = await invoke<ReviewRequest | null>('review_get');
        if (r) load(r);
        else await invoke('review_dismiss').catch(() => {});
      } catch {
        /* 非 Tauri 环境 */
      }
      try {
        const cfg = await invoke<{ features: { review: { send_key: string } } }>('get_config');
        const k = cfg?.features?.review?.send_key;
        if (k === 'cmd_enter') sendKey = 'cmd_enter';
      } catch {
        /* fallback: enter */
      }
    })();
    window.addEventListener('keydown', onKeydown);
    let un: (() => void) | null = null;
    let unInsert: (() => void) | null = null;
    let unClose: (() => void) | null = null;
    (async () => {
      const l = (await listen<ReviewRequest>('review:open', (e) => load(e.payload))) as unknown as
        | (() => void)
        | { unlisten: () => void };
      un = typeof l === 'function' ? l : () => l.unlisten();
      const ins = (await listen<{ text: string }>('review:insert', (e) => insertVoice(e.payload))) as unknown as
        | (() => void)
        | { unlisten: () => void };
      unInsert = typeof ins === 'function' ? ins : () => ins.unlisten();
      try {
        unClose = await getCurrentWindow().onCloseRequested(async () => {
          await invoke('review_dismiss').catch(() => {});
        });
      } catch {
        /* 非 Tauri 环境 */
      }
    })();
    return () => {
      un?.();
      unInsert?.();
      unClose?.();
      window.removeEventListener('keydown', onKeydown);
    };
  });

  const primaryLabel = $derived.by(() => {
    if (currentAction === 'paste') return req?.target_app ? `粘贴到 ${req.target_app}` : '确认粘贴';
    if (currentAction === 'translate') return `翻译为${targetLang}并粘贴`;
    if (currentAction === 'ask') return '转入对话';
    return '发送到 Hook';
  });

  const ambiguous = $derived(!!req && req.alternatives.length > 0);
  const altKind = $derived(req?.alternatives[0] as ActionKind | undefined);
  const sendHint = $derived(sendKey === 'enter' ? '⏎' : '⌘⏎');

  const prettyBody = $derived.by(() => {
    if (!req?.hook) return '';
    try {
      return JSON.stringify(JSON.parse(req.hook.body), null, 2);
    } catch {
      return req.hook.body;
    }
  });
</script>
```

- [ ] **Step 2: 重写按钮区 markup**

替换原「去向选择 + 操作」与歧义提示（保留 hook 预览/翻译语言选择/错误提示区不动）：

```svelte
    <!-- 去向 + 操作 -->
    <div class="border-t border-border p-4">
      {#if ambiguous}
        <div class="mb-3 rounded-lg border border-[#ffb020]/40 bg-[#ffb020]/10 px-3 py-2 text-caption text-[#8a5a00] dark:text-[#ffb020]">
          ⚠ 语义路由不确定，请选择去向
        </div>
      {/if}
      {#if ambiguous && altKind}
        <div class="mb-3 flex flex-col gap-2">
          <button
            class="h-10 rounded-lg bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-body font-medium text-white shadow-btn-primary transition-opacity disabled:opacity-50"
            disabled={busy || !text.trim()}
            onclick={confirm}
          >
            {KIND_LABEL[currentAction]} <span class="ml-1 text-caption opacity-80">{sendHint}</span>
          </button>
          <button
            class="h-10 rounded-lg border border-border bg-background text-body text-foreground transition-colors hover:bg-muted/50"
            disabled={busy}
            onclick={() => (currentAction = altKind)}
          >
            {KIND_LABEL[altKind]}
          </button>
        </div>
      {:else}
        <button
          class="mb-3 h-10 w-full rounded-lg bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-body font-medium text-white shadow-btn-primary transition-opacity disabled:opacity-50"
          disabled={busy || !text.trim()}
          onclick={confirm}
        >
          {primaryLabel} <span class="ml-1 text-caption opacity-80">{sendHint}</span>
        </button>
      {/if}

      {#if showMore}
        <div class="mb-3 grid grid-cols-4 gap-2">
          {#each ALL_KINDS as kind, i (kind)}
            <button
              class="rounded-lg border px-1 py-2 text-center text-caption transition-colors {currentAction === kind
                ? 'border-btn-primary-to bg-gradient-to-b from-btn-primary-from/15 to-btn-primary-to/10 text-foreground font-medium'
                : req?.available.includes(kind)
                  ? 'border-border bg-background text-foreground-alt hover:bg-muted/50'
                  : 'cursor-not-allowed border-border bg-background text-muted-foreground opacity-40'}"
              onclick={() => {
                if (req?.available.includes(kind)) {
                  currentAction = kind;
                  showMore = false;
                }
              }}
              disabled={!req?.available.includes(kind)}
            >
              {KIND_LABEL[kind]}<span class="ml-1 opacity-60">⌘{i + 1}</span>
            </button>
          {/each}
        </div>
      {/if}

      <div class="flex items-center justify-between">
        <button
          class="text-caption text-muted-foreground underline-offset-2 transition-colors hover:text-foreground hover:underline"
          onclick={() => (showMore = !showMore)}
        >
          其他去向…
        </button>
        <button
          class="rounded-lg border border-border px-4 py-2 text-caption text-foreground transition-colors hover:bg-muted/50 disabled:opacity-50"
          disabled={busy}
          onclick={dismiss}
        >
          取消
        </button>
      </div>
    </div>
```

textarea 加绑定与 keydown：

```svelte
        <textarea
          bind:this={textareaEl}
          bind:value={text}
          onkeydown={onTextareaKeydown}
          class="min-h-[96px] w-full flex-1 resize-none rounded-lg border border-border-input bg-background px-3 py-2 text-body leading-relaxed focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20"
          placeholder="内容为空"
        ></textarea>
```

- [ ] **Step 3: 更新测试（review-page.test.ts 全量替换关键用例）**

fixture 改造 + 新用例：

```ts
function sampleRequest(overrides: Record<string, unknown> = {}) {
  return {
    id: 7,
    raw_text: '记录 明天下午三点开会',
    text: '明天下午三点开会',
    original_text: null,
    target_lang: null,
    primary: 'hook',
    alternatives: [],
    available: ['paste', 'translate', 'ask', 'hook'],
    target_app: 'Safari',
    hook: {
      url: 'https://api.dida365.com/open/v1/task',
      body: '{"title":"开会","content":"记录 明天下午三点开会"}',
      vars: { title: '开会' }
    },
    audio_path: '/tmp/a.wav',
    ...overrides
  };
}
```

mock `get_config` 返回 `{ features: { review: { send_key: 'enter' } } }`。

用例（替换原「确认时提交编辑后的文本与所选去向」中按钮定位）：

```ts
  it('歧义时渲染主次双按钮,点击次按钮切换后确认', async () => {
    mockTauriInvoke(async (cmd) => {
      if (cmd === 'review_get')
        return sampleRequest({ primary: 'paste', alternatives: ['hook'], hook: null });
      if (cmd === 'get_config') return { features: { review: { send_key: 'enter' } } };
      return null;
    });
    render(ReviewPage);
    await waitFor(() => screen.getByDisplayValue('明天下午三点开会'));
    // 主按钮 = paste,次按钮 = hook
    expect(screen.getByText(/转写粘贴/)).toBeTruthy();
    const altBtn = screen.getByText('发送到 Hook', { selector: 'button' });
    await fireEvent.click(altBtn);
    await fireEvent.click(screen.getByText(/发送到 Hook/));
    // confirm 提交 action=hook(按钮文案已切换为主按钮形态)
  });

  it('Enter 触发发送(默认 enter 配置)', async () => {
    const confirmSpy = vi.fn(async () => {});
    mockTauriInvoke(async (cmd) => {
      if (cmd === 'review_get') return sampleRequest({ primary: 'paste', hook: null });
      if (cmd === 'get_config') return { features: { review: { send_key: 'enter' } } };
      if (cmd === 'review_confirm') return confirmSpy();
      return null;
    });
    render(ReviewPage);
    await waitFor(() => screen.getByDisplayValue('明天下午三点开会'));
    await fireEvent.keyDown(window, { key: 'Enter' });
    await waitFor(() => expect(confirmSpy).toHaveBeenCalled());
  });

  it('review:insert 事件替换 textarea 内容', async () => {
    // mock listen 捕获 insert 回调,手动触发
    let insertCb: ((e: { payload: { text: string } }) => void) | null = null;
    vi.mocked(await import('@tauri-apps/api/event')).listen.mockImplementation(
      (async (name: string, cb: never) => {
        if (name === 'review:insert') insertCb = cb;
        return { unlisten: () => {} };
      }) as never
    );
    mockTauriInvoke(async (cmd) => {
      if (cmd === 'review_get') return sampleRequest({ primary: 'paste', hook: null, text: '原始内容' });
      if (cmd === 'get_config') return { features: { review: { send_key: 'enter' } } };
      return null;
    });
    render(ReviewPage);
    await waitFor(() => screen.getByDisplayValue('原始内容'));
    const ta = screen.getByRole('textbox') as HTMLTextAreaElement;
    ta.focus();
    ta.setSelectionRange(0, 4); // 选中「原始内容」
    insertCb?.({ payload: { text: '新内容' } });
    await waitFor(() => expect(screen.getByDisplayValue('新内容')).toBeTruthy());
  });
```

- [ ] **Step 4: 运行前端测试 + check**

Run: `npx vitest run src/routes/review 2>&1 | grep -E "Test Files|Tests "`
Expected: all passed, 0 errors

Run: `npm run check 2>&1 | tail -2`
Expected: `0 errors`

- [ ] **Step 5: Commit**

```bash
git add src/routes/review
git commit -m "feat: 校对浮窗智能候选按钮(主按钮/歧义双选/其他去向折叠) + 语音插入 + 发送键"
```

---

### Task 6: settings — 发送键选项

**Files:**
- Modify: `src/routes/settings/+page.svelte`
- Modify: `src/routes/settings/settings-page.integration.test.ts`（如 fixture 需更新）

- [ ] **Step 1: 卡片内加单选**

「发送前人工确认」卡片（`{#if $config.features?.intent_routing?.enabled}` 块后第二个卡片）内部加：

```svelte
      {#if $config.features?.review?.enabled}
        <div class="mt-4 border-t border-border pt-4">
          <div class="text-body text-foreground-alt mb-2">浮窗发送键（校对浮窗内触发确认）</div>
          <div class="flex gap-2">
            {#each [{ v: 'enter', l: 'Enter' }, { v: 'cmd_enter', l: '⌘Enter' }] as opt (opt.v)}
              <button
                class="rounded-lg border px-4 py-2 text-sm transition-colors {($config.features?.review?.send_key ?? 'enter') === opt.v
                  ? 'border-btn-primary-to bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white font-medium'
                  : 'border-border bg-background text-foreground hover:bg-muted/50'}"
                onclick={() => config.save(updateFeature($config, 'review', (r) => ({ ...r, send_key: opt.v })))}
              >
                {opt.l}
              </button>
            {/each}
          </div>
          <div class="mt-2 text-caption text-muted-foreground">另一变体在文本框中作为换行；Shift+Enter 恒为换行；Esc 取消</div>
        </div>
      {/if}
```

- [ ] **Step 2: check + settings 测试**

Run: `npm run check 2>&1 | tail -2 && npx vitest run src/routes/settings 2>&1 | grep -E "Test Files|Tests "`
Expected: 0 errors / passed

- [ ] **Step 3: Commit**

```bash
git add src/routes/settings
git commit -m "feat: 设置页浮窗发送键选项(Enter/⌘Enter)"
```

---

### Task 7: 全量验证 + README

**Files:**
- Modify: `README.md`（使用方式一节补两行）

- [ ] **Step 1: 全量**

Run: `npm run check && npx vitest run 2>&1 | grep -E "Test Files|Tests " && cd src-tauri && cargo fmt --check && cargo clippy --all-targets --all-features 2>&1 | grep -c "^error" && cargo test 2>&1 | grep "test result"`
Expected: check 0 errors / 前端全过 / clippy 0 / Rust 全过

- [ ] **Step 2: README 快捷键表下方补**

```markdown
### 发送前人工确认（校对浮窗）

设置 → 语音指令 → 「发送前人工确认」开启后，每次录音处理完成弹出校对浮窗：

- 主按钮为语义路由推荐的去向，回车（可配置为 ⌘Enter）直接发送
- 语义路由不确定时给出主次双选，其余路径收进「其他去向…」（⌘1-4）
- 浮窗打开期间再次录音 = 编辑模式：纯转写文本替换选中内容/插入光标处，不进路由
```

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: 校对浮窗智能候选使用说明"
```

---

## Self-Review 记录

- 规格覆盖：§1→Task1，§2→Task3/4，§3→Task5，§4→Task4，§5→Task2/6，§7 边界→Task1(filter)/Task3(assemble)，§8 测试→各 Task 步骤 ✓
- 占位符：无 TBD；Task3 Step4 明确与 Task4 合并编译 ✓
- 类型一致：`assemble_candidates(primary,&alternatives,hook_configured)->(Vec,Vec)` 三调用点一致；前端 `primary/alternatives/available` 与 Rust serde 字段对齐（snake_case 序列化）✓
- 假设校验：`emit_to("review",...)` 窗口 label 已在 Task review.rs `open_review` 用同 label 创建 ✓；`get_config` 命令已存在 ✓；`updateFeature` 泛型 key `review` 存在 ✓
