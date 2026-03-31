# 模型配置页输入框交互改进实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 改进模型配置页中 API Key 和 Endpoint 输入框的交互体验，解决布局适配问题

**Architecture:** 将 PasswordInput 组件重命名为 EditableField，支持 password 和 text 两种模式，统一交互方式为点击进入编辑模式

**Tech Stack:** Svelte 5, TypeScript, Tailwind CSS

---

## 文件结构

- **组件:** `src/lib/components/ui/password-input/index.svelte` → 重命名为 `src/lib/components/ui/editable-field/index.svelte`
- **页面:** `src/routes/models/+page.svelte` - 更新导入和使用方式

---

### Task 1: 创建 EditableField 组件

**Files:**
- Create: `src/lib/components/ui/editable-field/index.svelte`
- Modify: `src/routes/models/+page.svelte`

- [ ] **Step 1: 创建 EditableField 组件**

```svelte
<script lang="ts">
  import { Eye, EyeOff, Check, X } from 'lucide-svelte';

  interface Props {
    value: string;
    placeholder?: string;
    mode?: 'password' | 'text';
    onChange: (value: string) => void;
  }

  let { value, placeholder = '', mode = 'password', onChange }: Props = $props();

  let visible = $state(false);
  let editing = $state(false);
  let editValue = $state(value);

  function mask(val: string): string {
    if (!val) return '';
    return val.slice(0, 3) + '•'.repeat(Math.max(val.length - 3, 6));
  }

  function startEdit() {
    editValue = value;
    editing = true;
  }

  function confirm() {
    onChange(editValue);
    editing = false;
  }

  function cancel() {
    editValue = value;
    editing = false;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') cancel();
    if (e.key === 'Enter') confirm();
  }
</script>

<div class="flex items-center gap-1 min-w-0">
  {#if editing}
    <input
      class="flex h-10 min-w-0 flex-1 rounded-md border border-border-input bg-background px-4 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-xs file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
      type={mode === 'password' && !visible ? 'password' : 'text'}
      bind:value={editValue}
      {placeholder}
      onkeydown={handleKeyDown}
    />
    <button
      class="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-border-input bg-green-500/10 text-green-500 transition-colors hover:bg-green-500/20"
      onclick={confirm}
      title="确认"
    >
      <Check class="h-4 w-4" />
    </button>
    <button
      class="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-border-input bg-red-500/10 text-red-500 transition-colors hover:bg-red-500/20"
      onclick={cancel}
      title="取消"
    >
      <X class="h-4 w-4" />
    </button>
  {:else}
    <div
      class="flex h-10 min-w-0 flex-1 items-center truncate rounded-md border border-border-input bg-background px-4 text-sm text-muted-foreground cursor-pointer select-none hover:bg-muted/50 transition-colors"
      onclick={startEdit}
      title="点击编辑"
    >
      {#if mode === 'password'}
        {visible ? value : mask(value)}
      {:else}
        {value || placeholder}
      {/if}
    </div>
    {#if mode === 'password'}
      <button
        class="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-border-input bg-background text-muted-foreground transition-colors hover:bg-muted"
        onclick={() => visible = !visible}
        title={visible ? '隐藏' : '显示'}
      >
        {#if visible}
          <Eye class="h-4 w-4" />
        {:else}
          <EyeOff class="h-4 w-4" />
        {/if}
      </button>
    {/if}
  {/if}
</div>
```

- [ ] **Step 2: 更新 models 页面导入**

修改 `src/routes/models/+page.svelte` 第 5 行：

```typescript
import PasswordInput from '$lib/components/ui/password-input/index.svelte';
```

改为：

```typescript
import EditableField from '$lib/components/ui/editable-field/index.svelte';
```

- [ ] **Step 3: 更新 API Key 输入框**

修改 `src/routes/models/+page.svelte` 第 547-551 行：

```svelte
<PasswordInput
  value={provider.api_key || ''}
  placeholder="sk-..."
  onChange={(val: string) => handleApiKeyChange(provider.id, val)}
/>
```

改为：

```svelte
<EditableField
  value={provider.api_key || ''}
  placeholder="sk-..."
  mode="password"
  onChange={(val: string) => handleApiKeyChange(provider.id, val)}
/>
```

- [ ] **Step 4: 更新 Endpoint 输入框**

修改 `src/routes/models/+page.svelte` 第 566-572 行：

```svelte
<input
  class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
  type="text"
  value={provider.endpoint}
  onchange={(e) => handleProviderFieldChange(provider.id, 'endpoint', (e.target as HTMLInputElement).value)}
/>
```

改为：

```svelte
<EditableField
  value={provider.endpoint}
  placeholder="https://api.example.com/v1"
  mode="text"
  onChange={(val: string) => handleProviderFieldChange(provider.id, 'endpoint', val)}
/>
```

- [ ] **Step 5: 测试组件功能**

运行应用并测试：
- API Key 输入框点击进入编辑模式
- Endpoint 输入框点击进入编辑模式
- 确认/取消按钮正常工作
- Esc/Enter 键盘快捷键正常工作
- 眼睛图标切换显示/隐藏

- [ ] **Step 6: 提交更改**

```bash
git add src/lib/components/ui/editable-field/index.svelte src/routes/models/+page.svelte
git commit -m "feat: add EditableField component and update models page"
```

---

### Task 2: 删除旧的 PasswordInput 组件

**Files:**
- Delete: `src/lib/components/ui/password-input/index.svelte`

- [ ] **Step 1: 确认没有其他引用**

运行以下命令确认 PasswordInput 没有其他引用：

```bash
cd /Users/jia.xia/development/talkshow && grep -r "PasswordInput" src/
```

预期：只有 models 页面的旧导入（已修改）

- [ ] **Step 2: 删除旧组件目录**

```bash
rm -rf src/lib/components/ui/password-input
```

- [ ] **Step 3: 提交删除**

```bash
git add src/lib/components/ui/password-input
git commit -m "chore: remove old PasswordInput component"
```

---

### Task 3: 修复布局适配问题

**Files:**
- Modify: `src/routes/models/+page.svelte`

- [ ] **Step 1: 检查当前布局问题**

查看 models 页面第 474 行的卡片容器，确认是否有 `overflow-hidden`

- [ ] **Step 2: 添加 overflow-hidden 到卡片**

修改 `src/routes/models/+page.svelte` 第 474 行：

```svelte
<div class="rounded-xl border border-border bg-background-alt p-5">
```

改为：

```svelte
<div class="rounded-xl border border-border bg-background-alt p-5 overflow-hidden">
```

- [ ] **Step 3: 测试布局修复**

运行应用并测试：
- 长 API Key 不超出卡片宽度
- 长 Endpoint 不超出卡片宽度
- 缩小窗口宽度时布局正常

- [ ] **Step 4: 提交布局修复**

```bash
git add src/routes/models/+page.svelte
git commit -m "fix: add overflow-hidden to provider cards"
```

---

### Task 4: 最终测试和验证

- [ ] **Step 1: 运行类型检查**

```bash
cd /Users/jia.xia/development/talkshow && npm run check
```

预期：无类型错误

- [ ] **Step 2: 完整功能测试**

测试所有场景：
1. API Key 输入框
   - 点击进入编辑模式
   - 编辑后点击 ✓ 保存
   - 编辑后点击 ✕ 取消
   - 按 Esc 键取消
   - 按 Enter 键确认
   - 眼睛图标切换显示/隐藏

2. Endpoint 输入框
   - 点击进入编辑模式
   - 编辑后点击 ✓ 保存
   - 编辑后点击 ✕ 取消
   - 按 Esc 键取消
   - 按 Enter 键确认

3. 布局测试
   - 长 API Key 不超出卡片宽度
   - 长 Endpoint 不超出卡片宽度
   - 不同窗口宽度下布局正常

- [ ] **Step 3: 提交最终版本**

```bash
git add .
git commit -m "feat: improve API Key and Endpoint input interaction"
```

---

## 自我审查

**1. 规范覆盖：** 所有设计规范都已覆盖
- ✅ 组件重命名 PasswordInput → EditableField
- ✅ 支持 password 和 text 两种模式
- ✅ 点击进入编辑模式
- ✅ 确认/取消按钮
- ✅ Esc/Enter 键盘快捷键
- ✅ 布局适配修复

**2. 占位符扫描：** 无占位符

**3. 类型一致性：** 类型定义一致
