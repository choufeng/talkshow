# 按钮渐变立体风格升级 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将所有按钮从平铺单色升级为绿色渐变 + 微阴影的立体风格，统一使用项目的绿色系色板。

**Architecture:** 在 `app.css` 中新增渐变/阴影 CSS 变量（Light + Dark 双模式），映射到 Tailwind v4 的 `@theme inline`，然后逐页面更新按钮的 Tailwind class。

**Tech Stack:** Svelte 5 + Tailwind CSS v4 + oklch 色彩空间

---

## 文件结构

| 操作 | 文件 | 职责 |
|------|------|------|
| 修改 | `src/app.css` | 新增按钮渐变/阴影 CSS 变量 + Tailwind 主题映射 |
| 修改 | `src/routes/+layout.svelte` | 侧边栏导航按钮渐变立体化 |
| 修改 | `src/routes/settings/+page.svelte` | 主题切换按钮渐变立体化 |
| 修改 | `src/routes/skills/+page.svelte` | Toggle 开关、主要/次要/危险按钮、图标按钮 |
| 修改 | `src/routes/models/+page.svelte` | Toggle 开关、主要/次要/危险按钮、图标按钮 |
| 修改 | `src/routes/logs/+page.svelte` | Tab 切换、模块过滤按钮、拷贝按钮、返回按钮；修复 `bg-primary` |
| 修改 | `src/routes/recording/+page.svelte` | 取消按钮（scoped style 中） |

---

### Task 1: 新增 CSS 变量和 Tailwind 主题映射

**Files:**
- Modify: `src/app.css`

- [ ] **Step 1: 在 `:root` (Light 模式) 中新增按钮渐变/阴影变量**

在 `src/app.css` 的 `:root { ... }` 块中，`--key-text` 之后添加：

```css
  --btn-primary-from: oklch(48% 0.1 160);
  --btn-primary-to: oklch(38% 0.1 160);
  --btn-primary-shadow: 0 2px 4px oklch(38% 0.1 160 / 0.3), inset 0 1px 0 oklch(100% 0 0 / 0.15);
  --btn-secondary-from: oklch(100% 0 0);
  --btn-secondary-to: oklch(97% 0.003 160);
  --btn-secondary-border: oklch(48% 0.1 160 / 0.3);
  --btn-secondary-shadow: 0 1px 2px oklch(0% 0 0 / 0.06);
  --btn-destructive-from: oklch(62% 0.2 25);
  --btn-destructive-to: oklch(55% 0.2 25);
  --btn-destructive-shadow: 0 2px 4px oklch(55% 0.2 25 / 0.3), inset 0 1px 0 oklch(100% 0 0 / 0.15);
  --toggle-off-from: oklch(82% 0.005 160);
  --toggle-off-to: oklch(76% 0.005 160);
  --toggle-thumb-from: oklch(100% 0 0);
  --toggle-thumb-to: oklch(96% 0 0);
```

- [ ] **Step 2: 在 `.dark` (Dark 模式) 中新增对应变量**

在 `src/app.css` 的 `.dark { ... }` 块中，`--key-text` 之后添加：

```css
  --btn-primary-from: oklch(62% 0.1 160);
  --btn-primary-to: oklch(55% 0.1 160);
  --btn-primary-shadow: 0 2px 6px oklch(0% 0 0 / 0.4), 0 0 10px oklch(55% 0.1 160 / 0.2), inset 0 1px 0 oklch(100% 0 0 / 0.1);
  --btn-secondary-from: oklch(24% 0.006 160);
  --btn-secondary-to: oklch(18% 0.005 160);
  --btn-secondary-border: oklch(55% 0.1 160 / 0.35);
  --btn-secondary-shadow: 0 1px 3px oklch(0% 0 0 / 0.2);
  --btn-destructive-from: oklch(62% 0.2 25);
  --btn-destructive-to: oklch(55% 0.2 25);
  --btn-destructive-shadow: 0 2px 6px oklch(0% 0 0 / 0.4), 0 0 8px oklch(55% 0.2 25 / 0.2), inset 0 1px 0 oklch(100% 0 0 / 0.1);
  --toggle-off-from: oklch(30% 0.006 160);
  --toggle-off-to: oklch(24% 0.005 160);
  --toggle-thumb-from: oklch(45% 0 0);
  --toggle-thumb-to: oklch(38% 0 0);
```

- [ ] **Step 3: 在 `@theme inline` 中映射新变量**

在 `src/app.css` 的 `@theme inline { ... }` 块中，`--color-key-text` 之后添加：

```css
  --color-btn-primary-from: var(--btn-primary-from);
  --color-btn-primary-to: var(--btn-primary-to);
  --color-btn-secondary-from: var(--btn-secondary-from);
  --color-btn-secondary-to: var(--btn-secondary-to);
  --color-btn-secondary-border: var(--btn-secondary-border);
  --color-btn-destructive-from: var(--btn-destructive-from);
  --color-btn-destructive-to: var(--btn-destructive-to);
  --color-toggle-off-from: var(--toggle-off-from);
  --color-toggle-off-to: var(--toggle-off-to);
  --color-toggle-thumb-from: var(--toggle-thumb-from);
  --color-toggle-thumb-to: var(--toggle-thumb-to);

  --shadow-btn-primary: var(--btn-primary-shadow);
  --shadow-btn-secondary: var(--btn-secondary-shadow);
  --shadow-btn-destructive: var(--btn-destructive-shadow);
```

- [ ] **Step 4: 验证构建**

Run: `npm run build 2>&1 | head -30`
Expected: 构建成功，无错误。

- [ ] **Step 5: Commit**

```bash
git add src/app.css
git commit -m "feat: add gradient/shadow CSS variables for button upgrade"
```

---

### Task 2: 升级侧边栏导航按钮 (`+layout.svelte`)

**Files:**
- Modify: `src/routes/+layout.svelte`

- [ ] **Step 1: 更新导航按钮样式**

将 `src/routes/+layout.svelte` 中所有 5 个导航按钮的样式替换。每个按钮当前使用的是：

```
class="flex items-center gap-3 px-6 py-3 w-full text-[15px] text-foreground text-left transition-colors {activeMenu === 'home' ? 'bg-muted border-l-[3px] border-l-accent-foreground' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
```

替换激活态从 `'bg-muted border-l-[3px] border-l-accent-foreground'` 为：
```
'bg-gradient-to-r from-btn-primary-from/15 to-btn-primary-from/5 border-l-[3px] border-l-accent-foreground font-medium shadow-[0_1px_3px_oklch(38%_0.1_160/0.1)]'
```

未激活态保持不变：`'hover:bg-muted/50 border-l-[3px] border-l-transparent'`

需要将所有 5 个按钮（首页、模型、技能、设置、日志）都做此替换。

- [ ] **Step 2: 验证构建**

Run: `npm run build 2>&1 | head -30`
Expected: 构建成功。

- [ ] **Step 3: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "feat: upgrade sidebar nav buttons with gradient style"
```

---

### Task 3: 升级设置页面主题切换按钮 (`settings/+page.svelte`)

**Files:**
- Modify: `src/routes/settings/+page.svelte`

- [ ] **Step 1: 更新主题切换按钮**

将 `src/routes/settings/+page.svelte` 第 61 行的按钮 class 从：

```
class="flex-1 flex items-center justify-center gap-2 rounded-lg border px-4 py-2.5 text-sm transition-colors {$theme === opt.value ? 'border-accent-foreground bg-accent/50 text-accent-foreground font-medium' : 'border-border bg-background text-foreground hover:bg-muted/50'}"
```

替换为：

```
class="flex-1 flex items-center justify-center gap-2 rounded-lg border px-4 py-2.5 text-sm transition-colors {$theme === opt.value ? 'border-btn-primary-to bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white font-medium shadow-btn-primary' : 'border-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to text-foreground hover:bg-muted/50 shadow-btn-secondary'}"
```

- [ ] **Step 2: 验证构建**

Run: `npm run build 2>&1 | head -30`
Expected: 构建成功。

- [ ] **Step 3: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat: upgrade settings theme switcher buttons with gradient style"
```

---

### Task 4: 升级技能页面按钮 (`skills/+page.svelte`)

**Files:**
- Modify: `src/routes/skills/+page.svelte`

- [ ] **Step 1: 更新 Toggle 开关样式**

将第 131 行的 Toggle 按钮的 class 中：
- 激活态 `'bg-accent-foreground'` 替换为 `'bg-gradient-to-b from-btn-primary-from to-btn-primary-to shadow-btn-primary'`
- 未激活态 `'bg-border'` 替换为 `'bg-gradient-to-b from-toggle-off-from to-toggle-off-to shadow-btn-secondary'`

将第 136 行的滑块 span：
```
class="pointer-events-none inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {skill.enabled ? 'translate-x-4' : 'translate-x-0'}"
```
替换为：
```
class="pointer-events-none inline-block h-3.5 w-3.5 transform rounded-full bg-gradient-to-b from-toggle-thumb-from to-toggle-thumb-to shadow-sm ring-0 transition duration-200 ease-in-out {skill.enabled ? 'translate-x-4' : 'translate-x-0'}"
```

- [ ] **Step 2: 更新图标按钮样式**

将第 150 行编辑按钮的 class：
```
class="rounded p-1.5 text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
```
替换为：
```
class="rounded p-1.5 text-muted-foreground hover:text-foreground bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to shadow-btn-secondary transition-colors"
```

将第 158 行删除图标按钮的 class：
```
class="rounded p-1.5 text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
```
替换为：
```
class="rounded p-1.5 text-muted-foreground hover:text-destructive bg-gradient-to-b from-btn-destructive-from/10 to-btn-destructive-to/10 shadow-btn-secondary transition-colors"
```

- [ ] **Step 3: 更新编辑对话框中的"取消"按钮（第 217-220 行）**

将：
```
class="rounded-md border border-border px-4 py-2 text-sm text-foreground hover:bg-muted transition-colors"
```
替换为：
```
class="rounded-md border border-btn-secondary-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to px-4 py-2 text-sm text-accent-foreground hover:bg-muted transition-colors shadow-btn-secondary"
```

- [ ] **Step 4: 更新编辑对话框中的"保存"按钮（第 224-227 行）**

将：
```
class="rounded-md bg-foreground px-4 py-2 text-sm text-background hover:bg-foreground/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
```
替换为：
```
class="rounded-md bg-gradient-to-b from-btn-primary-from to-btn-primary-to px-4 py-2 text-sm text-white hover:opacity-90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed shadow-btn-primary"
```

- [ ] **Step 5: 更新删除确认对话框中的"取消"按钮（第 242-244 行）**

同 Step 3 的"取消"按钮样式替换。

- [ ] **Step 6: 更新删除确认对话框中的"删除"按钮（第 249-252 行）**

将：
```
class="rounded-md bg-destructive px-4 py-2 text-sm text-white hover:bg-destructive/90 transition-colors"
```
替换为：
```
class="rounded-md bg-gradient-to-b from-btn-destructive-from to-btn-destructive-to px-4 py-2 text-sm text-white hover:opacity-90 transition-colors shadow-btn-destructive"
```

- [ ] **Step 7: 验证构建**

Run: `npm run build 2>&1 | head -30`
Expected: 构建成功。

- [ ] **Step 8: Commit**

```bash
git add src/routes/skills/+page.svelte
git commit -m "feat: upgrade skills page buttons with gradient style"
```

---

### Task 5: 升级模型页面按钮 (`models/+page.svelte`)

**Files:**
- Modify: `src/routes/models/+page.svelte`

- [ ] **Step 1: 更新润色 Toggle 开关（第 429 行）**

激活态 `'bg-accent-foreground'` 替换为 `'bg-gradient-to-b from-btn-primary-from to-btn-primary-to shadow-btn-primary'`。
未激活态 `'bg-border'` 替换为 `'bg-gradient-to-b from-toggle-off-from to-toggle-off-to shadow-btn-secondary'`。

将第 434 行的滑块 span：
```
class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {$config.features.transcription.polish_enabled ? 'translate-x-5' : 'translate-x-0'}"
```
替换为：
```
class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-gradient-to-b from-toggle-thumb-from to-toggle-thumb-to shadow ring-0 transition duration-200 ease-in-out {$config.features.transcription.polish_enabled ? 'translate-x-5' : 'translate-x-0'}"
```

- [ ] **Step 2: 更新添加 Provider 对话框按钮**

"取消"按钮（第 694-696 行）：
```
class="rounded-md border border-border px-4 py-2 text-sm text-foreground hover:bg-muted transition-colors"
```
替换为：
```
class="rounded-md border border-btn-secondary-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to px-4 py-2 text-sm text-accent-foreground hover:bg-muted transition-colors shadow-btn-secondary"
```

"添加"按钮（第 701-703 行）：
```
class="rounded-md bg-foreground px-4 py-2 text-sm text-background hover:bg-foreground/90 transition-colors"
```
替换为：
```
class="rounded-md bg-gradient-to-b from-btn-primary-from to-btn-primary-to px-4 py-2 text-sm text-white hover:opacity-90 transition-colors shadow-btn-primary"
```

- [ ] **Step 3: 更新删除 Provider 对话框按钮**

"取消"按钮（第 718-720 行）：同 Step 2 的取消样式。
"删除"按钮（第 725-727 行）：
```
class="rounded-md bg-destructive px-4 py-2 text-sm text-white hover:bg-destructive/90 transition-colors"
```
替换为：
```
class="rounded-md bg-gradient-to-b from-btn-destructive-from to-btn-destructive-to px-4 py-2 text-sm text-white hover:opacity-90 transition-colors shadow-btn-destructive"
```

- [ ] **Step 4: 更新重置 Provider 对话框按钮**

"取消"按钮（第 743-745 行）：同 Step 2 的取消样式。
"重置"按钮（第 749-751 行）：
```
class="rounded-md bg-foreground px-4 py-2 text-sm text-background hover:bg-foreground/90 transition-colors"
```
替换为：
```
class="rounded-md bg-gradient-to-b from-btn-primary-from to-btn-primary-to px-4 py-2 text-sm text-white hover:opacity-90 transition-colors shadow-btn-primary"
```

- [ ] **Step 5: 更新删除模型对话框按钮**

"取消"按钮（第 767-769 行）：同 Step 2 的取消样式。
"删除"按钮（第 773-775 行）：同 Step 3 的删除样式。

- [ ] **Step 6: 更新添加模型对话框按钮**

"取消"按钮（第 820-822 行）：同 Step 2 的取消样式。
"添加"按钮（第 827-829 行）：同 Step 2 的添加样式。

- [ ] **Step 7: 验证构建**

Run: `npm run build 2>&1 | head -30`
Expected: 构建成功。

- [ ] **Step 8: Commit**

```bash
git add src/routes/models/+page.svelte
git commit -m "feat: upgrade models page buttons with gradient style"
```

---

### Task 6: 升级日志页面按钮 (`logs/+page.svelte`)

**Files:**
- Modify: `src/routes/logs/+page.svelte`

- [ ] **Step 1: 更新 Tab 切换按钮**

当前 Tab 样式在第 157 行：
```
class="px-4 py-1.5 rounded text-sm transition-colors {activeTab === 'current' ? 'bg-background text-foreground font-medium' : 'text-muted-foreground hover:text-foreground'}"
```
激活态替换为：
```
'bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white font-medium shadow-btn-primary'
```
未激活态保持不变。

同文件中第 163 行的 history Tab 也做相同替换（`activeTab === 'history'` 的判断）。

- [ ] **Step 2: 修复模块过滤按钮的 `bg-primary` 并升级样式**

第 172-173 行，将：
```
class="px-3 py-1.5 rounded text-xs transition-colors {selectedModule === mod ? 'bg-primary text-primary-foreground' : 'border border-border text-muted-foreground hover:text-foreground'}"
```
替换为：
```
class="px-3 py-1.5 rounded text-xs transition-colors {selectedModule === mod ? 'bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white font-medium shadow-btn-primary' : 'border border-border text-muted-foreground hover:text-foreground'}"
```

- [ ] **Step 3: 更新"拷贝全部"按钮**

第 185-188 行，将：
```
class="px-3 py-1 rounded text-xs border border-border text-muted-foreground hover:text-foreground transition-colors"
```
替换为：
```
class="px-3 py-1 rounded text-xs border border-btn-secondary-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to text-accent-foreground hover:opacity-90 transition-colors shadow-btn-secondary"
```

- [ ] **Step 4: 更新"返回历史列表"按钮**

第 247 行，将：
```
class="mt-3 text-sm text-muted-foreground hover:text-foreground transition-colors"
```
替换为：
```
class="mt-3 text-sm text-accent-foreground hover:opacity-80 transition-colors"
```

- [ ] **Step 5: 验证构建**

Run: `npm run build 2>&1 | head -30`
Expected: 构建成功。

- [ ] **Step 6: Commit**

```bash
git add src/routes/logs/+page.svelte
git commit -m "feat: upgrade logs page buttons with gradient style, fix bg-primary"
```

---

### Task 7: 升级录音页面取消按钮 (`recording/+page.svelte`)

**Files:**
- Modify: `src/routes/recording/+page.svelte`

- [ ] **Step 1: 更新取消按钮的 scoped style**

在 `<style>` 块中，将 `.btn` 的样式（第 250-263 行）从：
```css
  .btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border: none;
    border-radius: 50%;
    cursor: pointer;
    background: transparent;
    color: #64748b;
    transition: background 0.15s, color 0.15s;
    flex-shrink: 0;
  }
```
替换为：
```css
  .btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border: none;
    border-radius: 50%;
    cursor: pointer;
    background: linear-gradient(to bottom, rgba(255,255,255,0.08), rgba(255,255,255,0.03));
    color: #64748b;
    transition: background 0.15s, color 0.15s;
    flex-shrink: 0;
    box-shadow: 0 1px 3px rgba(0,0,0,0.2);
  }
```

将 `.btn:hover` 的样式从：
```css
  .btn:hover {
    background: rgba(255, 95, 87, 0.2);
    color: #ff5f57;
  }
```
替换为：
```css
  .btn:hover {
    background: linear-gradient(to bottom, rgba(255, 95, 87, 0.25), rgba(255, 95, 87, 0.15));
    color: #ff5f57;
    box-shadow: 0 1px 4px rgba(255, 95, 87, 0.2);
  }
```

- [ ] **Step 2: 验证构建**

Run: `npm run build 2>&1 | head -30`
Expected: 构建成功。

- [ ] **Step 3: Commit**

```bash
git add src/routes/recording/+page.svelte
git commit -m "feat: upgrade recording cancel button with gradient style"
```

---

### Task 8: 最终验证

- [ ] **Step 1: 完整构建验证**

Run: `npm run build`
Expected: 构建成功，无错误。

- [ ] **Step 2: 检查所有修改文件**

Run: `git diff --stat main...HEAD`（或当前分支与主分支对比）
Expected: 7 个文件被修改，所有修改均为按钮样式变更。

- [ ] **Step 3: Commit（如有 lint/fix 自动修改）**

仅在有自动修改时提交。
