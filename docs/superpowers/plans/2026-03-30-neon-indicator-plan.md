# 霓虹科幻风录音指示器实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将录音浮动窗从简单的脉冲点/旋转圆环改为 Cyberpunk 霓虹风格（品红脉冲环 + 青色发光水母）。

**Architecture:** 修改单个 Svelte 组件 `src/routes/recording/+page.svelte`，用内联 SVG 动画替换现有 CSS 动画。如需要，微调 Rust 端窗口尺寸。

**Tech Stack:** Svelte 5 (Runes), SVG 动画, CSS filter (辉光), Tauri 2

**Spec:** `docs/superpowers/specs/2026-03-30-neon-indicator-design.md`

---

### Task 1: 替换录音中 (recording) 的 UI

**Files:**
- Modify: `src/routes/recording/+page.svelte` (模板 + 样式)

- [ ] **Step 1: 替换录音中的模板部分**

将 `{#if phase === "recording"}` 块中的 `pulse-dot` span 替换为 SVG 脉冲环，添加 REC 标签：

```svelte
  {#if phase === "recording"}
    <div class="status">
      <svg class="neon-ring" width="24" height="24" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="8" fill="none" stroke="#ff0055" stroke-width="1.5">
          <animate attributeName="r" values="7;9;7" dur="1.2s" repeatCount="indefinite"/>
          <animate attributeName="opacity" values="1;0.5;1" dur="1.2s" repeatCount="indefinite"/>
        </circle>
        <circle cx="12" cy="12" r="3" fill="#ff0055"/>
      </svg>
      <span class="timer">{formatTime(seconds)}</span>
      <span class="rec-label">REC</span>
    </div>
```

- [ ] **Step 2: 替换录音中的样式**

删除 `.pulse-dot` 样式和 `@keyframes pulse`，替换为：

```css
  .neon-ring {
    filter: drop-shadow(0 0 4px rgba(255, 0, 85, 0.6));
    flex-shrink: 0;
  }

  .timer {
    font-family: -apple-system, BlinkMacSystemFont, "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 13px;
    color: #ff0055;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    min-width: 36px;
    text-align: center;
    text-shadow: 0 0 6px rgba(255, 0, 85, 0.4);
  }

  .rec-label {
    font-family: -apple-system, BlinkMacSystemFont, "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 8px;
    color: rgba(255, 0, 85, 0.4);
    letter-spacing: 2px;
  }
```

注意：这会替换掉已有的 `.timer` 样式块。

- [ ] **Step 3: 验证录音中阶段显示正常**

在浏览器中观察：品红脉冲环呼吸、计时器带品红辉光、REC 标签低透明度显示。

- [ ] **Step 4: 提交**

```bash
git add src/routes/recording/+page.svelte
git commit -m "feat: replace recording dot with neon pulse ring"
```

---

### Task 2: 替换处理中 (processing) 的 UI

**Files:**
- Modify: `src/routes/recording/+page.svelte` (模板 + 样式)

- [ ] **Step 1: 替换处理中的模板部分**

将 `{:else}` 块中的 `spinner` span 替换为水母 SVG：

```svelte
  {:else}
    <div class="status">
      <svg class="jellyfish" width="28" height="28" viewBox="0 0 28 28" style="overflow:visible">
        <defs>
          <filter id="jg-bell">
            <feGaussianBlur stdDeviation="2.5" result="b"/>
            <feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge>
          </filter>
          <filter id="jg-tent">
            <feGaussianBlur stdDeviation="1.5" result="b"/>
            <feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge>
          </filter>
          <radialGradient id="jbell-grad" cx="50%" cy="40%" r="50%">
            <stop offset="0%" style="stop-color:#00c8ff;stop-opacity:0.3"/>
            <stop offset="100%" style="stop-color:#00c8ff;stop-opacity:0.05"/>
          </radialGradient>
        </defs>
        <g class="jellyfish-body">
          <path class="jelly-bell" d="M5,16 Q5,5 14,5 Q23,5 23,16 Q19,19 14,17 Q9,19 5,16 Z"
                fill="url(#jbell-grad)" stroke="#00c8ff" stroke-width="1.2" filter="url(#jg-bell)">
            <animate attributeName="d"
              values="M5,16 Q5,5 14,5 Q23,5 23,16 Q19,19 14,17 Q9,19 5,16 Z;
                      M5,15 Q6,7 14,7 Q22,7 23,15 Q19,17 14,15 Q9,17 5,15 Z;
                      M5,16 Q5,5 14,5 Q23,5 23,16 Q19,19 14,17 Q9,19 5,16 Z"
              dur="2s" repeatCount="indefinite"/>
          </path>
          <ellipse cx="14" cy="12" rx="4" ry="3" fill="#00c8ff" opacity="0.15" filter="url(#jg-tent)">
            <animate attributeName="ry" values="3;2;3" dur="2s" repeatCount="indefinite"/>
          </ellipse>
          <path fill="none" stroke="#00c8ff" stroke-width="0.8" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.5">
            <animate attributeName="d"
              values="M8,16.5 Q6,22 8,27;M8,15.5 Q5,21 7,26;M8,16.5 Q6,22 8,27"
              dur="2.5s" repeatCount="indefinite"/>
          </path>
          <path fill="none" stroke="#00c8ff" stroke-width="1" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.7">
            <animate attributeName="d"
              values="M11,17 Q9,23 10,28;M11,16 Q8,22 9,27;M11,17 Q9,23 10,28"
              dur="2.2s" repeatCount="indefinite"/>
          </path>
          <path fill="none" stroke="#00c8ff" stroke-width="1.2" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.8">
            <animate attributeName="d"
              values="M14,17.5 Q13,24 14,28;M14,16.5 Q12,23 13,27;M14,17.5 Q13,24 14,28"
              dur="2s" repeatCount="indefinite"/>
          </path>
          <path fill="none" stroke="#00c8ff" stroke-width="1" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.7">
            <animate attributeName="d"
              values="M17,17 Q19,23 18,28;M17,16 Q20,22 19,27;M17,17 Q19,23 18,28"
              dur="2.2s" repeatCount="indefinite"/>
          </path>
          <path fill="none" stroke="#00c8ff" stroke-width="0.8" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.5">
            <animate attributeName="d"
              values="M20,16.5 Q22,22 20,27;M20,15.5 Q23,21 21,26;M20,16.5 Q22,22 20,27"
              dur="2.5s" repeatCount="indefinite"/>
          </path>
        </g>
      </svg>
      <span class="label">处理中</span>
    </div>
  {/if}
```

注意：触手 `<path>` 不需要 `d` 属性，因为 `<animate>` 会立即覆盖它。但为了首次渲染，每个触手 path 需要一个初始 `d` 属性，值等于 animate 的第一个 values。上述代码中触手没有初始 `d`，需补上。修正后的触手代码（每个 path 加上 `d` 属性）：

```svelte
          <path d="M8,16.5 Q6,22 8,27" fill="none" stroke="#00c8ff" stroke-width="0.8" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.5">
            <animate attributeName="d"
              values="M8,16.5 Q6,22 8,27;M8,15.5 Q5,21 7,26;M8,16.5 Q6,22 8,27"
              dur="2.5s" repeatCount="indefinite"/>
          </path>
          <path d="M11,17 Q9,23 10,28" fill="none" stroke="#00c8ff" stroke-width="1" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.7">
            <animate attributeName="d"
              values="M11,17 Q9,23 10,28;M11,16 Q8,22 9,27;M11,17 Q9,23 10,28"
              dur="2.2s" repeatCount="indefinite"/>
          </path>
          <path d="M14,17.5 Q13,24 14,28" fill="none" stroke="#00c8ff" stroke-width="1.2" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.8">
            <animate attributeName="d"
              values="M14,17.5 Q13,24 14,28;M14,16.5 Q12,23 13,27;M14,17.5 Q13,24 14,28"
              dur="2s" repeatCount="indefinite"/>
          </path>
          <path d="M17,17 Q19,23 18,28" fill="none" stroke="#00c8ff" stroke-width="1" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.7">
            <animate attributeName="d"
              values="M17,17 Q19,23 18,28;M17,16 Q20,22 19,27;M17,17 Q19,23 18,28"
              dur="2.2s" repeatCount="indefinite"/>
          </path>
          <path d="M20,16.5 Q22,22 20,27" fill="none" stroke="#00c8ff" stroke-width="0.8" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.5">
            <animate attributeName="d"
              values="M20,16.5 Q22,22 20,27;M20,15.5 Q23,21 21,26;M20,16.5 Q22,22 20,27"
              dur="2.5s" repeatCount="indefinite"/>
          </path>
```

- [ ] **Step 2: 替换处理中的样式**

删除 `.spinner`、`@keyframes spin`、`.label` 样式，替换为：

```css
  .jellyfish {
    flex-shrink: 0;
  }

  .jellyfish-body {
    animation: jelly-swim 2s ease-in-out infinite;
  }

  @keyframes jelly-swim {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-2px); }
  }

  .label {
    font-family: -apple-system, BlinkMacSystemFont, "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 13px;
    color: #00c8ff;
    font-weight: 500;
    text-shadow: 0 0 6px rgba(0, 200, 255, 0.4);
  }
```

- [ ] **Step 3: 调整容器样式**

处理中阶段需要稍微宽一点的容器。将 `.indicator` 的 `width` 改为 `180px`，并在处理中阶段动态调整宽度。修改模板中 `<div class="indicator"` 的 class 绑定：

```svelte
<div
  class="indicator"
  class:processing={phase === "processing"}
  class:fade-out={!visible}
>
```

在样式中添加：

```css
  .indicator {
    /* ...existing... */
    width: 160px;
    height: 44px;
  }

  .indicator.processing {
    width: 180px;
  }
```

- [ ] **Step 4: 验证两个阶段切换正常**

在浏览器中观察：
1. 录音中：品红脉冲环 + 计时器 + REC 标签
2. 处理中：青色水母 SVG 动画（钟体收缩、触手飘动、整体浮动）+ "处理中" 辉光文字

- [ ] **Step 5: 提交**

```bash
git add src/routes/recording/+page.svelte
git commit -m "feat: replace spinner with neon jellyfish SVG animation"
```

---

### Task 3: 调整 Rust 端窗口尺寸

**Files:**
- Modify: `src-tauri/src/lib.rs:303-304, 306, 316`

- [ ] **Step 1: 更新窗口尺寸常量**

将 `lib.rs` 中 `show_indicator` 函数的窗口尺寸从 160x44 调整为 180x48：

在 `lib.rs` 第 303-304 行，将：
```rust
            let win_w = 160.0;
            let win_h = 44.0;
```
改为：
```rust
            let win_w = 180.0;
            let win_h = 48.0;
```

在第 316 行，将：
```rust
    .inner_size(160.0, 44.0)
```
改为：
```rust
    .inner_size(180.0, 48.0)
```

- [ ] **Step 2: 编译验证**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: 编译成功，无错误。

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: adjust indicator window size for neon animations"
```

---

### Task 4: 视觉调优与最终验证

**Files:**
- Modify: `src/routes/recording/+page.svelte` (微调)

- [ ] **Step 1: 端到端运行应用**

```bash
cargo tauri dev
```

触发录音快捷键，验证：
1. 浮动窗出现，显示品红脉冲环 + 计时器 + REC
2. 再按快捷键停止录音，切换到青色水母动画 + "处理中"
3. 处理完成后窗口淡出关闭
4. 取消按钮仍可正常点击

- [ ] **Step 2: 微调视觉参数**

根据实际渲染效果，可能需要微调：
- 水母 SVG 的 `viewBox` 大小或整体缩放
- 辉光 `filter` 的 `stdDeviation` 值
- 容器宽度是否足够容纳水母
- 字体大小和间距

如果一切正常无需修改，跳过此步。

- [ ] **Step 3: 最终提交**

```bash
git add -A
git commit -m "style: fine-tune neon indicator visual parameters"
```
