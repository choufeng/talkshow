# 快捷键徽章组件实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将快捷键展示从单一灰色文本容器改为 macOS 系统风格的独立按键徽章组件

**Architecture:** 新建 `KeyBadge` 原子组件负责单个按键的渲染，改造 `ShortcutRecorder` 组件消费它。通过 Tailwind CSS 的 `dark:` 变体实现 light/dark 模式切换。使用 Tauri 的平台检测 API 判断 macOS vs 其他系统。

**Tech Stack:** Svelte 5 (Runes), Tailwind CSS v4, Tauri v2

---

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `src/lib/components/ui/key-badge/index.svelte` | 单个按键徽章组件 |
| Modify | `src/lib/components/ui/shortcut-recorder/index.svelte` | 拆分快捷键为多个 KeyBadge |
| Modify | `src/app.css` | 添加按键徽章主题 CSS 变量 |

---

### Task 1: 添加按键徽章的 CSS 变量

**Files:**
- Modify: `src/app.css`

- [ ] **Step 1: 在 `:root` 和 `.dark` 中添加按键徽章专用 CSS 变量**

在 `src/app.css` 的 `:root` 块中（`--destructive` 之后）添加：

```css
  --key-bg-from: oklch(97% 0 0);
  --key-bg-to: oklch(91% 0.005 160);
  --key-border: oklch(75% 0.01 160 / 0.5);
  --key-text: oklch(25% 0 0);
```

在 `.dark` 块中（`--destructive` 之后）添加：

```css
  --key-bg-from: oklch(35% 0.005 160);
  --key-bg-to: oklch(28% 0.006 160);
  --key-border: oklch(45% 0.01 160 / 0.6);
  --key-text: oklch(85% 0.01 160);
```

在 `@theme inline` 块中添加对应的 Tailwind 颜色映射：

```css
  --color-key-bg-from: var(--key-bg-from);
  --color-key-bg-to: var(--key-bg-to);
  --color-key-border: var(--key-border);
  --color-key-text: var(--key-text);
```

- [ ] **Step 2: 运行 dev 验证 CSS 变量生效**

Run: `npm run dev` (检查无报错)

- [ ] **Step 3: Commit**

```bash
git add src/app.css
git commit -m "feat: add CSS variables for keyboard badge component"
```

---

### Task 2: 创建 KeyBadge 组件

**Files:**
- Create: `src/lib/components/ui/key-badge/index.svelte`

- [ ] **Step 1: 创建 KeyBadge 组件文件**

创建 `src/lib/components/ui/key-badge/index.svelte`：

```svelte
<script lang="ts">
  interface Props {
    label: string;
  }

  let { label }: Props = $props();
</script>

<span
  class="inline-flex items-center justify-center min-w-[40px] h-[40px] px-3
         rounded-md border-b-[2px]
         bg-gradient-to-b from-key-bg-from to-key-bg-to
         border-key-border
         shadow-[0_1px_2px_rgba(0,0,0,0.12),inset_0_1px_0_rgba(255,255,255,0.08)]
         dark:shadow-[0_1px_3px_rgba(0,0,0,0.3),inset_0_1px_0_rgba(255,255,255,0.05)]
         font-sans text-base text-key-text select-none"
>
  {label}
</span>
```

- [ ] **Step 2: 在设置页面临时验证组件渲染**

在 `src/routes/settings/+page.svelte` 中临时添加 KeyBadge 导入和使用来验证渲染效果：

```svelte
import KeyBadge from '$lib/components/ui/key-badge/index.svelte';
```

在页面底部临时添加：

```svelte
<div class="flex gap-2 mt-4">
  <KeyBadge label="⌃" />
  <KeyBadge label="⇧" />
  <KeyBadge label="'" />
</div>
```

Run: `npm run dev` 打开浏览器和设置页面确认渲染效果。分别检查 light 和 dark 模式。

- [ ] **Step 3: 移除临时验证代码**

删除 Step 2 中添加的临时导入和测试元素。

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/ui/key-badge/
git commit -m "feat: add KeyBadge component with macOS-style keyboard key appearance"
```

---

### Task 3: 改造 ShortcutRecorder 组件

**Files:**
- Modify: `src/lib/components/ui/shortcut-recorder/index.svelte`

- [ ] **Step 1: 添加 KeyBadge 导入和平台检测**

在 `<script lang="ts">` 顶部添加导入：

```typescript
import KeyBadge from '$lib/components/ui/key-badge/index.svelte';
```

在 `formatShortcut` 函数之前添加平台检测和键名解析工具函数：

```typescript
const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.userAgent);

const MODIFIER_DISPLAY: Record<string, string> = isMac
  ? { Control: '⌃', Shift: '⇧', Alt: '⌥', Command: '⌘', Super: '⌘' }
  : { Control: 'Ctrl', Shift: 'Shift', Alt: 'Alt', Command: 'Cmd', Super: 'Cmd' };

const KEY_DISPLAY: Record<string, string> = {
  Quote: "'",
  Backslash: '\\',
  Space: 'Space',
};

function parseKeys(shortcut: string): string[] {
  return shortcut.split('+').map((key) => {
    if (MODIFIER_DISPLAY[key]) return MODIFIER_DISPLAY[key];
    if (KEY_DISPLAY[key]) return KEY_DISPLAY[key];
    return key.replace('Key', '').replace('Digit', '');
  });
}
```

- [ ] **Step 2: 替换快捷键展示区域**

将第 105 行的快捷键展示 `<div>`：

```svelte
<div class="rounded-md px-5 py-2.5 font-mono text-base min-w-[130px] text-center {isRecording ? 'bg-accent text-accent-foreground' : 'bg-muted text-foreground'}">
  {#if isRecording}
    请按下快捷键...
  {:else}
    {formatShortcut(currentValue)}
  {/if}
</div>
```

替换为：

```svelte
{#if isRecording}
  <div class="rounded-md px-5 py-2.5 text-base min-w-[130px] text-center bg-accent text-accent-foreground">
    请按下快捷键...
  </div>
{:else}
  <div class="flex items-center gap-2">
    {#each parseKeys(currentValue) as key}
      <KeyBadge label={key} />
    {/each}
  </div>
{/if}
```

- [ ] **Step 3: 删除旧的 `formatShortcut` 函数**

删除第 19-31 行的 `formatShortcut` 函数（已被 `parseKeys` 取代）。

- [ ] **Step 4: 运行 dev 验证完整效果**

Run: `npm run dev`

验证清单：
1. 设置页面中两个快捷键区域均显示为独立按键徽章
2. 窗口切换显示 `⌃` `⇧` `'` 三个独立徽章
3. 录音控制显示 `⌃` `\` 两个独立徽章
4. 点击"修改"进入录制状态，显示"请按下快捷键..."提示
5. 按下新快捷键组合后，徽章正确更新
6. Light 和 Dark 模式均正确渲染

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/ui/shortcut-recorder/index.svelte
git commit -m "feat: render shortcuts as individual key badges in ShortcutRecorder"
```

---

## Self-Review Checklist

- [x] **Spec coverage:** 每个设计规格项都有对应任务覆盖（KeyBadge 组件、平台自适应、Light/Dark 模式、按键拆分逻辑）
- [x] **Placeholder scan:** 无 TBD、TODO 或模糊描述
- [x] **Type consistency:** `parseKeys` 返回 `string[]`，`KeyBadge` 接收 `label: string`，类型一致
