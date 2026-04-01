# AI 功能模块微重构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 提取 AI 转写和 AI 翻译模块中的重复代码为共享模块，按功能域重组目录结构。

**Architecture:** 创建 `src/lib/ai/shared/` 存放 AI 共享逻辑，`src/lib/utils/` 存放通用工具，`src/lib/hooks/` 存放状态管理 hooks，新增 UI 组件。逐步替换现有页面中的重复代码。

**Tech Stack:** Svelte 5 (runes), TypeScript, Tauri API, TailwindCSS

---

### Task 1: 创建目录结构和类型定义

**Files:**
- Create: `src/lib/ai/shared/types.ts`
- Create: `src/lib/ai/shared/index.ts`
- Create: `src/lib/ai/transcription/.gitkeep`
- Create: `src/lib/ai/translation/.gitkeep`

从 `src/routes/models/+page.svelte` 和 `src/routes/skills/+page.svelte` 中提取共享类型：

```typescript
// src/lib/ai/shared/types.ts

export interface Skill {
  id: string;
  name: string;
  description: string;
  prompt: string;
  enabled: boolean;
}

export interface ProviderConfig {
  id: string;
  name: string;
  type: string;
  endpoint: string;
  api_key?: string;
  models?: string[];
  last_tested_at?: string;
  last_test_result?: string;
}

export interface TranscriptionConfig {
  provider_id: string;
  model: string;
  polish_enabled: boolean;
  polish_provider_id: string;
  polish_model: string;
}

export interface TranslationConfig {
  provider_id: string;
  model: string;
}

export interface RecordingConfig {
  auto_mute: boolean;
}

export interface SkillsConfig {
  enabled: boolean;
  skills: Skill[];
}

export interface FeaturesConfig {
  transcription: TranscriptionConfig;
  translation: TranslationConfig;
  recording: RecordingConfig;
  skills: SkillsConfig;
}

export interface AppConfig {
  features: FeaturesConfig;
  ai: {
    providers: ProviderConfig[];
  };
}
```

```typescript
// src/lib/ai/shared/index.ts
export * from './types';
export * from './config';
export * from './invoke';
```

```bash
git add src/lib/ai/
git commit -m "refactor: 创建 AI 模块目录结构和共享类型"
```

---

### Task 2: 创建配置更新辅助函数

**Files:**
- Create: `src/lib/ai/shared/config.ts`

```typescript
// src/lib/ai/shared/config.ts

import type { AppConfig, FeaturesConfig } from './types';

/**
 * 不可变更新嵌套配置并返回新对象
 * 用法: updateFeature(config, 'transcription', t => ({ ...t, model: 'new-model' }))
 */
export function updateFeature<K extends keyof FeaturesConfig>(
  config: AppConfig,
  featureKey: K,
  updater: (feature: FeaturesConfig[K]) => FeaturesConfig[K]
): AppConfig {
  return {
    ...config,
    features: {
      ...config.features,
      [featureKey]: updater(config.features[featureKey])
    }
  };
}

/**
 * 深度更新嵌套对象路径
 * 用法: updateNestedPath(config, ['ai', 'providers'], providers => [...providers, newProvider])
 */
export function updateNestedPath<T extends Record<string, unknown>>(
  obj: T,
  path: string[],
  updater: (value: unknown) => unknown
): T {
  const [key, ...rest] = path;
  if (rest.length === 0) {
    return { ...obj, [key]: updater(obj[key as keyof T]) };
  }
  return {
    ...obj,
    [key]: updateNestedPath(
      obj[key as keyof T] as Record<string, unknown>,
      rest,
      updater
    )
  } as T;
}
```

```bash
git add src/lib/ai/shared/config.ts
git commit -m "refactor: 添加配置更新辅助函数"
```

---

### Task 3: 创建 Tauri invoke 封装

**Files:**
- Create: `src/lib/ai/shared/invoke.ts`

```typescript
// src/lib/ai/shared/invoke.ts

import { invoke } from '@tauri-apps/api/core';

export interface InvokeOptions {
  onError?: (error: Error) => void;
}

/**
 * 带统一错误处理的 Tauri invoke 封装
 * 成功时返回值，失败时调用 onError 并返回 null
 */
export async function invokeWithError<T>(
  command: string,
  args?: Record<string, unknown>,
  options: InvokeOptions = {}
): Promise<T | null> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    const error = e instanceof Error ? e : new Error(String(e));
    (options.onError ?? console.error)(error);
    return null;
  }
}
```

```bash
git add src/lib/ai/shared/invoke.ts
git commit -m "refactor: 添加带错误处理的 Tauri invoke 封装"
```

---

### Task 4: 创建工具函数模块

**Files:**
- Create: `src/lib/utils/format.ts`
- Create: `src/lib/utils/string.ts`
- Create: `src/lib/utils/index.ts`

```typescript
// src/lib/utils/format.ts

/**
 * 格式化总秒数为 MM:SS
 */
export function formatTime(totalSeconds: number): string {
  const mins = Math.floor(totalSeconds / 60);
  const secs = totalSeconds % 60;
  return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
}

/**
 * 格式化 ISO 时间戳为完整日期时间字符串
 */
export function formatTimestamp(ts: string): string {
  try {
    const d = new Date(ts);
    const pad = (n: number) => n.toString().padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  } catch {
    return ts;
  }
}

/**
 * 格式化 ISO 日期字符串为短日期 (MM/DD)
 */
export function formatDate(isoStr: string): string {
  try {
    return new Date(isoStr).toLocaleDateString(undefined, {
      month: '2-digit',
      day: '2-digit'
    });
  } catch {
    return '';
  }
}
```

```typescript
// src/lib/utils/string.ts

/**
 * 将名称转换为 URL 友好的 slug
 */
export function generateSlug(name: string): string {
  return name
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}
```

```typescript
// src/lib/utils/index.ts
export * from './format';
export * from './string';
```

```bash
git add src/lib/utils/
git commit -m "refactor: 提取格式化和字符串工具函数"
```

---

### Task 5: 创建 Dialog 状态管理 Hook

**Files:**
- Create: `src/lib/hooks/use-dialog-state.svelte.ts`
- Create: `src/lib/hooks/index.ts`

```typescript
// src/lib/hooks/use-dialog-state.svelte.ts

export interface DialogStateOptions {
  initialOpen?: boolean;
  onReset?: () => void;
}

export function createDialogState(options: DialogStateOptions = {}) {
  let isOpen = $state(options.initialOpen ?? false);
  let resetFn = $state<(() => void) | null>(options.onReset ?? null);

  return {
    get isOpen() {
      return isOpen;
    },
    open() {
      isOpen = true;
    },
    close() {
      isOpen = false;
      resetFn?.();
    },
    onOpenChange(open: boolean) {
      if (open) {
        isOpen = true;
      } else {
        this.close();
      }
    },
    setReset(fn: () => void) {
      resetFn = fn;
    }
  };
}
```

```typescript
// src/lib/hooks/index.ts
export { createDialogState } from './use-dialog-state.svelte';
export type { DialogStateOptions } from './use-dialog-state.svelte';
```

```bash
git add src/lib/hooks/
git commit -m "refactor: 添加 Dialog 状态管理 hook"
```

---

### Task 6: 创建 Toggle UI 组件

**Files:**
- Create: `src/lib/components/ui/toggle/index.svelte`

```svelte
<!-- src/lib/components/ui/toggle/index.svelte -->
<script lang="ts">
  interface Props {
    checked: boolean;
    onCheckedChange?: (checked: boolean) => void;
    size?: 'sm' | 'md';
    disabled?: boolean;
    ariaLabel?: string;
  }

  let { checked, onCheckedChange, size = 'md', disabled = false, ariaLabel }: Props = $props();

  const sizes = {
    sm: { button: 'h-5 w-9', thumb: 'h-3.5 w-3.5', translate: 'translate-x-4' },
    md: { button: 'h-6 w-11', thumb: 'h-4 w-4', translate: 'translate-x-5' }
  };

  function toggle() {
    if (disabled) return;
    onCheckedChange?.(!checked);
  }
</script>

<button
  class="relative inline-flex shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-2 {checked ? 'bg-gradient-to-b from-btn-primary-from to-btn-primary-to shadow-btn-primary' : 'bg-gradient-to-b from-toggle-off-from to-toggle-off-to shadow-btn-secondary'} {disabled ? 'cursor-not-allowed opacity-50' : ''} {sizes[size].button}"
  role="switch"
  aria-checked={checked}
  aria-label={ariaLabel}
  disabled={disabled}
  onclick={toggle}
>
  <span class="pointer-events-none inline-block transform rounded-full bg-gradient-to-b from-toggle-thumb-from to-toggle-thumb-to shadow ring-0 transition duration-200 ease-in-out {checked ? sizes[size].translate : 'translate-x-0'} {sizes[size].thumb}"></span>
</button>
```

```bash
git add src/lib/components/ui/toggle/
git commit -m "refactor: 添加统一 Toggle UI 组件"
```

---

### Task 7: 创建 Dialog Footer UI 组件

**Files:**
- Create: `src/lib/components/ui/dialog-footer/index.svelte`

```svelte
<!-- src/lib/components/ui/dialog-footer/index.svelte -->
<script lang="ts">
  interface Props {
    onCancel: () => void;
    onConfirm: () => void;
    cancelText?: string;
    confirmText?: string;
    confirmDisabled?: boolean;
    confirmVariant?: 'primary' | 'danger';
  }

  let {
    onCancel,
    onConfirm,
    cancelText = '取消',
    confirmText = '确认',
    confirmDisabled = false,
    confirmVariant = 'primary'
  }: Props = $props();

  const confirmClasses = confirmVariant === 'danger'
    ? 'rounded-md bg-red-600 px-4 py-2 text-body text-white hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed'
    : 'rounded-md bg-gradient-to-b from-btn-primary-from to-btn-primary-to px-4 py-2 text-body text-white hover:opacity-90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed shadow-btn-primary';
</script>

<div class="flex justify-end gap-2 mt-4">
  <button
    class="rounded-md border border-btn-secondary-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to px-4 py-2 text-body text-accent-foreground hover:bg-muted transition-colors shadow-btn-secondary"
    onclick={onCancel}
  >
    {cancelText}
  </button>
  <button
    class={confirmClasses}
    disabled={confirmDisabled}
    onclick={onConfirm}
  >
    {confirmText}
  </button>
</div>
```

```bash
git add src/lib/components/ui/dialog-footer/
git commit -m "refactor: 添加统一 Dialog Footer 组件"
```

---

### Task 8: 替换 models/+page.svelte 中的重复代码

**Files:**
- Modify: `src/routes/models/+page.svelte`

需要替换的模式：
1. 导入新模块
2. 替换 `formatTestDate` 为 `formatDate`
3. 替换 `generateSlug` 为导入版本
4. 替换内联的 `onOpenChange` 处理器为 `createDialogState`
5. 替换配置更新逻辑为 `updateFeature`
6. 替换 `invoke` 调用为 `invokeWithError`
7. 替换 Toggle 按钮为 `<Toggle>` 组件
8. 替换 Dialog footer 为 `<DialogFooter>` 组件

关键修改点（行号参考原始文件）：
- L1-8: 添加新导入
- L20: 移除 `formErrors`，改用 hook
- L118-216: 配置更新函数简化
- L309-315: 删除 `generateSlug`
- L380-386: 删除 `handleDialogOpenChange`
- L388-394: 删除 `formatTestDate`
- L472-479: 替换 Toggle
- L783-922: 替换 Dialog footers

```bash
git add src/routes/models/+page.svelte
git commit -m "refactor: models 页面使用共享模块替换重复代码"
```

---

### Task 9: 替换 skills/+page.svelte 中的重复代码

**Files:**
- Modify: `src/routes/skills/+page.svelte`

需要替换的模式：
1. 导入新模块
2. 替换 `handleEditDialogClose` 和 `handleDeleteDialogClose` 为 `createDialogState`
3. 替换配置更新逻辑为 `updateFeature`
4. 替换 `invoke` 调用为 `invokeWithError`
5. 替换 Toggle 按钮为 `<Toggle>` 组件
6. 替换 Dialog footer 为 `<DialogFooter>` 组件

```bash
git add src/routes/skills/+page.svelte
git commit -m "refactor: skills 页面使用共享模块替换重复代码"
```

---

### Task 10: 替换 settings/+page.svelte 中的重复代码

**Files:**
- Modify: `src/routes/settings/+page.svelte`

需要替换的模式：
1. 导入 `updateFeature`
2. 替换内联的配置更新为 `updateFeature`
3. 替换 Toggle 按钮为 `<Toggle>` 组件

```bash
git add src/routes/settings/+page.svelte
git commit -m "refactor: settings 页面使用共享模块替换重复代码"
```

---

### Task 11: 替换 recording/+page.svelte 中的重复代码

**Files:**
- Modify: `src/routes/recording/+page.svelte`

需要替换的模式：
1. 导入 `formatTime`
2. 删除本地 `formatTime` 函数
3. 替换 `invoke` 为 `invokeWithError`

```bash
git add src/routes/recording/+page.svelte
git commit -m "refactor: recording 页面使用共享模块替换重复代码"
```

---

### Task 12: 替换 logs/+page.svelte 中的重复代码

**Files:**
- Modify: `src/routes/logs/+page.svelte`

需要替换的模式：
1. 导入 `formatTimestamp`
2. 删除本地 `formatTimestamp` 函数

```bash
git add src/routes/logs/+page.svelte
git commit -m "refactor: logs 页面使用共享模块替换重复代码"
```

---

### Task 13: 验证和清理

1. 运行 `npm run check` 确保 TypeScript 类型检查通过
2. 运行 `npm run build` 确保构建成功
3. 手动测试各页面功能正常
4. 检查是否有未使用的导入
5. 确认 `.gitkeep` 文件是否需要保留或删除

```bash
npm run check && npm run build
git add -A
git commit -m "refactor: 验证所有页面功能正常，清理残留代码"
```
