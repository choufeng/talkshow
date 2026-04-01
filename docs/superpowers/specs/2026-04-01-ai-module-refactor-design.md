# AI 功能模块微重构设计文档

**日期**: 2026-04-01
**状态**: 已批准
**范围**: 全局性改进

## 背景

项目中 AI 转写和 AI 翻译两个功能模块存在大量重复代码模式，包括：
- Dialog CRUD 状态管理（7+ 处重复）
- 配置不可变更新模式（8+ 处重复）
- Toggle 开关组件样式（3 处重复）
- 时间/日期格式化函数（3 处不同实现）
- Tauri invoke 错误处理（不统一）
- Dialog Footer 按钮样式（7+ 处重复）

## 目标

1. 提取重复逻辑为独立模块和纯函数
2. 按功能域重组目录结构
3. 确保未来新功能可以进行同样的链式调用

## 目录结构

```
src/lib/
├── ai/
│   ├── shared/
│   │   ├── config.ts            # 配置更新辅助函数
│   │   ├── invoke.ts            # Tauri invoke 封装
│   │   └── types.ts             # 共享类型定义
│   ├── transcription/           # AI 转写模块（预留）
│   └── translation/             # AI 翻译模块（预留）
├── utils/
│   ├── format.ts                # 时间/日期格式化
│   └── string.ts                # 字符串处理
├── hooks/
│   └── use-dialog-state.svelte.ts  # Dialog 状态管理
├── components/ui/
│   ├── toggle/                  # 新增
│   └── dialog-footer/           # 新增
└── stores/                      # 保持不变
```

## 模块设计

### src/lib/ai/shared/config.ts

```ts
import type { AppConfig, FeaturesConfig } from './types';

export function updateFeature<T>(
  config: AppConfig,
  featureKey: keyof FeaturesConfig,
  updater: (feature: FeaturesConfig[keyof FeaturesConfig]) => T
): AppConfig {
  return {
    ...config,
    features: {
      ...config.features,
      [featureKey]: updater(config.features[featureKey])
    }
  };
}

export function updateNestedPath<T>(
  obj: Record<string, unknown>,
  path: string[],
  updater: (value: unknown) => T
): Record<string, unknown> {
  const [key, ...rest] = path;
  if (rest.length === 0) {
    return { ...obj, [key]: updater(obj[key]) };
  }
  return {
    ...obj,
    [key]: updateNestedPath(obj[key] as Record<string, unknown>, rest, updater)
  };
}
```

### src/lib/ai/shared/invoke.ts

```ts
import { invoke } from '@tauri-apps/api/core';

export async function invokeWithError<T>(
  command: string,
  args?: Record<string, unknown>,
  onError?: (error: Error) => void
): Promise<T | null> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    const error = e instanceof Error ? e : new Error(String(e));
    (onError ?? console.error)(error);
    return null;
  }
}
```

### src/lib/ai/shared/types.ts

从 models/+page.svelte 和 skills/+page.svelte 中提取：
- `ProviderConfig`
- `Skill`
- `FeatureConfig` 子类型

### src/lib/utils/format.ts

```ts
export function formatTime(totalSeconds: number): string {
  const mins = Math.floor(totalSeconds / 60);
  const secs = totalSeconds % 60;
  return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
}

export function formatTimestamp(ts: string): string {
  try {
    const d = new Date(ts);
    const pad = (n: number) => n.toString().padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  } catch {
    return ts;
  }
}

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

### src/lib/utils/string.ts

```ts
export function generateSlug(name: string): string {
  return name
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}
```

### src/lib/hooks/use-dialog-state.svelte.ts

```ts
export function createDialogState(initialOpen = false) {
  let isOpen = $state(initialOpen);
  let resetFn = $state<(() => void) | null>(null);

  return {
    get isOpen() { return isOpen; },
    open() { isOpen = true; },
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

### src/lib/components/ui/toggle/index.svelte

- 支持 `size` prop: `'sm'` (h-5/w-9) | `'md'` (h-6/w-11)
- 支持 `checked` 双向绑定
- 统一 gradient 样式

### src/lib/components/ui/dialog-footer/index.svelte

- 接受 `onCancel`、`onConfirm` 回调
- 接受 `confirmText`、`cancelText`
- 接受 `confirmDisabled` 状态

## 迁移策略

1. 创建所有新文件和模块（不修改现有代码）
2. 逐个页面替换重复代码：
   - models/+page.svelte
   - skills/+page.svelte
   - settings/+page.svelte
   - recording/+page.svelte
   - logs/+page.svelte
3. 每替换一个页面就验证功能
4. 删除旧的内联函数和重复样式

## 成功标准

- 所有页面功能不变
- 重复代码消除率 > 80%
- 新增模块可独立测试
- 未来新功能可通过 import 复用这些模块
