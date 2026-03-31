# Skills 与润色配置优化 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 优化润色模型选择框的显示逻辑，移除 Skills 全局开关，并调整后端依赖。

**Architecture:** 前端条件渲染控制润色模型选择框可见性，移除 Skills 页面全局开关，后端移除 polish_enabled 检查并强制 skills.enabled=true。

**Tech Stack:** Svelte 5, Tauri 2, Rust

---

### Task 1: 前端 - 润色模型选择框条件渲染

**Files:**
- Modify: `src/routes/models/+page.svelte:438-446`

- [ ] **Step 1: 用 `{#if}` 包裹润色模型选择框**

将 `src/routes/models/+page.svelte` 第 438-446 行的润色模型选择框包裹在 `{#if}` 条件块中：

```svelte
{#if $config.features.transcription.polish_enabled}
  <div>
    <label class="block text-sm text-foreground-alt mb-1.5">润色模型</label>
    <GroupedSelect
      value={getPolishValue()}
      groups={buildTranscriptionGroups()}
      placeholder="选择模型"
      onChange={handlePolishChange}
    />
  </div>
{/if}
```

当前代码（第 438-446 行）：
```svelte
        <div>
          <label class="block text-sm text-foreground-alt mb-1.5">润色模型</label>
          <GroupedSelect
            value={getPolishValue()}
            groups={buildTranscriptionGroups()}
            placeholder="选择模型"
            onChange={handlePolishChange}
          />
        </div>
```

- [ ] **Step 2: 验证变更**

运行前端开发服务器，确认：
- 开启"启用润色"时，润色模型选择框显示
- 关闭"启用润色"时，润色模型选择框隐藏

- [ ] **Step 3: 提交**

```bash
git add src/routes/models/+page.svelte
git commit -m "feat: 润色模型选择框在启用润色关闭时隐藏"
```

---

### Task 2: 前端 - 移除 Skills 全局开关

**Files:**
- Modify: `src/routes/skills/+page.svelte:16-29,123-141`

- [ ] **Step 1: 移除 handleGlobalToggle 函数**

删除 `src/routes/skills/+page.svelte` 第 20-29 行的 `handleGlobalToggle` 函数：

```typescript
// 删除以下代码：
function handleGlobalToggle(enabled: boolean) {
  const newConfig: AppConfig = {
    ...$config,
    features: {
      ...$config.features,
      skills: { ...$config.features.skills, enabled }
    }
  };
  config.save(newConfig);
}
```

- [ ] **Step 2: 移除全局开关 UI 区块**

删除 `src/routes/skills/+page.svelte` 第 123-141 行的全局开关区块：

```svelte
<!-- 删除以下代码： -->
  <section class="mb-10">
    <div class="text-xs text-muted-foreground uppercase tracking-wider mb-3">全局</div>
    <div class="rounded-xl border border-border bg-background-alt p-5">
      <div class="flex items-center justify-between">
        <div>
          <div class="text-[15px] font-semibold text-foreground">Skills 功能</div>
          <div class="text-sm text-foreground-alt">启用后，转写文字将自动经过 Skill 处理管线</div>
        </div>
        <button
          class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-2 {$config.features.skills.enabled ? 'bg-accent-foreground' : 'bg-border'}"
          role="switch"
          aria-checked={$config.features.skills.enabled}
          onclick={() => handleGlobalToggle(!$config.features.skills.enabled)}
        >
          <span class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {$config.features.skills.enabled ? 'translate-x-5' : 'translate-x-0'}"></span>
        </button>
      </div>
    </div>
  </section>
```

- [ ] **Step 3: 修改 handleSkillToggle 确保 skills.enabled 始终为 true**

修改 `handleSkillToggle` 函数，在保存配置时强制 `enabled: true`：

```typescript
function handleSkillToggle(skillId: string, enabled: boolean) {
  const newSkills = $config.features.skills.skills.map((s: Skill) =>
    s.id === skillId ? { ...s, enabled } : s
  );
  const newConfig: AppConfig = {
    ...$config,
    features: {
      ...$config.features,
      skills: { ...$config.features.skills, skills: newSkills, enabled: true }
    }
  };
  config.save(newConfig);
}
```

- [ ] **Step 4: 修改 handleSave（添加/编辑 Skill 时）确保 enabled 为 true**

修改 `handleSave` 函数末尾的配置保存逻辑（第 66-92 行区域），在 `config.load()` 后确保 `enabled` 为 true。实际上由于 `config.load()` 会从后端加载配置，我们需要在后端强制。前端只需确保在直接保存时传递 `enabled: true`。

当前 `handleSave` 通过 `invoke('add_skill')` 和 `invoke('update_skill')` 调用后端，这些命令会读取完整配置并保存。我们需要确保后端的 `add_skill` 和 `update_skill` 也强制 `enabled: true`（见 Task 4）。

- [ ] **Step 5: 提交**

```bash
git add src/routes/skills/+page.svelte
git commit -m "feat: 移除 Skills 全局开关，保留单个 Skill 开关"
```

---

### Task 3: 后端 - 移除 polish_enabled 检查

**Files:**
- Modify: `src-tauri/src/skills.rs:100-103`

- [ ] **Step 1: 删除 polish_enabled 检查**

删除 `src-tauri/src/skills.rs` 第 100-103 行：

```rust
// 删除以下代码：
    if !transcription_config.polish_enabled {
        logger.info("skills", "润色功能未启用，跳过处理", None);
        return Ok(transcription.to_string());
    }
```

- [ ] **Step 2: 更新函数签名（可选优化）**

由于 `process_with_skills` 函数仍然需要 `transcription_config` 来获取 `polish_provider_id` 和 `polish_model`，保持现有签名不变。

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/skills.rs
git commit -m "refactor: 移除 Skills 管线对 polish_enabled 的依赖检查"
```

---

### Task 4: 后端 - 强制 skills.enabled = true

**Files:**
- Modify: `src-tauri/src/lib.rs:641-647,660-666,668-678`

- [ ] **Step 1: 修改 save_skills_config 强制 enabled=true**

修改 `src-tauri/src/lib.rs` 第 641-647 行的 `save_skills_config` 函数：

```rust
#[tauri::command]
fn save_skills_config(app_handle: tauri::AppHandle, mut skills_config: config::SkillsConfig) -> Result<(), String> {
    skills_config.enabled = true;
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);
    app_config.features.skills = skills_config;
    config::save_config(&app_data_dir, &app_config)
}
```

- [ ] **Step 2: 修改 add_skill 强制 enabled=true**

修改 `src-tauri/src/lib.rs` 第 660-666 行的 `add_skill` 函数：

```rust
#[tauri::command]
fn add_skill(app_handle: tauri::AppHandle, skill: config::Skill) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);
    app_config.features.skills.skills.push(skill);
    app_config.features.skills.enabled = true;
    config::save_config(&app_data_dir, &app_config)
}
```

- [ ] **Step 3: 修改 update_skill 强制 enabled=true**

修改 `src-tauri/src/lib.rs` 第 668-678 行的 `update_skill` 函数：

```rust
#[tauri::command]
fn update_skill(app_handle: tauri::AppHandle, skill: config::Skill) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);
    if let Some(existing) = app_config.features.skills.skills.iter_mut().find(|s| s.id == skill.id) {
        *existing = skill;
        app_config.features.skills.enabled = true;
        config::save_config(&app_data_dir, &app_config)
    } else {
        Err(format!("Skill not found: {}", skill.id))
    }
}
```

- [ ] **Step 4: 修改 delete_skill 强制 enabled=true**

修改 `src-tauri/src/lib.rs` 第 680-693 行的 `delete_skill` 函数：

```rust
#[tauri::command]
fn delete_skill(app_handle: tauri::AppHandle, skill_id: String) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);
    let skill = app_config.features.skills.skills.iter().find(|s| s.id == skill_id);
    if skill.is_none() {
        return Err(format!("Skill not found: {}", skill_id));
    }
    if skill.unwrap().builtin {
        return Err("Cannot delete builtin skill".to_string());
    }
    app_config.features.skills.skills.retain(|s| s.id != skill_id);
    app_config.features.skills.enabled = true;
    config::save_config(&app_data_dir, &app_config)
}
```

- [ ] **Step 5: 编译验证**

```bash
cd src-tauri && cargo check
```

Expected: 编译通过，无错误。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: 所有 Skills 配置保存操作强制 enabled=true"
```

---

### Task 5: 验证与测试

- [ ] **Step 1: 完整编译**

```bash
cd src-tauri && cargo build
```

Expected: 编译成功。

- [ ] **Step 2: 前端类型检查**

```bash
npx svelte-check
```

Expected: 无类型错误。

- [ ] **Step 3: 手动测试清单**

1. 打开模型配置页
   - 切换"启用润色"开关，确认润色模型选择框显示/隐藏正确
   - 关闭润色后选择润色模型（应不可见）
   - 开启润色后确认之前选择的模型值保留

2. 打开技能配置页
   - 确认不再显示"Skills 功能"全局开关
   - 确认单个 Skill 开关仍然正常工作
   - 添加/编辑/删除 Skill 功能正常

3. 测试完整管线
   - 配置润色模型
   - 启用至少一个 Skill
   - 执行录音转写，确认文字经过 Skills 处理
   - 关闭润色，确认 Skills 管线不再执行（因为 polish_provider/model 检查会通过，但 polish_enabled 不再阻止——实际上 Skills 仍会执行，只是 polish_provider/model 未配置时会跳过）

- [ ] **Step 4: 提交最终变更（如有）**

```bash
git add -A
git commit -m "test: 验证 Skills 与润色配置优化"
```
