# 转写/润色模型选择框分离 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复转写服务设置中润色模型选择框错误显示转写模型的问题，实现按能力正确过滤

**Architecture:** 新增独立的 `buildPolishGroups()` 函数过滤具有 `chat` 能力的模型，润色选择框改用新函数；同时为内置 Gemini 模型添加 `chat` 能力标记

**Tech Stack:** Svelte, TypeScript

---

### Task 1: 扩展模型能力定义

**Files:**
- Modify: `src/lib/stores/config.ts:64-66`

- [ ] **Step 1: 添加 chat 能力到 MODEL_CAPABILITIES**

修改 `src/lib/stores/config.ts` 第 64-66 行：

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

- [ ] **Step 2: 验证修改**

运行类型检查确保无语法错误：
```bash
npx svelte-check --threshold warning 2>&1 | head -20
```

---

### Task 2: 更新内置模型能力标记

**Files:**
- Modify: `src/lib/stores/config.ts:68-91`

- [ ] **Step 1: 为 Vertex AI Gemini 添加 chat 能力**

修改 `src/lib/stores/config.ts` 中 BUILTIN_PROVIDERS 定义：

```typescript
// 修改前（第 74 行）
models: [{ name: 'gemini-2.0-flash', capabilities: ['transcription'] }]

// 修改后
models: [{ name: 'gemini-2.0-flash', capabilities: ['transcription', 'chat'] }]
```

阿里云 Qwen-Audio 和 SenseVoice 保持不变（只有 `transcription` 能力）。

- [ ] **Step 2: 验证修改**

确认文件语法正确：
```bash
npx svelte-check --threshold warning 2>&1 | head -20
```

---

### Task 3: 新增 buildPolishGroups 函数

**Files:**
- Modify: `src/routes/models/+page.svelte:84-94`

- [ ] **Step 1: 在 buildTranscriptionGroups 后添加新函数**

在 `src/routes/models/+page.svelte` 第 94 行后添加：

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

- [ ] **Step 2: 验证修改**

```bash
npx svelte-check --threshold warning 2>&1 | head -20
```

---

### Task 4: 润色选择框改用新函数

**Files:**
- Modify: `src/routes/models/+page.svelte:441-446`

- [ ] **Step 1: 修改润色选择框的 groups 属性**

修改 `src/routes/models/+page.svelte` 第 443 行：

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

- [ ] **Step 2: 验证修改**

```bash
npx svelte-check --threshold warning 2>&1 | head -20
```

- [ ] **Step 3: 构建验证**

```bash
npm run build 2>&1 | tail -20
```

---

### Task 5: 提交变更

- [ ] **Step 1: 检查变更状态**

```bash
git status
git diff
```

- [ ] **Step 2: 提交**

```bash
git add src/lib/stores/config.ts src/routes/models/+page.svelte
git commit -m "fix: 分离转写/润色模型选择框，润色列表只显示chat能力模型"
```

---

## 验证清单

完成所有任务后，手动验证：

1. 打开模型设置页面
2. 转写模型选择框应显示：gemini-2.0-flash、qwen2-audio-instruct、SenseVoice-Small
3. 启用润色开关
4. 润色模型选择框应只显示：gemini-2.0-flash
5. SenseVoice 不应出现在润色列表中
6. 阿里云 Qwen-Audio 不应出现在润色列表中
