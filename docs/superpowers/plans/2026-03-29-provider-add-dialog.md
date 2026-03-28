# Provider 添加弹窗实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Models 页面的 Providers 网格中添加虚位卡片，点击后弹出 Dialog 弹窗，实现添加新 Provider。

**Architecture:** 基于 bits-ui Dialog 组件封装通用 Dialog UI 组件，在 models 页面中新增虚位卡片和添加 Provider 弹窗逻辑。表单包含 Name、Type（下拉选择）、ID（自动生成）、Endpoint 四个字段。

**Tech Stack:** Svelte 5, bits-ui Dialog, Tailwind CSS v4, Lucide icons

---

### Task 1: 创建 Dialog 通用组件

**Files:**
- Create: `src/lib/components/ui/dialog/index.svelte`

- [ ] **Step 1: 创建 Dialog 组件**

基于 bits-ui `Dialog` 封装，遵循现有组件（select、tag-input）的风格。使用 Svelte 5 的 `$props` 和 snippet。

```svelte
<script lang="ts">
  import { Dialog } from "bits-ui";
  import { X } from "lucide-svelte";

  interface Props {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    title: string;
    description?: string;
    children: import('svelte').Snippet;
    footer?: import('svelte').Snippet;
  }

  let { open, onOpenChange, title, description, children, footer }: Props = $props();
</script>

<Dialog.Root bind:open {onOpenChange}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-50 bg-black/50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
    <Dialog.Content class="fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 rounded-lg border border-border bg-background-alt p-5 shadow-popover data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95">
      <div class="flex justify-between items-center mb-3">
        <div>
          <Dialog.Title class="text-sm font-semibold text-foreground">{title}</Dialog.Title>
          {#if description}
            <Dialog.Description class="text-[11px] text-foreground-alt mt-0.5">{description}</Dialog.Description>
          {/if}
        </div>
        <Dialog.Close class="rounded-md p-1 text-muted-foreground hover:text-foreground transition-colors">
          <X class="h-4 w-4" />
        </Dialog.Close>
      </div>

      <div class="space-y-3">
        {@render children()}
      </div>

      {#if footer}
        <div class="flex justify-end gap-2 mt-4 pt-3 border-t border-border">
          {@render footer()}
        </div>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
```

- [ ] **Step 2: 运行类型检查**

Run: `npm run check`
Expected: 无新增错误

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/ui/dialog/index.svelte
git commit -m "feat: add Dialog UI component based on bits-ui"
```

---

### Task 2: 添加虚位卡片和弹窗逻辑

**Files:**
- Modify: `src/routes/models/+page.svelte`

- [ ] **Step 1: 添加 import 和状态**

在 `<script>` 顶部添加 Dialog 组件和 Plus 图标的导入，以及弹窗状态：

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { config } from '$lib/stores/config';
  import GroupedSelect from '$lib/components/ui/select/index.svelte';
  import TagInput from '$lib/components/ui/tag-input/index.svelte';
  import PasswordInput from '$lib/components/ui/password-input/index.svelte';
  import Dialog from '$lib/components/ui/dialog/index.svelte';
  import { Plus } from 'lucide-svelte';
  import type { ProviderConfig, AppConfig } from '$lib/stores/config';

  let showAddDialog = $state(false);
  let newProvider = $state({
    name: '',
    type: '',
    id: '',
    endpoint: ''
  });
  let formErrors = $state<Record<string, string>>({});

  const PROVIDER_TYPES = [
    { value: 'openai-compatible', label: 'OpenAI Compatible' },
    { value: 'stubbed', label: 'Stubbed' },
    { value: 'anthropic-compatible', label: 'Anthropic Compatible' }
  ];
```

- [ ] **Step 2: 添加表单处理函数**

在 `needsApiKey` 函数之后添加：

```typescript
  function generateSlug(name: string): string {
    return name
      .toLowerCase()
      .trim()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');
  }

  function handleNameInput(value: string) {
    newProvider.name = value;
    if (!formErrors.id) {
      newProvider.id = generateSlug(value);
    }
  }

  function handleTypeChange(value: string) {
    newProvider.type = value;
    if (formErrors.type) {
      formErrors = { ...formErrors, type: '' };
    }
  }

  function validateForm(): boolean {
    const errors: Record<string, string> = {};
    if (!newProvider.name.trim()) errors.name = '请输入名称';
    if (!newProvider.type) errors.type = '请选择类型';
    if (!newProvider.id.trim()) errors.id = '请输入 ID';
    if (!newProvider.endpoint.trim()) errors.endpoint = '请输入端点';

    const idRegex = /^[a-z0-9-]+$/;
    if (newProvider.id && !idRegex.test(newProvider.id)) {
      errors.id = 'ID 仅允许小写字母、数字和连字符';
    }
    if (
      newProvider.id &&
      ($config.ai.providers || []).some((p: ProviderConfig) => p.id === newProvider.id)
    ) {
      errors.id = '该 ID 已存在';
    }

    formErrors = errors;
    return Object.keys(errors).length === 0;
  }

  function handleAddProvider() {
    if (!validateForm()) return;

    const provider: ProviderConfig = {
      id: newProvider.id.trim(),
      type: newProvider.type,
      name: newProvider.name.trim(),
      endpoint: newProvider.endpoint.trim(),
      models: []
    };

    const newProviders = [...($config.ai.providers || []), provider];
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);

    newProvider = { name: '', type: '', id: '', endpoint: '' };
    formErrors = {};
    showAddDialog = false;
  }

  function handleDialogOpenChange(open: boolean) {
    if (!open) {
      newProvider = { name: '', type: '', id: '', endpoint: '' };
      formErrors = {};
    }
    showAddDialog = open;
  }
```

- [ ] **Step 3: 在 Providers 网格中添加虚位卡片**

在 `{#each}` 循环结束后、`</div>` 闭合前，添加虚位卡片和 Dialog：

```svelte
      <!-- 虚位卡片 -->
      <button
        class="rounded-lg border-2 border-dashed border-border bg-background-alt/50 hover:bg-background-alt transition-colors flex flex-col items-center justify-center gap-1.5 cursor-pointer p-3.5 min-h-[140px]"
        onclick={() => (showAddDialog = true)}
      >
        <Plus class="h-5 w-5 text-muted-foreground" />
        <span class="text-[11px] text-muted-foreground">添加 Provider</span>
      </button>
    </div>
  </section>

  <!-- 添加 Provider 弹窗 -->
  <Dialog
    open={showAddDialog}
    onOpenChange={handleDialogOpenChange}
    title="添加 Provider"
    description="配置新的 AI 服务提供商"
  >
    {#snippet children()}
      <div>
        <label class="block text-[11px] text-foreground-alt mb-1">Name</label>
        <input
          class="flex h-8 w-full rounded-md border border-border-input bg-background px-3 py-1 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="如：阿里云"
          value={newProvider.name}
          oninput={(e) => handleNameInput((e.target as HTMLInputElement).value)}
        />
        {#if formErrors.name}
          <p class="text-[10px] text-destructive mt-0.5">{formErrors.name}</p>
        {/if}
      </div>

      <div>
        <label class="block text-[11px] text-foreground-alt mb-1">Type</label>
        <GroupedSelect
          value={newProvider.type}
          groups={[{ label: '', items: PROVIDER_TYPES }]}
          placeholder="选择类型"
          onChange={handleTypeChange}
        />
        {#if formErrors.type}
          <p class="text-[10px] text-destructive mt-0.5">{formErrors.type}</p>
        {/if}
      </div>

      <div>
        <label class="block text-[11px] text-foreground-alt mb-1">ID</label>
        <input
          class="flex h-8 w-full rounded-md border border-border-input bg-background px-3 py-1 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="如：ali-yun"
          value={newProvider.id}
          oninput={(e) => { newProvider.id = (e.target as HTMLInputElement).value; formErrors = { ...formErrors, id: '' }; }}
        />
        {#if formErrors.id}
          <p class="text-[10px] text-destructive mt-0.5">{formErrors.id}</p>
        {/if}
      </div>

      <div>
        <label class="block text-[11px] text-foreground-alt mb-1">Endpoint</label>
        <input
          class="flex h-8 w-full rounded-md border border-border-input bg-background px-3 py-1 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="https://api.example.com/v1"
          value={newProvider.endpoint}
          oninput={(e) => { newProvider.endpoint = (e.target as HTMLInputElement).value; formErrors = { ...formErrors, endpoint: '' }; }}
        />
        {#if formErrors.endpoint}
          <p class="text-[10px] text-destructive mt-0.5">{formErrors.endpoint}</p>
        {/if}
      </div>
    {/snippet}

    {#snippet footer()}
      <button
        class="rounded-md border border-border px-3 py-1.5 text-xs text-foreground hover:bg-muted transition-colors"
        onclick={() => handleDialogOpenChange(false)}
      >
        取消
      </button>
      <button
        class="rounded-md bg-foreground px-3 py-1.5 text-xs text-background hover:bg-foreground/90 transition-colors"
        onclick={handleAddProvider}
      >
        添加
      </button>
    {/snippet}
  </Dialog>
</div>
```

注意：需要删除原有的 `</div>` 闭合（第178行和第179行），替换为上面的虚位卡片和弹窗。具体来说，将第178-179行的：

```svelte
    </div>
  </section>
</div>
```

替换为上面的虚位卡片 + Dialog 代码块。

- [ ] **Step 4: 运行类型检查**

Run: `npm run check`
Expected: 无新增错误（原有的1个预存错误不变）

- [ ] **Step 5: Commit**

```bash
git add src/routes/models/+page.svelte
git commit -m "feat: add provider creation dialog with placeholder card"
```
