# 日志选择复制功能实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在日志页面增加条目选择功能，支持部分复制，统一名称为"复制"

**Architecture:** 在现有日志页面组件中添加选择模式状态管理，通过复选框和 Shift+点击实现条目选择，修改复制逻辑支持选中复制

**Tech Stack:** Svelte 5 (runes), Tailwind CSS, Tauri

---

### Task 1: 添加选择模式状态管理

**Files:**
- Modify: `src/routes/logs/+page.svelte`

- [ ] **Step 1: 添加选择模式状态变量**

在现有状态变量后（第 38 行 `let copied = $state(false);` 之后）添加：

```ts
let selectMode = $state(false);
let selectedIds = $state<Set<number>>(new Set());
let lastClickedIndex = $state<number>(-1);
```

- [ ] **Step 2: 添加选择模式控制函数**

在 `copyAll` 函数后添加：

```ts
function toggleSelectMode() {
  selectMode = !selectMode;
  if (!selectMode) {
    selectedIds.clear();
    lastClickedIndex = -1;
  }
}

function toggleEntrySelection(index: number, event: MouseEvent) {
  if (event.shiftKey && lastClickedIndex >= 0) {
    const start = Math.min(lastClickedIndex, index);
    const end = Math.max(lastClickedIndex, index);
    for (let i = start; i <= end; i++) {
      selectedIds.add(i);
    }
  } else {
    if (selectedIds.has(index)) {
      selectedIds.delete(index);
    } else {
      selectedIds.add(index);
    }
    lastClickedIndex = index;
  }
}

async function copySelected() {
  const indices = Array.from(selectedIds).sort((a, b) => a - b);
  const text = indices.map((i) => formatEntryForCopy(filteredEntries[i])).join('\n');
  await navigator.clipboard.writeText(text);
  copied = true;
  selectMode = false;
  selectedIds.clear();
  lastClickedIndex = -1;
  setTimeout(() => { copied = false; }, 2000);
}
```

- [ ] **Step 3: 提交**

```bash
git add src/routes/logs/+page.svelte
git commit -m "feat: add selection mode state management for log entries"
```

---

### Task 2: 修改 UI - 按钮和复选框

**Files:**
- Modify: `src/routes/logs/+page.svelte`

- [ ] **Step 1: 修改顶部按钮区域**

将第 163-169 行的按钮替换为：

```svelte
{#if currentSession && activeTab === 'current'}
  <span class="ml-auto text-caption text-muted-foreground">
    {filteredEntries.length} 条日志
    {#if selectMode && selectedIds.size > 0}
      · 已选 {selectedIds.size} 条
    {/if}
  </span>
  {#if selectMode}
    <button
      class="px-3 py-1 rounded text-caption border border-btn-secondary-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to text-accent-foreground hover:opacity-90 transition-colors shadow-btn-secondary"
      onclick={copySelected}
      disabled={selectedIds.size === 0 || copied}
    >
      {copied ? '已复制' : `复制选中 (${selectedIds.size})`}
    </button>
    <button
      class="px-3 py-1 rounded text-caption border border-border text-muted-foreground hover:text-foreground transition-colors"
      onclick={toggleSelectMode}
    >
      取消
    </button>
  {:else}
    <button
      class="px-3 py-1 rounded text-caption border border-border text-muted-foreground hover:text-foreground transition-colors"
      onclick={toggleSelectMode}
    >
      选择
    </button>
    <button
      class="px-3 py-1 rounded text-caption border border-btn-secondary-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to text-accent-foreground hover:opacity-90 transition-colors shadow-btn-secondary"
      onclick={copyAll}
      disabled={copied}
    >
      {copied ? '已复制' : '复制'}
    </button>
  {/if}
{/if}
```

- [ ] **Step 2: 添加复选框到日志条目**

将第 207-219 行的日志条目渲染修改为：

```svelte
{#each filteredEntries as entry, i}
  <div class="flex gap-3 px-5 py-2 border-b border-border last:border-b-0 hover:bg-muted/30 {selectMode && selectedIds.has(i) ? 'bg-muted/50' : ''}">
    {#if selectMode}
      <input
        type="checkbox"
        checked={selectedIds.has(i)}
        onchange={(e) => toggleEntrySelection(i, e)}
        class="mt-0.5 cursor-pointer"
      />
    {/if}
    <span class="text-muted-foreground whitespace-nowrap shrink-0">{formatTimestamp(entry.ts)}</span>
    <span class="{levelColor(entry.level)} shrink-0 w-4 text-center">
      {entry.level === 'error' ? '✕' : entry.level === 'warn' ? '!' : '·'}
    </span>
    <span class="{MODULE_COLORS[entry.module] || 'text-muted-foreground'} shrink-0 w-28">{entry.module}</span>
    <span class="{entry.level === 'error' ? 'text-red-400' : 'text-foreground'} truncate">{entry.msg}</span>
    <span class="text-muted-foreground text-[11px] ml-auto whitespace-nowrap shrink-0">
      {metaSummary(entry.meta as Record<string, unknown> | undefined)}
    </span>
  </div>
{/each}
```

- [ ] **Step 3: 修改 copyAll 函数中的文本**

将 `copyAll` 函数中的 `'已拷贝'` 改为 `'已复制'`

- [ ] **Step 4: 提交**

```bash
git add src/routes/logs/+page.svelte
git commit -m "feat: add selection UI with checkboxes and updated button labels"
```

---

### Task 3: 验证和测试

**Files:**
- No file changes

- [ ] **Step 1: 启动开发服务器验证**

```bash
npm run dev
```

验证以下功能：
1. 默认状态显示「选择」和「复制」按钮
2. 点击「选择」进入选择模式，复选框出现
3. 点击复选框可以选中/取消单条日志
4. Shift+点击可以范围选择
5. 顶部显示选中数量
6. 点击「复制选中 (N)」复制选中日志
7. 复制后自动退出选择模式
8. 点击「取消」退出选择模式

- [ ] **Step 2: 运行类型检查**

```bash
npm run check
```

- [ ] **Step 3: 提交最终版本**

```bash
git add src/routes/logs/+page.svelte
git commit -m "feat: complete log selection copy feature"
```
