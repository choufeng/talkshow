# TalkShow UI 重构实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 引入 bits-ui + TailwindCSS 对 TalkShow 桌面应用进行全量 UI 重构，建立 Design Token 体系，支持亮色/暗色双模式。

**Architecture:** 在 `feature/ui` 分支上工作。先搭建 TailwindCSS + Design Token 基础设施，再逐个迁移组件，最后改造页面布局。每个任务独立可提交。

**Tech Stack:** TailwindCSS v4, @tailwindcss/vite, bits-ui v2, tailwindcss-animate, lucide-svelte, SvelteKit 5

---

## File Structure

| 操作 | 文件路径 | 职责 |
|------|---------|------|
| Create | `src/app.css` | Design Tokens + Tailwind 导入 |
| Create | `src/lib/stores/theme.ts` | 主题状态管理（light/dark/system） |
| Create | `src/lib/components/ui/select/index.svelte` | bits-ui Select 封装组件 |
| Modify | `src/lib/components/PasswordInput.svelte` | 迁移到 Tailwind 样式 |
| Modify | `src/lib/components/TagInput.svelte` | 迁移到 Tailwind 样式 |
| Modify | `src/lib/components/ShortcutRecorder.svelte` | 迁移到 Tailwind 样式 |
| Delete | `src/lib/components/GroupedSelect.svelte` | 被 ui/select 替代 |
| Create | `src/lib/components/ui/tag-input/index.svelte` | TagInput Tailwind 版 |
| Create | `src/lib/components/ui/password-input/index.svelte` | PasswordInput Tailwind 版 |
| Create | `src/lib/components/ui/shortcut-recorder/index.svelte` | ShortcutRecorder Tailwind 版 |
| Modify | `src/routes/+layout.svelte` | Tailwind 重构布局 |
| Modify | `src/routes/+page.svelte` | Tailwind 重构首页 |
| Modify | `src/routes/settings/+page.svelte` | Tailwind 重构 + 外观设置 |
| Modify | `src/routes/models/+page.svelte` | Tailwind 重构 + bits-ui Select |
| Modify | `vite.config.js` | 添加 @tailwindcss/vite 插件 |
| Modify | `src/app.html` | 添加 dark class 支持 |

---

### Task 1: 安装依赖 + 配置 TailwindCSS

**Files:**
- Modify: `vite.config.js`
- Create: `src/app.css`

- [ ] **Step 1: 安装依赖**

```bash
npm install -D tailwindcss @tailwindcss/vite tailwindcss-animate
npm install bits-ui
```

- [ ] **Step 2: 配置 Vite 插件**

替换 `vite.config.js` 全部内容：

```js
import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [tailwindcss(), sveltekit()],

  clearScreen: false,
  server: {
    port: 1421,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
```

- [ ] **Step 3: 创建 app.css**

创建 `src/app.css`：

```css
@import "tailwindcss";
@plugin "tailwindcss-animate";

@custom-variant dark (&:is(.dark *));

@theme inline {
  --color-background: var(--background);
  --color-background-alt: var(--background-alt);
  --color-foreground: var(--foreground);
  --color-foreground-alt: var(--foreground-alt);
  --color-muted: var(--muted);
  --color-muted-foreground: var(--muted-foreground);
  --color-border: var(--border);
  --color-border-input: var(--border-input);
  --color-accent: var(--accent);
  --color-accent-foreground: var(--accent-foreground);
  --color-destructive: var(--destructive);

  --shadow-card: 0px 2px 0px 1px rgba(0, 0, 0, 0.04);
  --shadow-popover: 0px 7px 12px 3px rgba(0, 0, 0, 0.08);
  --shadow-mini: 0px 1px 0px 1px rgba(0, 0, 0, 0.04);

  --font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  --font-mono: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
}

:root {
  --background: hsl(0 0% 100%);
  --background-alt: hsl(0 0% 98%);
  --foreground: hsl(0 0% 9%);
  --foreground-alt: hsl(0 0% 32%);
  --muted: hsl(240 5% 96%);
  --muted-foreground: hsla(0 0% 9% / 0.4);
  --border: hsla(240 6% 10% / 0.1);
  --border-input: hsla(240 6% 10% / 0.17);
  --accent: hsl(204 94% 94%);
  --accent-foreground: hsl(204 80% 16%);
  --destructive: hsl(347 77% 50%);
}

.dark {
  --background: hsl(0 0% 5%);
  --background-alt: hsl(0 0% 8%);
  --foreground: hsl(0 0% 95%);
  --foreground-alt: hsl(0 0% 70%);
  --muted: hsl(240 4% 16%);
  --muted-foreground: hsla(0 0% 100% / 0.4);
  --border: hsla(0 0% 96% / 0.1);
  --border-input: hsla(0 0% 96% / 0.17);
  --accent: hsl(204 90% 90%);
  --accent-foreground: hsl(204 94% 94%);
  --destructive: hsl(350 89% 60%);
}

@layer base {
  *,
  ::after,
  ::before {
    border-color: var(--border);
  }

  body {
    font-family: var(--font-sans);
    background-color: var(--background);
    color: var(--foreground);
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }
}
```

- [ ] **Step 4: 在 layout 中导入 app.css**

在 `src/routes/+layout.svelte` 的 `<script>` 标签顶部添加导入：

```ts
import '../app.css';
```

- [ ] **Step 5: 验证构建**

```bash
npm run build
```

Expected: 构建成功，无错误。

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: add TailwindCSS v4 and bits-ui dependencies"
```

---

### Task 2: 创建主题 Store

**Files:**
- Create: `src/lib/stores/theme.ts`

- [ ] **Step 1: 创建 theme.ts**

创建 `src/lib/stores/theme.ts`：

```ts
import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export type Theme = 'light' | 'dark' | 'system';

function getSystemTheme(): 'light' | 'dark' {
  if (!browser) return 'light';
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function applyTheme(theme: Theme) {
  if (!browser) return;
  const resolved = theme === 'system' ? getSystemTheme() : theme;
  document.documentElement.classList.toggle('dark', resolved === 'dark');
}

function createThemeStore() {
  const initial: Theme = browser
    ? (localStorage.getItem('theme') as Theme) || 'system'
    : 'system';

  applyTheme(initial);

  const { subscribe, set } = writable<Theme>(initial);

  return {
    subscribe,
    set: (theme: Theme) => {
      if (browser) {
        localStorage.setItem('theme', theme);
      }
      applyTheme(theme);
      set(theme);
    },
    getResolved: (): 'light' | 'dark' => {
      const stored: Theme = browser
        ? (localStorage.getItem('theme') as Theme) || 'system'
        : 'system';
      return stored === 'system' ? getSystemTheme() : stored;
    }
  };
}

export const theme = createThemeStore();

if (browser) {
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    const current: Theme = (localStorage.getItem('theme') as Theme) || 'system';
    if (current === 'system') {
      applyTheme('system');
    }
  });
}
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/stores/theme.ts
git commit -m "feat: add theme store with light/dark/system support"
```

---

### Task 3: 创建 UI Select 组件（bits-ui 封装）

**Files:**
- Create: `src/lib/components/ui/select/index.svelte`

- [ ] **Step 1: 创建目录结构**

```bash
mkdir -p src/lib/components/ui/select
```

- [ ] **Step 2: 创建 GroupedSelect 封装组件**

创建 `src/lib/components/ui/select/index.svelte`：

```svelte
<script lang="ts">
  import { Select } from "bits-ui";
  import { ChevronDown, Check } from "lucide-svelte";

  interface Group {
    label: string;
    items: { value: string; label: string }[];
  }

  interface Props {
    value: string;
    groups: Group[];
    placeholder?: string;
    onChange: (value: string) => void;
  }

  let { value, groups, placeholder = "请选择", onChange }: Props = $props();

  function getDisplayLabel(): string {
    for (const group of groups) {
      for (const item of group.items) {
        if (item.value === value) return `${group.label} — ${item.label}`;
      }
    }
    return placeholder;
  }
</script>

<Select.Root
  type="single"
  {value}
  onValueChange={(v) => { if (v) onChange(v); }}
>
  <Select.Trigger
    class="flex h-9 w-full items-center justify-between rounded-md border border-border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-accent-foreground/20 focus:ring-offset-1 disabled:cursor-not-allowed disabled:opacity-50"
  >
    <span class="truncate">{getDisplayLabel()}</span>
    <ChevronDown class="h-4 w-4 shrink-0 opacity-50" />
  </Select.Trigger>
  <Select.Portal>
    <Select.Content
      class="z-50 max-h-64 min-w-[var(--bits-select-anchor-width)] w-[var(--bits-select-anchor-width)] rounded-lg border border-border bg-background p-1 text-foreground shadow-popover data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=top]:slide-in-from-bottom-2"
      position="popper"
      sideOffset={4}
    >
      <Select.ScrollUpButton class="flex h-4 items-center justify-center">
        <ChevronDown class="h-3 w-3 rotate-180" />
      </Select.ScrollUpButton>
      <Select.Viewport>
        {#each groups as group}
          <Select.Group>
            <Select.GroupHeading class="px-2 py-1.5 text-xs font-medium text-muted-foreground">
              {group.label}
            </Select.GroupHeading>
            {#each group.items as item}
              <Select.Item
                value={item.value}
                label={item.label}
                class="relative flex w-full cursor-default select-none items-center rounded py-1.5 pl-2 pr-8 text-sm outline-none data-highlighted:bg-muted data-highlighted:text-foreground data-disabled:pointer-events-none data-disabled:opacity-50"
              >
                {#snippet children({ selected })}
                  <span class="absolute right-2 flex h-3.5 w-3.5 items-center justify-center">
                    {#if selected}
                      <Check class="h-4 w-4" />
                    {/if}
                  </span>
                  {item.label}
                {/snippet}
              </Select.Item>
            {/each}
          </Select.Group>
        {/each}
      </Select.Viewport>
      <Select.ScrollDownButton class="flex h-4 items-center justify-center">
        <ChevronDown class="h-3 w-3" />
      </Select.ScrollDownButton>
    </Select.Content>
  </Select.Portal>
</Select.Root>
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/ui/select/
git commit -m "feat: create GroupedSelect component using bits-ui Select"
```

---

### Task 4: 迁移 PasswordInput 组件

**Files:**
- Create: `src/lib/components/ui/password-input/index.svelte`

- [ ] **Step 1: 创建目录**

```bash
mkdir -p src/lib/components/ui/password-input
```

- [ ] **Step 2: 创建 Tailwind 版 PasswordInput**

创建 `src/lib/components/ui/password-input/index.svelte`：

```svelte
<script lang="ts">
  import { Eye, EyeOff } from 'lucide-svelte';

  interface Props {
    value: string;
    placeholder?: string;
    onChange: (value: string) => void;
  }

  let { value, placeholder = '', onChange }: Props = $props();

  let visible = $state(false);

  function mask(val: string): string {
    if (!val) return '';
    return val.slice(0, 3) + '•'.repeat(Math.max(val.length - 3, 6));
  }
</script>

<div class="flex items-center gap-1">
  {#if visible}
    <input
      class="flex h-8 w-full rounded-md border border-border-input bg-background px-3 py-1 text-xs ring-offset-background file:border-0 file:bg-transparent file:text-xs file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
      type="text"
      {placeholder}
      {value}
      oninput={(e) => onChange((e.target as HTMLInputElement).value)}
    />
  {:else}
    <div class="flex h-8 flex-1 items-center rounded-md border border-border-input bg-background px-3 text-xs text-muted-foreground select-none">
      {mask(value)}
    </div>
  {/if}
  <button
    class="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border-input bg-background text-muted-foreground transition-colors hover:bg-muted"
    onclick={() => visible = !visible}
    title={visible ? '隐藏' : '显示'}
  >
    {#if visible}
      <Eye class="h-3.5 w-3.5" />
    {:else}
      <EyeOff class="h-3.5 w-3.5" />
    {/if}
  </button>
</div>
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/ui/password-input/
git commit -m "feat: create PasswordInput with TailwindCSS styling"
```

---

### Task 5: 迁移 TagInput 组件

**Files:**
- Create: `src/lib/components/ui/tag-input/index.svelte`

- [ ] **Step 1: 创建目录**

```bash
mkdir -p src/lib/components/ui/tag-input
```

- [ ] **Step 2: 创建 Tailwind 版 TagInput**

创建 `src/lib/components/ui/tag-input/index.svelte`：

```svelte
<script lang="ts">
  interface Props {
    tags: string[];
    onAdd: (tag: string) => void;
    onRemove: (tag: string) => void;
    placeholder?: string;
  }

  let { tags, onAdd, onRemove, placeholder = '添加...' }: Props = $props();

  let inputValue = $state('');
  let isInputVisible = $state(false);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      addTag();
    } else if (event.key === 'Escape') {
      isInputVisible = false;
      inputValue = '';
    }
  }

  function addTag() {
    const trimmed = inputValue.trim();
    if (trimmed && !tags.includes(trimmed)) {
      onAdd(trimmed);
      inputValue = '';
    }
  }

  function showInput() {
    isInputVisible = true;
  }
</script>

<div class="mt-1">
  <div class="flex flex-wrap gap-1 mb-1">
    {#each tags as tag}
      <span class="inline-flex items-center gap-1 rounded bg-accent px-2 py-0.5 text-[10px] text-accent-foreground">
        {tag}
        <button
          class="opacity-60 hover:opacity-100 transition-opacity"
          onclick={() => onRemove(tag)}
        >
          ✕
        </button>
      </span>
    {/each}
  </div>
  {#if isInputVisible}
    <input
      class="flex h-7 w-full rounded-md border border-border-input bg-background px-2 py-1 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
      type="text"
      {placeholder}
      bind:value={inputValue}
      onkeydown={handleKeydown}
      onblur={() => { addTag(); isInputVisible = false; }}
    />
  {:else}
    <button
      class="text-xs text-accent-foreground hover:underline"
      onclick={showInput}
    >
      + 添加模型
    </button>
  {/if}
</div>
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/ui/tag-input/
git commit -m "feat: create TagInput with TailwindCSS styling"
```

---

### Task 6: 迁移 ShortcutRecorder 组件

**Files:**
- Create: `src/lib/components/ui/shortcut-recorder/index.svelte`

- [ ] **Step 1: 创建目录**

```bash
mkdir -p src/lib/components/ui/shortcut-recorder
```

- [ ] **Step 2: 创建 Tailwind 版 ShortcutRecorder**

创建 `src/lib/components/ui/shortcut-recorder/index.svelte`：

```svelte
<script lang="ts">
  interface Props {
    label: string;
    description: string;
    value: string;
    onUpdate: (shortcut: string) => Promise<void>;
  }

  let { label, description, value, onUpdate }: Props = $props();

  let isRecording = $state(false);
  let currentValue = $state(value);
  let error = $state<string | null>(null);

  $effect(() => {
    currentValue = value;
  });

  function formatShortcut(shortcut: string): string {
    return shortcut
      .replace('Control', '⌃')
      .replace('Shift', '⇧')
      .replace('Alt', '⌥')
      .replace('Command', '⌘')
      .replace('Super', '⌘')
      .replace('Quote', "'")
      .replace('Backslash', '\\')
      .replace('Key', '')
      .replace('Digit', '')
      .replace('+', ' ');
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!isRecording) return;

    event.preventDefault();
    event.stopPropagation();

    if (event.key === 'Escape') {
      isRecording = false;
      currentValue = value;
      return;
    }

    const modifiers: string[] = [];
    if (event.ctrlKey) modifiers.push('Control');
    if (event.shiftKey) modifiers.push('Shift');
    if (event.altKey) modifiers.push('Alt');
    if (event.metaKey) modifiers.push('Command');

    let key = event.code;
    if (key.startsWith('Key')) {
      key = key;
    } else if (key.startsWith('Digit')) {
      key = key;
    } else if (key === 'Quote') {
      key = 'Quote';
    } else if (key === 'Backslash') {
      key = 'Backslash';
    } else if (key === 'Space') {
      key = 'Space';
    } else {
      return;
    }

    if (modifiers.length === 0) {
      error = '请至少使用一个修饰键 (Ctrl, Shift, Alt, Command)';
      return;
    }

    const shortcut = [...modifiers, key].join('+');
    saveShortcut(shortcut);
  }

  async function saveShortcut(shortcut: string) {
    try {
      error = null;
      await onUpdate(shortcut);
      currentValue = shortcut;
      isRecording = false;
    } catch (e) {
      error = e instanceof Error ? e.message : '保存失败';
    }
  }

  function startRecording() {
    isRecording = true;
    error = null;
  }

  function cancelRecording() {
    isRecording = false;
    currentValue = value;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="rounded-lg border border-border bg-background p-5 mb-4">
  <div>
    <h4 class="text-sm font-semibold text-foreground m-0">{label}</h4>
    <p class="text-[13px] text-foreground-alt m-0 mt-1">{description}</p>
  </div>
  <div class="flex items-center gap-3 mt-3">
    <div class="rounded-md px-4 py-2 font-mono text-sm min-w-[120px] text-center {isRecording ? 'bg-accent text-accent-foreground' : 'bg-muted text-foreground'}">
      {#if isRecording}
        请按下快捷键...
      {:else}
        {formatShortcut(currentValue)}
      {/if}
    </div>
    {#if isRecording}
      <button
        class="inline-flex items-center justify-center rounded-md border border-accent-foreground bg-accent-foreground px-4 py-2 text-sm font-medium text-accent transition-colors hover:bg-accent-foreground/90"
        onclick={cancelRecording}
      >
        取消
      </button>
    {:else}
      <button
        class="inline-flex items-center justify-center rounded-md border border-border-input bg-background px-4 py-2 text-sm font-medium text-foreground transition-colors hover:bg-muted hover:border-foreground/20"
        onclick={startRecording}
      >
        修改
      </button>
    {/if}
  </div>
  {#if error}
    <p class="text-xs text-destructive m-0 mt-2">{error}</p>
  {/if}
</div>
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/ui/shortcut-recorder/
git commit -m "feat: create ShortcutRecorder with TailwindCSS styling"
```

---

### Task 7: 重构布局（+layout.svelte）

**Files:**
- Modify: `src/routes/+layout.svelte`

- [ ] **Step 1: 替换 +layout.svelte 全部内容**

```svelte
<script lang="ts">
  import '../app.css';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import type { Snippet } from 'svelte';
  import { House, Settings, Bot } from 'lucide-svelte';

  let { children }: { children: Snippet } = $props();

  let activeMenu = $derived(
    $page.url.pathname === '/settings' ? 'settings' :
    $page.url.pathname === '/models' ? 'models' : 'home'
  );

  function navigateTo(path: string) {
    goto(path);
  }
</script>

<div class="flex h-screen w-screen overflow-hidden">
  <aside class="w-40 bg-background-alt border-r border-border flex flex-col">
    <div class="px-5 py-4 font-semibold text-sm text-foreground border-b border-border">
      TalkShow
    </div>
    <nav class="py-2">
      <button
        class="flex items-center gap-2 px-5 py-2.5 w-full text-sm text-foreground text-left transition-colors {activeMenu === 'home' ? 'bg-muted border-l-[3px] border-l-accent-foreground' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/')}
      >
        <House size={18} class="shrink-0" />
        <span>首页</span>
      </button>
      <button
        class="flex items-center gap-2 px-5 py-2.5 w-full text-sm text-foreground text-left transition-colors {activeMenu === 'models' ? 'bg-muted border-l-[3px] border-l-accent-foreground' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/models')}
      >
        <Bot size={18} class="shrink-0" />
        <span>模型</span>
      </button>
      <button
        class="flex items-center gap-2 px-5 py-2.5 w-full text-sm text-foreground text-left transition-colors {activeMenu === 'settings' ? 'bg-muted border-l-[3px] border-l-accent-foreground' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/settings')}
      >
        <Settings size={18} class="shrink-0" />
        <span>设置</span>
      </button>
    </nav>
  </aside>
  <main class="flex-1 p-6 overflow-y-auto bg-background">
    {@render children()}
  </main>
</div>
```

注意：模型菜单图标从 emoji `🤖` 改为 lucide `Bot` 图标，保持一致性。

- [ ] **Step 2: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "refactor: migrate layout to TailwindCSS with design tokens"
```

---

### Task 8: 重构首页（+page.svelte）

**Files:**
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: 替换 +page.svelte 全部内容**

```svelte
<script lang="ts">
</script>

<div class="flex flex-col items-center justify-center h-full text-center">
  <h1 class="text-xl font-semibold text-foreground mb-2">欢迎使用 TalkShow</h1>
  <p class="text-sm text-foreground-alt">这是一个语音转文字应用</p>
</div>
```

- [ ] **Step 2: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "refactor: migrate home page to TailwindCSS"
```

---

### Task 9: 重构设置页 + 添加外观设置

**Files:**
- Modify: `src/routes/settings/+page.svelte`

- [ ] **Step 1: 替换 settings/+page.svelte 全部内容**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { Lightbulb, Sun, Moon, Monitor } from 'lucide-svelte';
  import { config } from '$lib/stores/config';
  import { theme, type Theme } from '$lib/stores/theme';
  import ShortcutRecorder from '$lib/components/ui/shortcut-recorder/index.svelte';

  onMount(() => {
    config.load();
  });

  let currentTheme = $state<Theme>('system');

  onMount(() => {
    const stored = localStorage.getItem('theme');
    if (stored === 'light' || stored === 'dark' || stored === 'system') {
      currentTheme = stored;
    }
  });

  function setTheme(t: Theme) {
    currentTheme = t;
    theme.set(t);
  }

  async function handleUpdateToggle(shortcut: string) {
    await config.updateShortcut('toggle', shortcut);
  }

  async function handleUpdateRecording(shortcut: string) {
    await config.updateShortcut('recording', shortcut);
  }

  const themeOptions: { value: Theme; label: string; icon: typeof Sun }[] = [
    { value: 'light', label: '浅色', icon: Sun },
    { value: 'dark', label: '深色', icon: Moon },
    { value: 'system', label: '系统', icon: Monitor },
  ];
</script>

<div class="max-w-[600px]">
  <h2 class="text-xl font-semibold text-foreground m-0 mb-6">设置</h2>

  <section class="mb-8">
    <div class="text-[11px] text-muted-foreground uppercase tracking-wider mb-2.5">快捷键</div>
    <ShortcutRecorder
      label="窗口切换"
      description="显示或隐藏主窗口"
      value={$config.shortcut}
      onUpdate={handleUpdateToggle}
    />
    <ShortcutRecorder
      label="录音控制"
      description="开始或结束录音"
      value={$config.recording_shortcut}
      onUpdate={handleUpdateRecording}
    />
    <div class="rounded-lg bg-accent/50 border border-accent p-4 mt-5">
      <p class="text-xs text-accent-foreground m-0">
        <Lightbulb size={14} class="inline -align-[2px] mr-1" />
        提示：点击"修改"按钮后，直接按下键盘上的组合键即可完成设置。按 Esc 取消录制。
      </p>
    </div>
  </section>

  <section>
    <div class="text-[11px] text-muted-foreground uppercase tracking-wider mb-2.5">外观</div>
    <div class="flex gap-2">
      {#each themeOptions as opt}
        {@const Icon = opt.icon}
        <button
          class="flex items-center gap-2 px-4 py-2.5 rounded-md border text-sm transition-colors {currentTheme === opt.value ? 'border-accent-foreground bg-accent text-accent-foreground' : 'border-border bg-background text-foreground hover:bg-muted'}"
          onclick={() => setTheme(opt.value)}
        >
          <Icon size={16} />
          {opt.label}
        </button>
      {/each}
    </div>
  </section>
</div>
```

- [ ] **Step 2: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "refactor: migrate settings page to TailwindCSS, add theme switcher"
```

---

### Task 10: 重构模型页（使用 bits-ui Select）

**Files:**
- Modify: `src/routes/models/+page.svelte`

- [ ] **Step 1: 替换 models/+page.svelte 全部内容**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { config } from '$lib/stores/config';
  import GroupedSelect from '$lib/components/ui/select/index.svelte';
  import TagInput from '$lib/components/ui/tag-input/index.svelte';
  import PasswordInput from '$lib/components/ui/password-input/index.svelte';
  import type { ProviderConfig, AppConfig } from '$lib/stores/config';

  onMount(() => {
    config.load();
  });

  function buildTranscriptionGroups() {
    return ($config.ai.providers || []).map((p: ProviderConfig) => ({
      label: p.name,
      items: (p.models || []).map((m: string) => ({
        value: `${p.id}::${m}`,
        label: m
      }))
    }));
  }

  function getTranscriptionValue(): string {
    const t = $config.features.transcription;
    if (t.provider_id && t.model) {
      return `${t.provider_id}::${t.model}`;
    }
    return '';
  }

  function handleTranscriptionChange(val: string) {
    const [providerId, model] = val.split('::');
    const newConfig: AppConfig = {
      ...$config,
      features: {
        transcription: { provider_id: providerId, model }
      }
    };
    config.save(newConfig);
  }

  function handleProviderFieldChange(
    providerId: string,
    field: string,
    value: string
  ) {
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === providerId ? { ...p, [field]: value } : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function handleApiKeyChange(providerId: string, value: string) {
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === providerId ? { ...p, api_key: value } : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function handleAddModel(providerId: string, model: string) {
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === providerId
        ? { ...p, models: [...p.models, model] }
        : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function handleRemoveModel(providerId: string, model: string) {
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === providerId
        ? { ...p, models: p.models.filter((m: string) => m !== model) }
        : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function handleRemoveProvider(providerId: string) {
    const newProviders = $config.ai.providers.filter(
      (p: ProviderConfig) => p.id !== providerId
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function needsApiKey(provider: ProviderConfig): boolean {
    return provider.type === 'openai-compatible';
  }
</script>

<div class="max-w-[800px]">
  <h2 class="text-xl font-semibold text-foreground m-0 mb-6">模型</h2>

  <section class="mb-7">
    <div class="text-[11px] text-muted-foreground uppercase tracking-wider mb-2.5">Features</div>
    <div class="grid grid-cols-3 gap-3">
      <div class="rounded-lg border border-border bg-background p-3.5">
        <div class="text-[13px] font-semibold text-foreground mb-0.5">Transcription</div>
        <div class="text-[11px] text-foreground-alt mb-2.5">转写服务</div>
        <GroupedSelect
          value={getTranscriptionValue()}
          groups={buildTranscriptionGroups()}
          placeholder="选择模型"
          onChange={handleTranscriptionChange}
        />
      </div>
    </div>
  </section>

  <section>
    <div class="text-[11px] text-muted-foreground uppercase tracking-wider mb-2.5">Providers</div>
    <div class="grid grid-cols-2 gap-3">
      {#each $config.ai.providers || [] as provider (provider.id)}
        <div class="rounded-lg border border-border bg-background p-3.5">
          <div class="flex justify-between items-start mb-3">
            <div>
              <div class="text-sm font-semibold text-foreground">{provider.name}</div>
              <div class="text-[10px] text-muted-foreground mt-0.5">{provider.id}</div>
            </div>
            <button
              class="text-xs text-muted-foreground hover:text-destructive transition-colors p-0.5"
              onclick={() => handleRemoveProvider(provider.id)}
            >
              ✕
            </button>
          </div>

          {#if needsApiKey(provider)}
            <div class="mb-2.5">
              <label class="block text-[11px] text-foreground-alt mb-1">API Key</label>
              <PasswordInput
                value={provider.api_key || ''}
                placeholder="sk-..."
                onChange={(val: string) => handleApiKeyChange(provider.id, val)}
              />
            </div>
          {/if}

          <div class="mb-2.5">
            <label class="block text-[11px] text-foreground-alt mb-1">Endpoint</label>
            <input
              class="flex h-8 w-full rounded-md border border-border-input bg-background px-3 py-1 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
              type="text"
              value={provider.endpoint}
              onchange={(e) => handleProviderFieldChange(provider.id, 'endpoint', (e.target as HTMLInputElement).value)}
            />
          </div>

          <div>
            <label class="block text-[11px] text-foreground-alt mb-1">Models</label>
            <TagInput
              tags={provider.models}
              onAdd={(tag: string) => handleAddModel(provider.id, tag)}
              onRemove={(tag: string) => handleRemoveModel(provider.id, tag)}
            />
          </div>
        </div>
      {/each}
    </div>
  </section>
</div>
```

- [ ] **Step 2: Commit**

```bash
git add src/routes/models/+page.svelte
git commit -m "refactor: migrate models page to TailwindCSS with bits-ui Select"
```

---

### Task 11: 清理旧组件 + 验证构建

**Files:**
- Delete: `src/lib/components/GroupedSelect.svelte`
- Delete: `src/lib/components/PasswordInput.svelte`
- Delete: `src/lib/components/TagInput.svelte`
- Delete: `src/lib/components/ShortcutRecorder.svelte`

- [ ] **Step 1: 删除旧组件文件**

```bash
rm src/lib/components/GroupedSelect.svelte
rm src/lib/components/PasswordInput.svelte
rm src/lib/components/TagInput.svelte
rm src/lib/components/ShortcutRecorder.svelte
```

- [ ] **Step 2: 运行构建验证**

```bash
npm run build
```

Expected: 构建成功，无编译错误。

- [ ] **Step 3: 运行类型检查**

```bash
npm run check
```

Expected: 无类型错误。如果有导入路径问题，修复后重新运行。

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: remove legacy components after migration to ui/ directory"
```

---

### Task 12: 更新 app.html 支持暗色模式

**Files:**
- Modify: `src/app.html`

- [ ] **Step 1: 更新 app.html**

替换 `src/app.html` 全部内容：

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="utf-8" />
    <link rel="icon" href="%sveltekit.assets%/favicon.png" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>TalkShow</title>
    %sveltekit.head%
  </head>
  <body data-sveltekit-preload-data="hover">
    <div style="display: contents">%sveltekit.body%</div>
    <script>
      (function() {
        var theme = localStorage.getItem('theme') || 'system';
        var dark = theme === 'dark' || (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
        if (dark) document.documentElement.classList.add('dark');
      })();
    </script>
  </body>
</html>
```

关键点：
- `lang` 改为 `zh-CN`
- 底部 `<script>` 在页面渲染前同步读取主题并应用 `dark` class，避免 FOUC（闪烁）

- [ ] **Step 2: Commit**

```bash
git add src/app.html
git commit -m "feat: add dark mode flash prevention in app.html"
```

---

## Self-Review Checklist

- [x] **Spec coverage:** Design Token 体系 → Task 1；主题 store → Task 2；Select → Task 3；PasswordInput → Task 4；TagInput → Task 5；ShortcutRecorder → Task 6；布局 → Task 7；首页 → Task 8；设置页 → Task 9；模型页 → Task 10；清理 → Task 11；暗色防闪烁 → Task 12
- [x] **Placeholder scan:** 无 TBD、TODO、占位符
- [x] **Type consistency:** 所有组件接口与现有 config store 的类型引用一致
