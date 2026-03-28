# 模型页面 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 新增"模型"页面，将 AI Provider 配置和 Features 选择从数据结构暴露为完整 UI。

**Architecture:** 重构 Rust/TS 两端配置结构为泛化 ProviderConfig 数组，新增 Tauri save_config 命令，构建 3 个基础 UI 组件，组装模型页面，更新侧边栏导航。

**Tech Stack:** Svelte 5 + Tauri 2 (Rust) + 手写 CSS（无组件库）

**Design Spec:** `docs/superpowers/specs/2026-03-28-models-page-design.md`

---

### Task 1: 重构 Rust 配置结构

**Files:**
- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: 替换 config.rs 中所有结构体和默认值**

将 `src-tauri/src/config.rs` 的全部内容替换为：

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_SHORTCUT: &str = "Control+Shift+Quote";
const DEFAULT_RECORDING_SHORTCUT: &str = "Control+Backslash";
const CONFIG_FILE_NAME: &str = "config.json";

const DEFAULT_VERTEX_ENDPOINT: &str = "https://aiplatform.googleapis.com/v1";
const DEFAULT_DASHSCOPE_ENDPOINT: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ProviderConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub name: String,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub models: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct AiConfig {
    pub providers: Vec<ProviderConfig>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct TranscriptionConfig {
    pub provider_id: String,
    pub model: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct FeaturesConfig {
    pub transcription: TranscriptionConfig,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub shortcut: String,
    pub recording_shortcut: String,
    pub ai: AiConfig,
    pub features: FeaturesConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            shortcut: DEFAULT_SHORTCUT.to_string(),
            recording_shortcut: DEFAULT_RECORDING_SHORTCUT.to_string(),
            ai: AiConfig {
                providers: vec![
                    ProviderConfig {
                        id: "vertex".to_string(),
                        provider_type: "vertex".to_string(),
                        name: "VTX".to_string(),
                        endpoint: DEFAULT_VERTEX_ENDPOINT.to_string(),
                        api_key: None,
                        models: vec!["gemini-2.0-flash".to_string()],
                    },
                    ProviderConfig {
                        id: "dashscope".to_string(),
                        provider_type: "openai-compatible".to_string(),
                        name: "阿里云".to_string(),
                        endpoint: DEFAULT_DASHSCOPE_ENDPOINT.to_string(),
                        api_key: Some(String::new()),
                        models: vec!["qwen2-audio-instruct".to_string()],
                    },
                ],
            },
            features: FeaturesConfig {
                transcription: TranscriptionConfig {
                    provider_id: "vertex".to_string(),
                    model: "gemini-2.0-flash".to_string(),
                },
            },
        }
    }
}

pub fn config_file_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join(CONFIG_FILE_NAME)
}

pub fn load_config(app_data_dir: &PathBuf) -> AppConfig {
    let path = config_file_path(app_data_dir);
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        }
    } else {
        let config = AppConfig::default();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(&path, content);
        }
        config
    }
}

pub fn save_config(app_data_dir: &PathBuf, config: &AppConfig) -> Result<(), String> {
    let path = config_file_path(app_data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}
```

- [ ] **Step 2: 验证 Rust 编译通过**

Run: `cargo check`（在 `src-tauri/` 目录）
Expected: 编译通过，无错误

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "refactor: generalize Provider config to array-based structure"
```

---

### Task 2: 添加 save_config Tauri 命令

**Files:**
- Modify: `src-tauri/src/lib.rs:124-158`

- [ ] **Step 1: 在 lib.rs 中添加 save_config 命令**

在 `update_shortcut` 函数之后（第 158 行后）添加新的 Tauri 命令：

```rust
#[tauri::command]
fn save_config_cmd(app_handle: tauri::AppHandle, config: config::AppConfig) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    config::save_config(&app_data_dir, &config)
}
```

然后修改 `invoke_handler` 行（第 176 行），将 `save_config_cmd` 加入：

将：
```rust
.invoke_handler(tauri::generate_handler![get_config, update_shortcut])
```

替换为：
```rust
.invoke_handler(tauri::generate_handler![get_config, update_shortcut, save_config_cmd])
```

- [ ] **Step 2: 验证 Rust 编译通过**

Run: `cargo check`（在 `src-tauri/` 目录）
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add save_config Tauri command for full config persistence"
```

---

### Task 3: 重构 TS 配置 store

**Files:**
- Modify: `src/lib/stores/config.ts`

- [ ] **Step 1: 替换 config.ts 全部内容**

```typescript
import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';

export interface ProviderConfig {
  id: string;
  type: string;
  name: string;
  endpoint: string;
  api_key?: string;
  models: string[];
}

export interface AiConfig {
  providers: ProviderConfig[];
}

export interface TranscriptionConfig {
  provider_id: string;
  model: string;
}

export interface FeaturesConfig {
  transcription: TranscriptionConfig;
}

export interface AppConfig {
  shortcut: string;
  recording_shortcut: string;
  ai: AiConfig;
  features: FeaturesConfig;
}

function createConfigStore() {
  const { subscribe, set, update } = writable<AppConfig>({
    shortcut: 'Control+Shift+Quote',
    recording_shortcut: 'Control+Backslash',
    ai: {
      providers: [
        {
          id: 'vertex',
          type: 'vertex',
          name: 'VTX',
          endpoint: 'https://aiplatform.googleapis.com/v1',
          models: ['gemini-2.0-flash']
        },
        {
          id: 'dashscope',
          type: 'openai-compatible',
          name: '阿里云',
          endpoint: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
          api_key: '',
          models: ['qwen2-audio-instruct']
        }
      ]
    },
    features: {
      transcription: {
        provider_id: 'vertex',
        model: 'gemini-2.0-flash'
      }
    }
  });

  return {
    subscribe,
    load: async () => {
      try {
        const config = await invoke<AppConfig>('get_config');
        set(config);
      } catch (error) {
        console.error('Failed to load config:', error);
      }
    },
    updateShortcut: async (type: 'toggle' | 'recording', shortcut: string) => {
      try {
        await invoke('update_shortcut', { shortcutType: type, shortcut });
        update(config => {
          if (type === 'toggle') {
            return { ...config, shortcut };
          } else {
            return { ...config, recording_shortcut: shortcut };
          }
        });
      } catch (error) {
        console.error('Failed to update shortcut:', error);
        throw error;
      }
    },
    save: async (newConfig: AppConfig) => {
      try {
        await invoke('save_config_cmd', { config: newConfig });
        set(newConfig);
      } catch (error) {
        console.error('Failed to save config:', error);
        throw error;
      }
    }
  };
}

export const config = createConfigStore();
```

- [ ] **Step 2: 验证 TypeScript 编译**

Run: `npx svelte-check --tsconfig ./tsconfig.json`（在项目根目录）
Expected: 无类型错误

- [ ] **Step 3: Commit**

```bash
git add src/lib/stores/config.ts
git commit -m "refactor: generalize TS config store to ProviderConfig array"
```

---

### Task 4: 更新侧边栏导航

**Files:**
- Modify: `src/routes/+layout.svelte`

- [ ] **Step 1: 在 layout.svelte 中添加"模型"菜单项**

在 `<script>` 中修改 `activeMenu` 的逻辑：

将：
```typescript
let activeMenu = $derived($page.url.pathname === '/settings' ? 'settings' : 'home');
```

替换为：
```typescript
let activeMenu = $derived(
  $page.url.pathname === '/settings' ? 'settings' :
  $page.url.pathname === '/models' ? 'models' : 'home'
);
```

在 `<nav class="menu">` 中，在首页按钮和设置按钮之间添加模型按钮：

```svelte
<button
  class="menu-item"
  class:active={activeMenu === 'models'}
  onclick={() => navigateTo('/models')}
>
  <span class="icon">🤖</span>
  <span class="label">模型</span>
</button>
```

- [ ] **Step 2: 验证页面正常渲染**

Run: `npm run dev`，打开浏览器，检查侧边栏有三个菜单项，"模型"可点击跳转

- [ ] **Step 3: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "feat: add 🤖 模型 menu item to sidebar navigation"
```

---

### Task 5: 创建 GroupedSelect 组件

**Files:**
- Create: `src/lib/components/GroupedSelect.svelte`

- [ ] **Step 1: 编写 GroupedSelect 组件**

```svelte
<script lang="ts">
  interface Group {
    label: string;
    items: { value: string; label: string }[];
  }

  interface Props {
    value: string;
    groups: Group[];
    placeholder?: string;
    onChange: (value: string) => void;
  }

  let { value, groups, placeholder = '请选择', onChange }: Props = $props();

  let isOpen = $state(false);

  function toggle() {
    isOpen = !isOpen;
  }

  function select(val: string) {
    onChange(val);
    isOpen = false;
  }

  function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (!target.closest('.grouped-select')) {
      isOpen = false;
    }
  }

  function getDisplayLabel(): string {
    for (const group of groups) {
      for (const item of group.items) {
        if (item.value === value) return `${group.label} — ${item.label}`;
      }
    }
    return placeholder;
  }
</script>

<svelte:window onclick={handleClickOutside} />

<div class="grouped-select">
  <div class="select-trigger" class:open={isOpen} onclick={toggle}>
    <span class="select-value">{getDisplayLabel()}</span>
    <span class="select-arrow">▾</span>
  </div>
  {#if isOpen}
    <div class="select-dropdown">
      {#each groups as group}
        <div class="select-group">
          <div class="select-group-label">{group.label}</div>
          {#each group.items as item}
            <div
              class="select-option"
              class:selected={item.value === value}
              onclick={() => select(item.value)}
            >
              {item.label}
              {#if item.value === value}
                <span class="check">✓</span>
              {/if}
            </div>
          {/each}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .grouped-select {
    position: relative;
  }

  .select-trigger {
    background: #f8f8f8;
    border: 1px solid #ddd;
    border-radius: 6px;
    padding: 8px 10px;
    font-size: 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    cursor: pointer;
  }

  .select-trigger:hover {
    border-color: #ccc;
  }

  .select-trigger.open {
    border-color: #396cd8;
    box-shadow: 0 0 0 2px rgba(57, 108, 216, 0.15);
  }

  .select-arrow {
    color: #888;
    font-size: 12px;
  }

  .select-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    background: #fff;
    border: 1px solid #ddd;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    z-index: 100;
    max-height: 240px;
    overflow-y: auto;
  }

  .select-group-label {
    padding: 6px 10px;
    font-size: 10px;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-bottom: 1px solid #eee;
  }

  .select-option {
    padding: 6px 10px;
    font-size: 12px;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .select-option:hover {
    background: #f5f5f5;
  }

  .select-option.selected {
    background: #e8f0fe;
    color: #396cd8;
  }

  .check {
    color: #396cd8;
    font-size: 11px;
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/GroupedSelect.svelte
git commit -m "feat: add GroupedSelect component for provider-grouped model selection"
```

---

### Task 6: 创建 TagInput 组件

**Files:**
- Create: `src/lib/components/TagInput.svelte`

- [ ] **Step 1: 编写 TagInput 组件**

```svelte
<script lang="ts">
  interface Props {
    tags: string[];
    onAdd: (tag: string) => void;
    onRemove: (tag: string) => void;
    placeholder?: string;
  }

  let { tags, onAdd, onRemove, placeholder = '添加...' }: Props = $props();

  let inputValue = $state('');
  let isInputVisible = $state(false);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      addTag();
    } else if (event.key === 'Escape') {
      isInputVisible = false;
      inputValue = '';
    }
  }

  function addTag() {
    const trimmed = inputValue.trim();
    if (trimmed && !tags.includes(trimmed)) {
      onAdd(trimmed);
      inputValue = '';
    }
  }

  function showInput() {
    isInputVisible = true;
  }
</script>

<div class="tag-input">
  <div class="tags">
    {#each tags as tag}
      <span class="tag">
        {tag}
        <button class="tag-remove" onclick={() => onRemove(tag)}>✕</button>
      </span>
    {/each}
  </div>
  {#if isInputVisible}
    <input
      class="tag-field"
      type="text"
      {placeholder}
      bind:value={inputValue}
      onkeydown={handleKeydown}
      onblur={() => { addTag(); isInputVisible = false; }}
    />
  {:else}
    <button class="add-tag-btn" onclick={showInput}>+ 添加模型</button>
  {/if}
</div>

<style>
  .tag-input {
    margin-top: 4px;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 4px;
  }

  .tag {
    background: #e8f0fe;
    color: #396cd8;
    border-radius: 4px;
    padding: 3px 8px;
    font-size: 10px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .tag-remove {
    background: none;
    border: none;
    color: #396cd8;
    cursor: pointer;
    font-size: 10px;
    padding: 0;
    opacity: 0.6;
  }

  .tag-remove:hover {
    opacity: 1;
  }

  .tag-field {
    width: 100%;
    background: #f8f8f8;
    border: 1px solid #ddd;
    border-radius: 4px;
    padding: 5px 8px;
    font-size: 11px;
    outline: none;
  }

  .tag-field:focus {
    border-color: #396cd8;
    box-shadow: 0 0 0 2px rgba(57, 108, 216, 0.15);
  }

  .add-tag-btn {
    background: none;
    border: none;
    color: #396cd8;
    cursor: pointer;
    font-size: 10px;
    padding: 0;
  }

  .add-tag-btn:hover {
    text-decoration: underline;
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/TagInput.svelte
git commit -m "feat: add TagInput component for model tag management"
```

---

### Task 7: 创建 PasswordInput 组件

**Files:**
- Create: `src/lib/components/PasswordInput.svelte`

- [ ] **Step 1: 编写 PasswordInput 组件**

```svelte
<script lang="ts">
  interface Props {
    value: string;
    placeholder?: string;
    onChange: (value: string) => void;
  }

  let { value, placeholder = '', onChange }: Props = $props();

  let visible = $state(false);

  function mask(val: string): string {
    if (!val) return '';
    return val.slice(0, 3) + '•'.repeat(Math.max(val.length - 3, 6));
  }
</script>

<div class="password-input">
  {#if visible}
    <input
      class="field"
      type="text"
      {placeholder}
      {value}
      oninput={(e) => onChange((e.target as HTMLInputElement).value)}
    />
  {:else}
    <div class="field masked">{mask(value)}</div>
  {/if}
  <button class="toggle-btn" onclick={() => visible = !visible} title={visible ? '隐藏' : '显示'}>
    👁
  </button>
</div>

<style>
  .password-input {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .field {
    flex: 1;
    background: #f8f8f8;
    border: 1px solid #ddd;
    border-radius: 4px;
    padding: 6px 8px;
    font-size: 11px;
    outline: none;
  }

  .field:focus {
    border-color: #396cd8;
    box-shadow: 0 0 0 2px rgba(57, 108, 216, 0.15);
  }

  .masked {
    color: #666;
    user-select: none;
  }

  .toggle-btn {
    padding: 4px 8px;
    font-size: 12px;
    cursor: pointer;
    color: #888;
    background: #f8f8f8;
    border: 1px solid #ddd;
    border-radius: 4px;
  }

  .toggle-btn:hover {
    background: #f0f0f0;
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/PasswordInput.svelte
git commit -m "feat: add PasswordInput component with mask toggle"
```

---

### Task 8: 创建模型页面

**Files:**
- Create: `src/routes/models/+page.svelte`

- [ ] **Step 1: 编写模型页面**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { config } from '$lib/stores/config';
  import GroupedSelect from '$lib/components/GroupedSelect.svelte';
  import TagInput from '$lib/components/TagInput.svelte';
  import PasswordInput from '$lib/components/PasswordInput.svelte';
  import type { ProviderConfig, AppConfig } from '$lib/stores/config';

  onMount(() => {
    config.load();
  });

  function buildTranscriptionGroups() {
    return ($config.ai.providers || []).map((p: ProviderConfig) => ({
      label: p.name,
      items: (p.models || []).map((m: string) => ({
        value: `${p.id}::${m}`,
        label: m
      }))
    }));
  }

  function getTranscriptionValue(): string {
    const t = $config.features.transcription;
    if (t.provider_id && t.model) {
      return `${t.provider_id}::${t.model}`;
    }
    return '';
  }

  function handleTranscriptionChange(val: string) {
    const [providerId, model] = val.split('::');
    const newConfig: AppConfig = {
      ...$config,
      features: {
        transcription: { provider_id: providerId, model }
      }
    };
    config.save(newConfig);
  }

  function handleProviderFieldChange(
    providerId: string,
    field: string,
    value: string
  ) {
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === providerId ? { ...p, [field]: value } : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function handleApiKeyChange(providerId: string, value: string) {
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === providerId ? { ...p, api_key: value } : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function handleAddModel(providerId: string, model: string) {
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === providerId
        ? { ...p, models: [...p.models, model] }
        : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function handleRemoveModel(providerId: string, model: string) {
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === providerId
        ? { ...p, models: p.models.filter((m: string) => m !== model) }
        : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function handleRemoveProvider(providerId: string) {
    const newProviders = $config.ai.providers.filter(
      (p: ProviderConfig) => p.id !== providerId
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function needsApiKey(provider: ProviderConfig): boolean {
    return provider.type === 'openai-compatible';
  }
</script>

<div class="models-page">
  <h2>模型</h2>

  <section class="section">
    <div class="section-label">Features</div>
    <div class="features-grid">
      <div class="feature-card">
        <div class="feature-name">Transcription</div>
        <div class="feature-desc">转写服务</div>
        <GroupedSelect
          value={getTranscriptionValue()}
          groups={buildTranscriptionGroups()}
          placeholder="选择模型"
          onChange={handleTranscriptionChange}
        />
      </div>
    </div>
  </section>

  <section class="section">
    <div class="section-label">Providers</div>
    <div class="providers-grid">
      {#each $config.ai.providers || [] as provider (provider.id)}
        <div class="provider-card">
          <div class="provider-header">
            <div>
              <div class="provider-name">{provider.name}</div>
              <div class="provider-subtitle">{provider.id}</div>
            </div>
            <button class="provider-remove" onclick={() => handleRemoveProvider(provider.id)}>✕</button>
          </div>

          {#if needsApiKey(provider)}
            <div class="field-group">
              <label class="field-label">API Key</label>
              <PasswordInput
                value={provider.api_key || ''}
                placeholder="sk-..."
                onChange={(val: string) => handleApiKeyChange(provider.id, val)}
              />
            </div>
          {/if}

          <div class="field-group">
            <label class="field-label">Endpoint</label>
            <input
              class="field"
              type="text"
              value={provider.endpoint}
              onchange={(e) => handleProviderFieldChange(provider.id, 'endpoint', (e.target as HTMLInputElement).value)}
            />
          </div>

          <div class="field-group">
            <label class="field-label">Models</label>
            <TagInput
              tags={provider.models}
              onAdd={(tag: string) => handleAddModel(provider.id, tag)}
              onRemove={(tag: string) => handleRemoveModel(provider.id, tag)}
            />
          </div>
        </div>
      {/each}
    </div>
  </section>
</div>

<style>
  .models-page {
    max-width: 800px;
  }

  h2 {
    margin: 0 0 24px 0;
    font-size: 20px;
    font-weight: 600;
    color: #333;
  }

  .section {
    margin-bottom: 28px;
  }

  .section-label {
    font-size: 11px;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: 10px;
  }

  .features-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
  }

  .feature-card {
    background: #fff;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    padding: 14px;
  }

  .feature-name {
    font-size: 13px;
    font-weight: 600;
    color: #333;
    margin-bottom: 2px;
  }

  .feature-desc {
    font-size: 11px;
    color: #999;
    margin-bottom: 10px;
  }

  .providers-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }

  .provider-card {
    background: #fff;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    padding: 14px;
  }

  .provider-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 12px;
  }

  .provider-name {
    font-size: 14px;
    font-weight: 600;
    color: #333;
  }

  .provider-subtitle {
    font-size: 10px;
    color: #aaa;
    margin-top: 1px;
  }

  .provider-remove {
    background: none;
    border: none;
    font-size: 10px;
    color: #ccc;
    cursor: pointer;
    padding: 2px;
  }

  .provider-remove:hover {
    color: #d32f2f;
  }

  .field-group {
    margin-bottom: 10px;
  }

  .field-label {
    display: block;
    font-size: 11px;
    color: #888;
    margin-bottom: 4px;
  }

  .field {
    width: 100%;
    background: #f8f8f8;
    border: 1px solid #ddd;
    border-radius: 4px;
    padding: 6px 8px;
    font-size: 11px;
    outline: none;
    word-break: break-all;
    box-sizing: border-box;
  }

  .field:focus {
    border-color: #396cd8;
    box-shadow: 0 0 0 2px rgba(57, 108, 216, 0.15);
  }
</style>
```

- [ ] **Step 2: 验证完整功能**

Run: `npm run dev` + `cargo tauri dev`（在项目根目录）
验证：
1. 侧边栏显示 🤖 模型，点击可进入页面
2. Features 区域显示 Transcription 卡片和分组下拉框
3. Providers 区域显示 VTX 和阿里云两张卡片
4. 下拉框可切换模型，切换后自动保存
5. Models tag 可添加和删除
6. API Key 有遮罩，可切换
7. Endpoint 可编辑，失焦后保存

- [ ] **Step 3: Commit**

```bash
git add src/routes/models/+page.svelte
git commit -m "feat: add models page with Features and Providers configuration UI"
```
