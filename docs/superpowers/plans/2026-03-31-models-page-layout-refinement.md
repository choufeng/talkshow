# 模型页面布局优化实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 优化模型页面的视觉层级：用外层容器区分"AI 服务"和"Providers"，精简 SenseVoice 卡片的多余操作。

**Architecture:** 纯前端布局调整，不涉及后端、数据结构或新组件。修改 `src/routes/models/+page.svelte` 中的模板结构。

**Tech Stack:** Svelte + Tailwind CSS

---

### Task 1: AI 服务容器分组

**Files:**
- Modify: `src/routes/models/+page.svelte:423-465`

- [ ] **Step 1: 将"转写服务"section 包裹进"AI 服务"外层容器**

将第 423-465 行的 `<section class="mb-10">...</section>` 替换为：

```svelte
<section class="mb-10">
  <div class="rounded-xl border border-border bg-background-alt p-5">
    <div class="text-sm font-semibold text-foreground mb-4">AI 服务</div>
    <div class="mb-5">
      <div class="text-xs text-muted-foreground uppercase tracking-wider mb-3">转写服务</div>
      <div class="flex flex-col gap-5">
        <div>
          <label class="block text-sm text-foreground-alt mb-1.5">转写模型</label>
          <GroupedSelect
            value={getTranscriptionValue()}
            groups={buildTranscriptionGroups()}
            placeholder="选择模型"
            onChange={handleTranscriptionChange}
          />
        </div>

        <div class="flex items-center justify-between">
          <div>
            <div class="text-[15px] font-semibold text-foreground">启用润色</div>
            <div class="text-sm text-foreground-alt">转写后自动使用 LLM 润色文字</div>
          </div>
          <button
            class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-2 {$config.features.transcription.polish_enabled ? 'bg-gradient-to-b from-btn-primary-from to-btn-primary-to shadow-btn-primary' : 'bg-gradient-to-b from-toggle-off-from to-toggle-off-to shadow-btn-secondary'}"
            role="switch"
            aria-checked={$config.features.transcription.polish_enabled}
            onclick={() => handlePolishEnabled(!$config.features.transcription.polish_enabled)}
          >
            <span class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-gradient-to-b from-toggle-thumb-from to-toggle-thumb-to shadow ring-0 transition duration-200 ease-in-out {$config.features.transcription.polish_enabled ? 'translate-x-5' : 'translate-x-0'}"></span>
          </button>
        </div>

        {#if $config.features.transcription.polish_enabled}
        <div>
          <label class="block text-sm text-foreground-alt mb-1.5">润色模型</label>
          <GroupedSelect
            value={getPolishValue()}
            groups={buildPolishGroups()}
            placeholder="选择模型"
            onChange={handlePolishChange}
          />
        </div>
        {/if}
      </div>
    </div>
  </div>
</section>
```

关键变化：
- 新增外层 `<div class="rounded-xl border border-border bg-background-alt p-5">` 作为"AI 服务"容器
- 新增 `<div class="text-sm font-semibold text-foreground mb-4">AI 服务</div>` 作为容器标题
- "转写服务"子标题保留原有样式，包裹在 `<div class="mb-5">` 中（为未来扩展预留间距）
- 移除"转写服务"原有的 `rounded-xl border border-border bg-background-alt p-5` 卡片包裹

- [ ] **Step 2: 验证页面渲染**

Run: `npm run dev`
Expected: "转写服务"区域现在被包裹在"AI 服务"容器内，与 Providers 有明显的视觉层级区分。

---

### Task 2: SenseVoice 卡片操作精简

**Files:**
- Modify: `src/routes/models/+page.svelte:602-626`

- [ ] **Step 1: 对 sensevoice 类型跳过 Models 标签内的删除按钮**

将第 602-607 行的删除按钮：

```svelte
                    <button
                      class="opacity-60 hover:opacity-100 transition-opacity"
                      onclick={(e) => { e.stopPropagation(); handleRemoveModel(provider.id, model.name); }}
                    >
                      ✕
                    </button>
```

替换为：

```svelte
                    {#if provider.type !== 'sensevoice'}
                    <button
                      class="opacity-60 hover:opacity-100 transition-opacity"
                      onclick={(e) => { e.stopPropagation(); handleRemoveModel(provider.id, model.name); }}
                    >
                      ✕
                    </button>
                    {/if}
```

- [ ] **Step 2: 对 sensevoice 类型跳过"添加模型"和"测试全部"按钮**

将第 611-626 行的操作按钮区域：

```svelte
              <div class="flex items-center gap-1.5">
                <button
                  class="text-xs text-accent-foreground hover:underline"
                  onclick={() => openAddModelDialog(provider.id)}
                >
                  + 添加模型
                </button>
                <span class="text-border">|</span>
                <button
                  class="text-xs text-accent-foreground hover:underline inline-flex items-center gap-0.5"
                  onclick={() => testAllModels(provider)}
                  disabled={provider.models.length === 0 || [...testingModels].some(k => k.startsWith(provider.id + '::'))}
                >
                  ⟳ 测试全部
                </button>
              </div>
```

替换为：

```svelte
              {#if provider.type !== 'sensevoice'}
              <div class="flex items-center gap-1.5">
                <button
                  class="text-xs text-accent-foreground hover:underline"
                  onclick={() => openAddModelDialog(provider.id)}
                >
                  + 添加模型
                </button>
                <span class="text-border">|</span>
                <button
                  class="text-xs text-accent-foreground hover:underline inline-flex items-center gap-0.5"
                  onclick={() => testAllModels(provider)}
                  disabled={provider.models.length === 0 || [...testingModels].some(k => k.startsWith(provider.id + '::'))}
                >
                  ⟳ 测试全部
                </button>
              </div>
              {/if}
```

- [ ] **Step 3: 验证页面渲染**

Run: `npm run dev`
Expected: SenseVoice 卡片的 Models 区域只显示模型名称和验证状态，不显示 ✕ 删除按钮、"+ 添加模型"和"测试全部"。模型状态区的"删除模型"按钮仍然保留。
