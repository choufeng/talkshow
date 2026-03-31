# 转写/润色模型选择框分离设计

## 问题描述

当前转写服务设置页面中，"转写模型"和"润色模型"两个选择框使用相同的过滤函数 `buildTranscriptionGroups()`，导致：
- 润色模型选择框中只显示具有 `transcription` 能力的模型
- 纯转写模型（如 SenseVoice）错误地出现在润色列表中
- 具有文本对话能力的模型可能无法出现在润色列表中

## 目标

- 转写模型选择框：只显示具有 `transcription` 能力的模型
- 润色模型选择框：只显示具有 `chat` 或 `text_generation` 能力的模型

## 架构

### 数据流

```
ProviderConfig (models[])
  ├── capabilities: ['transcription']       → 转写模型选择框
  ├── capabilities: ['chat']                → 润色模型选择框
  └── capabilities: ['transcription', 'chat'] → 同时出现在两个选择框
```

### 变更范围

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src/lib/stores/config.ts` | 修改 | 添加 `chat` 能力选项，为内置 Gemini 模型添加 `chat` 能力 |
| `src/routes/models/+page.svelte` | 修改 | 新增 `buildPolishGroups()` 函数，润色选择框改用新函数 |

## 详细设计

### 1. 模型能力定义扩展

**文件**：`src/lib/stores/config.ts`

```typescript
// 修改前
export const MODEL_CAPABILITIES = [
  { value: 'transcription', label: '语音转文字' }
];

// 修改后
export const MODEL_CAPABILITIES = [
  { value: 'transcription', label: '语音转文字' },
  { value: 'chat', label: '文本对话' }
];
```

### 2. 内置模型能力更新

**文件**：`src/lib/stores/config.ts`

```typescript
// Vertex AI - Gemini 既是转写模型也是聊天模型
{
  id: 'vertex',
  type: 'vertex',
  name: 'Vertex AI',
  endpoint: '',
  models: [{ name: 'gemini-2.0-flash', capabilities: ['transcription', 'chat'] }]
}

// 阿里云 - Qwen-Audio 是纯音频转写模型
{
  id: 'dashscope',
  type: 'openai-compatible',
  name: '阿里云',
  endpoint: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
  api_key: '',
  models: [{ name: 'qwen2-audio-instruct', capabilities: ['transcription'] }]
}

// SenseVoice - 本地转写引擎
{
  id: 'sensevoice',
  type: 'sensevoice',
  name: 'SenseVoice (本地)',
  endpoint: '',
  models: [{ name: 'SenseVoice-Small', capabilities: ['transcription'] }]
}
```

### 3. 新增润色模型过滤函数

**文件**：`src/routes/models/+page.svelte`

```typescript
function buildPolishGroups() {
  return ($config.ai.providers || []).map((p: ProviderConfig) => ({
    label: p.name,
    items: (p.models || [])
      .filter((m: ModelConfig) => 
        m.capabilities.includes('chat') || m.capabilities.includes('text_generation')
      )
      .map((m: ModelConfig) => ({
        value: `${p.id}::${m.name}`,
        label: m.name
      }))
  }));
}
```

### 4. 润色选择框改用新函数

**文件**：`src/routes/models/+page.svelte`

```svelte
<!-- 修改前 -->
<GroupedSelect
  value={getPolishValue()}
  groups={buildTranscriptionGroups()}
  placeholder="选择模型"
  onChange={handlePolishChange}
/>

<!-- 修改后 -->
<GroupedSelect
  value={getPolishValue()}
  groups={buildPolishGroups()}
  placeholder="选择模型"
  onChange={handlePolishChange}
/>
```

## 边界情况处理

1. **空列表**：如果没有任何模型具有 `chat` 能力，润色选择框显示空列表（与现有行为一致）
2. **模型同时具有两种能力**：如 Gemini 同时出现在两个选择框中（这是正确的，因为它既能转写也能聊天）
3. **用户自定义模型**：用户可以通过模型管理页面为自定义模型添加 `chat` 能力

## 测试建议

1. 验证转写模型选择框只显示 `transcription` 模型
2. 验证润色模型选择框只显示 `chat` 模型
3. 验证 Gemini 同时出现在两个选择框中
4. 验证 SenseVoice 只出现在转写选择框中
5. 添加自定义模型并测试能力过滤
