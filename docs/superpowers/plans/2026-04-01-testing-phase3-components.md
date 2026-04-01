# Phase 3: 前端组件测试

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为核心 UI 组件和 hooks 编写交互行为测试，确保组件渲染、用户交互和状态管理正确。

**Architecture:** 使用 @testing-library/svelte 进行组件挂载和交互模拟。测试文件与源组件同目录放置。对于依赖 bits-ui Portal 的组件（如 select），使用 `vi.mock` 处理。

**Tech Stack:** Vitest, @testing-library/svelte, @testing-library/jest-dom, jsdom

**前置条件:** Phase 1 已完成（Vitest 和 testing-library 已安装）

---

### Task 1: 配置 @testing-library/jest-dom 全局 setup

**Files:**
- Modify: `vitest.config.ts`

- [ ] **Step 1: 创建 setup 文件**

创建 `src/test-setup.ts`：

```typescript
import '@testing-library/jest-dom/vitest';
```

- [ ] **Step 2: 更新 vitest.config.ts 引用 setup 文件**

```typescript
import { defineConfig } from 'vitest/config';
import { sveltekit } from '@sveltejs/kit/vite';

export default defineConfig({
  plugins: [sveltekit()],
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,js}'],
    setupFiles: ['./src/test-setup.ts'],
  },
});
```

- [ ] **Step 3: 验证配置**

Run: `npx vitest run`

Expected: 无错误，运行已有的 Phase 1 测试

- [ ] **Step 4: Commit**

```bash
git add src/test-setup.ts vitest.config.ts
git commit -m "chore: add jest-dom global setup for vitest"
```

---

### Task 2: Toggle 组件测试

**Files:**
- Create: `src/lib/components/ui/toggle/index.test.ts`
- Test: `src/lib/components/ui/toggle/index.svelte`

- [ ] **Step 1: 编写 Toggle 测试**

```typescript
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import Toggle from './index.svelte';

describe('Toggle', () => {
  it('renders as switch role', () => {
    render(Toggle, { props: { checked: false } });
    const button = screen.getByRole('switch');
    expect(button).toBeInTheDocument();
  });

  it('reflects checked state via aria-checked', () => {
    render(Toggle, { props: { checked: true } });
    const button = screen.getByRole('switch');
    expect(button).toHaveAttribute('aria-checked', 'true');
  });

  it('reflects unchecked state via aria-checked', () => {
    render(Toggle, { props: { checked: false } });
    const button = screen.getByRole('switch');
    expect(button).toHaveAttribute('aria-checked', 'false');
  });

  it('calls onCheckedChange when clicked', async () => {
    const onChange = vi.fn();
    render(Toggle, { props: { checked: false, onCheckedChange: onChange } });
    const button = screen.getByRole('switch');
    await fireEvent.click(button);
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it('calls onCheckedChange with false when toggling from checked', async () => {
    const onChange = vi.fn();
    render(Toggle, { props: { checked: true, onCheckedChange: onChange } });
    const button = screen.getByRole('switch');
    await fireEvent.click(button);
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it('does not call onCheckedChange when disabled', async () => {
    const onChange = vi.fn();
    render(Toggle, { props: { checked: false, onCheckedChange: onChange, disabled: true } });
    const button = screen.getByRole('switch');
    expect(button).toBeDisabled();
    await fireEvent.click(button);
    expect(onChange).not.toHaveBeenCalled();
  });

  it('applies aria-label when provided', () => {
    render(Toggle, { props: { checked: false, ariaLabel: 'Enable feature' } });
    const button = screen.getByRole('switch');
    expect(button).toHaveAttribute('aria-label', 'Enable feature');
  });

  it('applies size classes', () => {
    const { container } = render(Toggle, { props: { checked: false, size: 'sm' } });
    const button = container.querySelector('button');
    expect(button?.className).toContain('h-5');
  });
});
```

- [ ] **Step 2: 运行测试**

Run: `npx vitest run src/lib/components/ui/toggle/index.test.ts`

Expected: 全部通过

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/ui/toggle/index.test.ts
git commit -m "test: add Toggle component tests"
```

---

### Task 3: EditableField 组件测试

**Files:**
- Create: `src/lib/components/ui/editable-field/index.test.ts`
- Test: `src/lib/components/ui/editable-field/index.svelte`

- [ ] **Step 1: 编写 EditableField 测试**

```typescript
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import EditableField from './index.svelte';

describe('EditableField', () => {
  it('displays masked value in password mode', () => {
    render(EditableField, { props: { value: 'sk-1234567890', mode: 'password', onChange: vi.fn() } });
    const display = screen.getByRole('button');
    expect(display.textContent).toContain('sk-');
    expect(display.textContent).toContain('•');
  });

  it('displays plain value in text mode', () => {
    render(EditableField, { props: { value: 'hello world', mode: 'text', onChange: vi.fn() } });
    const display = screen.getByRole('button');
    expect(display.textContent).toContain('hello world');
  });

  it('shows placeholder when value is empty', () => {
    render(EditableField, { props: { value: '', mode: 'text', onChange: vi.fn(), placeholder: 'Enter value' } });
    const display = screen.getByRole('button');
    expect(display.textContent).toContain('Enter value');
  });

  it('enters edit mode on click', async () => {
    render(EditableField, { props: { value: 'test', mode: 'text', onChange: vi.fn() } });
    const display = screen.getByRole('button');
    await fireEvent.click(display);
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('confirms edit on Enter key', async () => {
    const onChange = vi.fn();
    render(EditableField, { props: { value: 'old', mode: 'text', onChange } });
    const display = screen.getByRole('button');
    await fireEvent.click(display);

    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value: 'new value' } });
    await fireEvent.keyDown(input, { key: 'Enter' });

    expect(onChange).toHaveBeenCalledWith('new value');
  });

  it('cancels edit on Escape key', async () => {
    const onChange = vi.fn();
    render(EditableField, { props: { value: 'original', mode: 'text', onChange } });
    const display = screen.getByRole('button');
    await fireEvent.click(display);

    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value: 'changed' } });
    await fireEvent.keyDown(input, { key: 'Escape' });

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
  });

  it('confirms edit on confirm button click', async () => {
    const onChange = vi.fn();
    render(EditableField, { props: { value: 'old', mode: 'text', onChange } });
    await fireEvent.click(screen.getByRole('button'));

    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value: 'confirmed' } });

    const buttons = screen.getAllByRole('button');
    const confirmBtn = buttons.find(b => b.getAttribute('title') === '确认');
    expect(confirmBtn).toBeTruthy();
    await fireEvent.click(confirmBtn!);

    expect(onChange).toHaveBeenCalledWith('confirmed');
  });

  it('cancels edit on cancel button click', async () => {
    const onChange = vi.fn();
    render(EditableField, { props: { value: 'original', mode: 'text', onChange } });
    await fireEvent.click(screen.getByRole('button'));

    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value: 'changed' } });

    const buttons = screen.getAllByRole('button');
    const cancelBtn = buttons.find(b => b.getAttribute('title') === '取消');
    expect(cancelBtn).toBeTruthy();
    await fireEvent.click(cancelBtn!);

    expect(onChange).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: 运行测试**

Run: `npx vitest run src/lib/components/ui/editable-field/index.test.ts`

Expected: 全部通过

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/ui/editable-field/index.test.ts
git commit -m "test: add EditableField component tests"
```

---

### Task 4: TagInput 组件测试

**Files:**
- Create: `src/lib/components/ui/tag-input/index.test.ts`
- Test: `src/lib/components/ui/tag-input/index.svelte`

- [ ] **Step 1: 编写 TagInput 测试**

```typescript
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import TagInput from './index.svelte';

describe('TagInput', () => {
  it('renders existing tags', () => {
    render(TagInput, { props: { tags: ['tag1', 'tag2'], onAdd: vi.fn(), onRemove: vi.fn() } });
    expect(screen.getByText('tag1')).toBeInTheDocument();
    expect(screen.getByText('tag2')).toBeInTheDocument();
  });

  it('calls onRemove when tag remove button clicked', async () => {
    const onRemove = vi.fn();
    render(TagInput, { props: { tags: ['tag1'], onAdd: vi.fn(), onRemove } });
    const removeBtn = screen.getByText('✕').closest('button')!;
    await fireEvent.click(removeBtn);
    expect(onRemove).toHaveBeenCalledWith('tag1');
  });

  it('shows input on add button click', async () => {
    render(TagInput, { props: { tags: [], onAdd: vi.fn(), onRemove: vi.fn() } });
    const addBtn = screen.getByText('+ 添加模型');
    await fireEvent.click(addBtn);
    expect(screen.getByPlaceholderText('添加...')).toBeInTheDocument();
  });

  it('calls onAdd when Enter pressed in input', async () => {
    const onAdd = vi.fn();
    render(TagInput, { props: { tags: [], onAdd, onRemove: vi.fn() } });
    await fireEvent.click(screen.getByText('+ 添加模型'));

    const input = screen.getByPlaceholderText('添加...');
    await fireEvent.input(input, { target: { value: 'new-tag' } });
    await fireEvent.keyDown(input, { key: 'Enter' });

    expect(onAdd).toHaveBeenCalledWith('new-tag');
  });

  it('does not add duplicate tag', async () => {
    const onAdd = vi.fn();
    render(TagInput, { props: { tags: ['existing'], onAdd, onRemove: vi.fn() } });
    await fireEvent.click(screen.getByText('+ 添加模型'));

    const input = screen.getByPlaceholderText('添加...');
    await fireEvent.input(input, { target: { value: 'existing' } });
    await fireEvent.keyDown(input, { key: 'Enter' });

    expect(onAdd).not.toHaveBeenCalled();
  });

  it('does not add empty tag', async () => {
    const onAdd = vi.fn();
    render(TagInput, { props: { tags: [], onAdd, onRemove: vi.fn() } });
    await fireEvent.click(screen.getByText('+ 添加模型'));

    const input = screen.getByPlaceholderText('添加...');
    await fireEvent.input(input, { target: { value: '   ' } });
    await fireEvent.keyDown(input, { key: 'Enter' });

    expect(onAdd).not.toHaveBeenCalled();
  });

  it('hides input on Escape', async () => {
    render(TagInput, { props: { tags: [], onAdd: vi.fn(), onRemove: vi.fn() } });
    await fireEvent.click(screen.getByText('+ 添加模型'));

    const input = screen.getByPlaceholderText('添加...');
    await fireEvent.keyDown(input, { key: 'Escape' });

    expect(screen.queryByPlaceholderText('添加...')).not.toBeInTheDocument();
  });
});
```

- [ ] **Step 2: 运行测试**

Run: `npx vitest run src/lib/components/ui/tag-input/index.test.ts`

Expected: 全部通过

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/ui/tag-input/index.test.ts
git commit -m "test: add TagInput component tests"
```

---

### Task 5: use-dialog-state hook 测试

**Files:**
- Create: `src/lib/hooks/use-dialog-state.test.ts`
- Test: `src/lib/hooks/use-dialog-state.svelte.ts`

- [ ] **Step 1: 编写 use-dialog-state 测试**

```typescript
import { describe, it, expect, vi } from 'vitest';
import { createDialogState } from './use-dialog-state.svelte';

describe('createDialogState', () => {
  it('defaults to closed', () => {
    const state = createDialogState();
    expect(state.isOpen).toBe(false);
  });

  it('initializes with initialOpen=true', () => {
    const state = createDialogState({ initialOpen: true });
    expect(state.isOpen).toBe(true);
  });

  it('opens when open() called', () => {
    const state = createDialogState();
    state.open();
    expect(state.isOpen).toBe(true);
  });

  it('closes when close() called', () => {
    const state = createDialogState({ initialOpen: true });
    state.close();
    expect(state.isOpen).toBe(false);
  });

  it('onOpenChange(true) opens', () => {
    const state = createDialogState();
    state.onOpenChange(true);
    expect(state.isOpen).toBe(true);
  });

  it('onOpenChange(false) closes', () => {
    const state = createDialogState({ initialOpen: true });
    state.onOpenChange(false);
    expect(state.isOpen).toBe(false);
  });

  it('calls onReset when closing', () => {
    const onReset = vi.fn();
    const state = createDialogState({ initialOpen: true, onReset });
    state.close();
    expect(onReset).toHaveBeenCalledTimes(1);
  });

  it('calls setReset function when closing', () => {
    const resetFn = vi.fn();
    const state = createDialogState({ initialOpen: true });
    state.setReset(resetFn);
    state.close();
    expect(resetFn).toHaveBeenCalledTimes(1);
  });

  it('replaces reset function via setReset', () => {
    const firstReset = vi.fn();
    const secondReset = vi.fn();
    const state = createDialogState({ initialOpen: true, onReset: firstReset });
    state.setReset(secondReset);
    state.close();
    expect(firstReset).not.toHaveBeenCalled();
    expect(secondReset).toHaveBeenCalledTimes(1);
  });

  it('does not call reset on open', () => {
    const onReset = vi.fn();
    const state = createDialogState({ onReset });
    state.open();
    expect(onReset).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: 运行测试**

Run: `npx vitest run src/lib/hooks/use-dialog-state.test.ts`

Expected: 全部通过

- [ ] **Step 3: Commit**

```bash
git add src/lib/hooks/use-dialog-state.test.ts
git commit -m "test: add use-dialog-state hook tests"
```

---

### Task 6: Select 组件测试（基础渲染）

**Files:**
- Create: `src/lib/components/ui/select/index.test.ts`
- Test: `src/lib/components/ui/select/index.svelte`

注意：Select 组件使用 bits-ui Portal，在 jsdom 中可能需要特殊处理。此测试聚焦于 Trigger 部分的基础渲染。

- [ ] **Step 1: 编写 Select 基础测试**

```typescript
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Select from './index.svelte';

describe('Select', () => {
  const groups = [
    {
      label: '内置',
      items: [
        { value: 'vertex', label: 'Vertex AI' },
        { value: 'dashscope', label: '阿里云' },
      ],
    },
    {
      label: '自定义',
      items: [
        { value: 'custom', label: 'Custom Provider' },
      ],
    },
  ];

  it('renders trigger with placeholder when no value matches', () => {
    render(Select, { props: { value: 'unknown', groups, placeholder: '请选择', onChange: () => {} } });
    expect(screen.getByText('请选择')).toBeInTheDocument();
  });

  it('renders trigger with selected value label', () => {
    render(Select, { props: { value: 'vertex', groups, onChange: () => {} } });
    expect(screen.getByText(/Vertex AI/)).toBeInTheDocument();
  });

  it('renders trigger with group prefix for selected item', () => {
    render(Select, { props: { value: 'vertex', groups, onChange: () => {} } });
    const trigger = screen.getByText(/内置.*Vertex AI/);
    expect(trigger).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: 运行测试**

Run: `npx vitest run src/lib/components/ui/select/index.test.ts`

Expected: 基础渲染测试通过（Portal 相关测试可能需要后续补充）

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/ui/select/index.test.ts
git commit -m "test: add Select component basic render tests"
```

---

### Task 7: 验证全部测试通过

- [ ] **Step 1: 运行全部前端测试**

Run: `npx vitest run`

Expected: Phase 1 + Phase 3 全部测试通过

- [ ] **Step 2: 运行全部后端测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`

Expected: Phase 1 + Phase 2 全部测试通过

- [ ] **Step 3: 最终 Commit**

```bash
git add -A
git commit -m "test: phase 3 complete - frontend component and hook tests"
```
