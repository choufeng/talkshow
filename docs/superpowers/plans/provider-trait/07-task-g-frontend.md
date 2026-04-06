# 子任务 G: 前端更新 — 类型、Models 页面和 Onboarding

> **依赖**: 无 (可与后端子任务并行) | **阶段**: Phase 3 | **复杂度**: 高

## 目标

1. 更新 TypeScript `ProviderConfig` 类型（移除 `type` 和 `endpoint`）
2. 更新 Models 页面（移除 Add Provider 对话框、Endpoint 显示）
3. 简化 Onboarding Provider Config 步骤

## 涉及文件

| 文件 | 操作 | 变更摘要 |
|------|------|----------|
| `src/lib/stores/config.ts` | 修改 | 简化 ProviderConfig 接口 |
| `src/routes/models/+page.svelte` | 修改 | 移除 Add Provider, Endpoint |
| `src/lib/components/onboarding/steps/ProviderConfigStep.svelte` | 修改 | 移除 Add Provider, 简化 |

## 步骤

### Part 1: 更新 `config.ts`

- [ ] **Step 1: 更新 `ProviderConfig` 接口**

```typescript
export interface ProviderConfig {
  id: string;
  name: string;
  api_key?: string;
  models: ModelConfig[];
}
```

移除 `type` 和 `endpoint` 字段。

- [ ] **Step 2: 更新 `BUILTIN_PROVIDERS`**

移除 `type` 和 `endpoint`：

```typescript
export const BUILTIN_PROVIDERS: ProviderConfig[] = [
  {
    id: 'vertex',
    name: 'Vertex AI',
    models: [{ name: 'gemini-2.0-flash', capabilities: ['transcription', 'chat'] }]
  },
  {
    id: 'dashscope',
    name: '阿里云',
    api_key: '',
    models: [{ name: 'qwen2-audio-instruct', capabilities: ['transcription'] }]
  },
  {
    id: 'sensevoice',
    name: 'SenseVoice (本地)',
    models: [{ name: 'SenseVoice-Small', capabilities: ['transcription'] }]
  }
];
```

- [ ] **Step 3: 简化 `mergeBuiltinProviders`**

移除 type/endpoint 修正逻辑，只保留 name 修正和缺失 provider 补充。

- [ ] **Step 4: 添加辅助常量**

```typescript
export const PROVIDERS_REQUIRING_KEY = ['dashscope'];
export function needsApiKey(providerId: string): boolean {
  return PROVIDERS_REQUIRING_KEY.includes(providerId);
}
```

### Part 2: 更新 `models/+page.svelte`

- [ ] **Step 5: 移除 Add Provider 相关代码**

删除以下内容：
- `addProviderDialog` 状态
- `newProvider` 状态
- `formErrors` 状态
- `PROVIDER_TYPES` 常量
- `handleNameInput`, `handleTypeChange`, `validateEndpointUrl`, `validateForm`, `handleAddProvider` 函数
- `handleProviderFieldChange` 函数
- "添加 Provider" 按钮
- 整个 Add Provider `<Dialog>` 组件
- `Plus` 图标的 import
- `generateSlug` 的 import

- [ ] **Step 6: 移除 Endpoint 显示**

删除 Endpoint 的 `EditableField` 显示块（`provider.type !== 'sensevoice'` 的条件分支）。

- [ ] **Step 7: 更新 needsApiKey 检查**

将本地 `needsApiKey(provider)` 函数改为使用 `PROVIDERS_REQUIRING_KEY`：

```typescript
import { PROVIDERS_REQUIRING_KEY } from '$lib/stores/config';

function needsApiKey(provider: ProviderConfig): boolean {
  return PROVIDERS_REQUIRING_KEY.includes(provider.id);
}
```

- [ ] **Step 8: 替换所有 `provider.type` 为 `provider.id`**

搜索替换：
- `provider.type === 'sensevoice'` → `provider.id === 'sensevoice'`
- `provider.type === 'vertex'` → `provider.id === 'vertex'`
- `provider.type !== 'sensevoice'` → `provider.id !== 'sensevoice'`

- [ ] **Step 9: 移除非内置 provider 的删除按钮**

由于所有 provider 现在都是内置的，移除自定义 provider 的删除 (✕) 按钮分支。

- [ ] **Step 10: 更新重置对话框描述**

将 "确定要重置为默认设置吗？自定义的 Endpoint、API Key 和 Models 将被覆盖。" 改为 "确定要重置为默认设置吗？API Key 和自定义模型将被覆盖。"

### Part 3: 更新 `ProviderConfigStep.svelte`

- [ ] **Step 11: 移除 Add Provider 相关代码**

删除以下内容：
- `newProvider` 状态
- `formErrors` 状态
- `addProviderDialogOpen` 状态
- `PROVIDER_TYPES` 常量
- `handleNameInput`, `validateForm`, `handleAddProvider`, `openAddProviderDialog`, `closeAddProviderDialog` 函数
- "添加自定义 Provider" 按钮
- 整个 Add Provider `<Dialog>` 组件
- `Plus` 图标 import
- `generateSlug` import
- `handleProviderFieldChange` 函数

- [ ] **Step 12: 替换所有 `provider.type` 为 `provider.id`**

- `provider.type === 'vertex'` → `provider.id === 'vertex'`
- `provider.type === 'openai-compatible'` → `provider.id === 'dashscope'`
- `provider.type === 'sensevoice'` → `provider.id === 'sensevoice'`
- `provider.type !== 'sensevoice'` → `provider.id !== 'sensevoice'`
- `isSensevoice` → `provider.id === 'sensevoice'`
- `hasProviderWithApiKey` → 使用 `needsApiKey(provider.id)` 检查

- [ ] **Step 13: 更新手动配置链接文字**

将 "手动配置其他 Provider" 改为 "手动配置 API Key"。

- [ ] **Step 14: 验证前端构建**

```bash
npm run build
```

预期：无构建错误。

## 提交

建议分为三个提交（可选）：

```bash
# 提交 1: 类型更新
git add src/lib/stores/config.ts
git commit -m "refactor: simplify ProviderConfig type in frontend stores"

# 提交 2: Models 页面
git add src/routes/models/+page.svelte
git commit -m "refactor: remove Add Provider and Endpoint from models page"

# 提交 3: Onboarding
git add src/lib/components/onboarding/steps/ProviderConfigStep.svelte
git commit -m "refactor: simplify onboarding provider config, remove Add Provider"
```
