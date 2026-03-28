# 设定页面实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 TalkShow 主界面中添加一个设定页面，采用经典侧边栏布局，支持快捷键设置功能。

**Architecture:** 使用 SvelteKit 路由实现页面切换，创建 Layout 组件提供左右栏布局，创建 ShortcutRecorder 组件处理快捷键录制逻辑。后端添加 Tauri 命令支持配置读写和快捷键动态更新。

**Tech Stack:** SvelteKit, Svelte 5 (runes), TypeScript, Tauri, Rust

---

## 文件结构

### 前端文件
- `src/routes/+layout.svelte` - 根布局组件，提供左右栏结构
- `src/routes/+page.svelte` - 首页（空白内容）
- `src/routes/settings/+page.svelte` - 设置页面
- `src/lib/components/ShortcutRecorder.svelte` - 快捷键录制组件
- `src/lib/stores/config.ts` - 配置状态管理

### 后端文件
- `src-tauri/src/lib.rs` - 添加 Tauri 命令
- `src-tauri/src/config.rs` - 配置模块（已有）

---

## Task 1: 创建 Layout 组件

**Files:**
- Create: `src/routes/+layout.svelte`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: 创建 Layout 组件**

```svelte
<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';

  let activeMenu = $derived($page.url.pathname === '/settings' ? 'settings' : 'home');

  function navigateTo(path: string) {
    goto(path);
  }
</script>

<div class="app-container">
  <aside class="sidebar">
    <div class="logo">TalkShow</div>
    <nav class="menu">
      <button
        class="menu-item"
        class:active={activeMenu === 'home'}
        onclick={() => navigateTo('/')}
      >
        <span class="icon">🏠</span>
        <span class="label">首页</span>
      </button>
      <button
        class="menu-item"
        class:active={activeMenu === 'settings'}
        onclick={() => navigateTo('/settings')}
      >
        <span class="icon">⚙️</span>
        <span class="label">设置</span>
      </button>
    </nav>
  </aside>
  <main class="content">
    <slot />
  </main>
</div>

<style>
  .app-container {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }

  .sidebar {
    width: 160px;
    background-color: #f5f5f5;
    border-right: 1px solid #e0e0e0;
    display: flex;
    flex-direction: column;
  }

  .logo {
    padding: 16px 20px;
    font-weight: 600;
    font-size: 14px;
    color: #333;
    border-bottom: 1px solid #e0e0e0;
  }

  .menu {
    padding: 8px 0;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    width: 100%;
    border: none;
    background: none;
    cursor: pointer;
    font-size: 14px;
    color: #333;
    text-align: left;
  }

  .menu-item:hover {
    background-color: #e8e8e8;
  }

  .menu-item.active {
    background-color: #e8e8e8;
    border-left: 3px solid #396cd8;
  }

  .icon {
    font-size: 16px;
  }

  .content {
    flex: 1;
    padding: 24px;
    overflow-y: auto;
    background-color: #fff;
  }
</style>
```

- [ ] **Step 2: 更新首页组件**

```svelte
<script lang="ts">
</script>

<div class="home-page">
  <h1>欢迎使用 TalkShow</h1>
  <p>这是一个语音转文字应用</p>
</div>

<style>
  .home-page {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    text-align: center;
  }

  h1 {
    margin-bottom: 8px;
    color: #333;
  }

  p {
    color: #666;
  }
</style>
```

- [ ] **Step 3: 验证布局效果**

运行: `npm run dev`
预期: 浏览器中显示左右栏布局，左侧菜单可点击切换

- [ ] **Step 4: 提交更改**

```bash
git add src/routes/+layout.svelte src/routes/+page.svelte
git commit -m "feat: add layout component with sidebar navigation"
```

---

## Task 2: 创建配置状态管理

**Files:**
- Create: `src/lib/stores/config.ts`

- [ ] **Step 1: 创建配置状态管理**

```typescript
import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';

export interface AppConfig {
  shortcut: string;
  recording_shortcut: string;
}

function createConfigStore() {
  const { subscribe, set, update } = writable<AppConfig>({
    shortcut: 'Control+Shift+Quote',
    recording_shortcut: 'Control+Backslash'
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
    }
  };
}

export const config = createConfigStore();
```

- [ ] **Step 2: 提交更改**

```bash
git add src/lib/stores/config.ts
git commit -m "feat: add config store for shortcut management"
```

---

## Task 3: 创建快捷键录制组件

**Files:**
- Create: `src/lib/components/ShortcutRecorder.svelte`

- [ ] **Step 1: 创建快捷键录制组件**

```svelte
<script lang="ts">
  interface Props {
    label: string;
    description: string;
    value: string;
    onUpdate: (shortcut: string) => Promise<void>;
  }

  let { label, description, value, onUpdate }: Props = $props();

  let isRecording = $state(false);
  let currentValue = $state(value);
  let error = $state<string | null>(null);

  $effect(() => {
    currentValue = value;
  });

  function formatShortcut(shortcut: string): string {
    return shortcut
      .replace('Control', '⌃')
      .replace('Shift', '⇧')
      .replace('Alt', '⌥')
      .replace('Command', '⌘')
      .replace('Super', '⌘')
      .replace('Quote', "'")
      .replace('Backslash', '\\')
      .replace('Key', '')
      .replace('Digit', '')
      .replace('+', ' ');
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!isRecording) return;

    event.preventDefault();
    event.stopPropagation();

    if (event.key === 'Escape') {
      isRecording = false;
      currentValue = value;
      return;
    }

    const modifiers: string[] = [];
    if (event.ctrlKey) modifiers.push('Control');
    if (event.shiftKey) modifiers.push('Shift');
    if (event.altKey) modifiers.push('Alt');
    if (event.metaKey) modifiers.push('Command');

    let key = event.code;
    if (key.startsWith('Key')) {
      key = key;
    } else if (key.startsWith('Digit')) {
      key = key;
    } else if (key === 'Quote') {
      key = 'Quote';
    } else if (key === 'Backslash') {
      key = 'Backslash';
    } else if (key === 'Space') {
      key = 'Space';
    } else {
      return;
    }

    if (modifiers.length === 0) {
      error = '请至少使用一个修饰键 (Ctrl, Shift, Alt, Command)';
      return;
    }

    const shortcut = [...modifiers, key].join('+');
    saveShortcut(shortcut);
  }

  async function saveShortcut(shortcut: string) {
    try {
      error = null;
      await onUpdate(shortcut);
      currentValue = shortcut;
      isRecording = false;
    } catch (e) {
      error = e instanceof Error ? e.message : '保存失败';
    }
  }

  function startRecording() {
    isRecording = true;
    error = null;
  }

  function cancelRecording() {
    isRecording = false;
    currentValue = value;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="shortcut-item">
  <div class="info">
    <h4>{label}</h4>
    <p>{description}</p>
  </div>
  <div class="controls">
    <div class="shortcut-display" class:recording={isRecording}>
      {#if isRecording}
        请按下快捷键...
      {:else}
        {formatShortcut(currentValue)}
      {/if}
    </div>
    {#if isRecording}
      <button class="btn cancel" onclick={cancelRecording}>
        取消
      </button>
    {:else}
      <button class="btn modify" onclick={startRecording}>
        修改
      </button>
    {/if}
  </div>
  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .shortcut-item {
    background: white;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    padding: 20px;
    margin-bottom: 16px;
  }

  .info h4 {
    margin: 0 0 4px 0;
    font-size: 14px;
    font-weight: 600;
    color: #333;
  }

  .info p {
    margin: 0;
    font-size: 13px;
    color: #666;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 12px;
  }

  .shortcut-display {
    background: #f0f0f0;
    padding: 8px 16px;
    border-radius: 6px;
    font-family: monospace;
    font-size: 14px;
    min-width: 120px;
    text-align: center;
  }

  .shortcut-display.recording {
    background: #396cd8;
    color: white;
  }

  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    border: 1px solid #ccc;
    background: white;
    cursor: pointer;
    font-size: 13px;
  }

  .btn:hover {
    border-color: #396cd8;
  }

  .btn.cancel {
    border-color: #396cd8;
    background: #396cd8;
    color: white;
  }

  .error {
    margin: 8px 0 0 0;
    color: #d32f2f;
    font-size: 12px;
  }
</style>
```

- [ ] **Step 2: 提交更改**

```bash
git add src/lib/components/ShortcutRecorder.svelte
git commit -m "feat: add shortcut recorder component"
```

---

## Task 4: 创建设置页面

**Files:**
- Create: `src/routes/settings/+page.svelte`

- [ ] **Step 1: 创建设置页面**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { config } from '$lib/stores/config';
  import ShortcutRecorder from '$lib/components/ShortcutRecorder.svelte';

  onMount(() => {
    config.load();
  });

  async function handleUpdateToggle(shortcut: string) {
    await config.updateShortcut('toggle', shortcut);
  }

  async function handleUpdateRecording(shortcut: string) {
    await config.updateShortcut('recording', shortcut);
  }
</script>

<div class="settings-page">
  <h2>快捷键设置</h2>

  <ShortcutRecorder
    label="窗口切换"
    description="显示或隐藏主窗口"
    value={$config.shortcut}
    onUpdate={handleUpdateToggle}
  />

  <ShortcutRecorder
    label="录音控制"
    description="开始或结束录音"
    value={$config.recording_shortcut}
    onUpdate={handleUpdateRecording}
  />

  <div class="hint">
    <p>💡 提示：点击"修改"按钮后，直接按下键盘上的组合键即可完成设置。按 Esc 取消录制。</p>
  </div>
</div>

<style>
  .settings-page {
    max-width: 600px;
  }

  h2 {
    margin: 0 0 24px 0;
    font-size: 20px;
    font-weight: 600;
    color: #333;
  }

  .hint {
    background: #fff9e6;
    border: 1px solid #ffd666;
    border-radius: 8px;
    padding: 16px;
    margin-top: 20px;
  }

  .hint p {
    margin: 0;
    color: #d48806;
    font-size: 13px;
  }
</style>
```

- [ ] **Step 2: 提交更改**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat: add settings page with shortcut configuration"
```

---

## Task 5: 添加 Tauri 命令

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 添加 Tauri 命令**

在 `lib.rs` 中添加以下命令：

```rust
#[tauri::command]
fn get_config(app_handle: tauri::AppHandle) -> config::AppConfig {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    config::load_config(&app_data_dir)
}

#[tauri::command]
fn update_shortcut(
    app_handle: tauri::AppHandle,
    shortcut_type: String,
    shortcut: String,
) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut config = config::load_config(&app_data_dir);
    
    match shortcut_type.as_str() {
        "toggle" => config.shortcut = shortcut,
        "recording" => config.recording_shortcut = shortcut,
        _ => return Err("Invalid shortcut type".to_string()),
    }
    
    config::save_config(&app_data_dir, &config)?;
    
    // Re-register shortcuts
    if let Some(sc) = parse_shortcut(&config.shortcut) {
        let _ = app_handle.global_shortcut().unregister(sc.clone());
        let _ = app_handle.global_shortcut().register(sc);
    }
    if let Some(sc) = parse_shortcut(&config.recording_shortcut) {
        let _ = app_handle.global_shortcut().unregister(sc.clone());
        let _ = app_handle.global_shortcut().register(sc);
    }
    
    Ok(())
}
```

- [ ] **Step 2: 注册命令到 Tauri Builder**

修改 `pub fn run()` 函数，在 `.setup()` 之前添加：

```rust
.invoke_handler(tauri::generate_handler![get_config, update_shortcut])
```

- [ ] **Step 3: 提交更改**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add Tauri commands for config management"
```

---

## Task 6: 全局样式调整

**Files:**
- Modify: `src/app.html`
- Modify: `src/routes/+layout.svelte`

- [ ] **Step 1: 更新全局样式**

在 `src/app.html` 中添加全局样式：

```html
<style>
  * {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  html, body {
    height: 100%;
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    font-size: 16px;
    line-height: 24px;
    font-weight: 400;
    color: #0f0f0f;
    background-color: #ffffff;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  #svelte {
    height: 100%;
  }
</style>
```

- [ ] **Step 2: 提交更改**

```bash
git add src/app.html
git commit -m "style: add global styles for settings page"
```

---

## Task 7: 测试和验证

- [ ] **Step 1: 启动开发服务器**

运行: `npm run dev`
预期: 应用正常启动，无错误

- [ ] **Step 2: 测试页面切换**

点击左侧菜单的"首页"和"设置"，验证页面切换正常

- [ ] **Step 3: 测试快捷键设置**

1. 进入设置页面
2. 点击"修改"按钮
3. 按下组合键（如 Ctrl+Shift+A）
4. 验证快捷键显示更新
5. 按 Esc 取消录制，验证恢复原值

- [ ] **Step 4: 测试快捷键生效**

1. 设置新的窗口切换快捷键
2. 在应用外按下该快捷键
3. 验证窗口显示/隐藏功能正常

- [ ] **Step 5: 提交最终更改**

```bash
git add -A
git commit -m "feat: complete settings page implementation"
```

---

## 自我审查

1. **规范覆盖:** 所有规范中的需求都已实现
   - ✅ 左右栏布局
   - ✅ 首页和设置页面
   - ✅ 快捷键录制交互
   - ✅ 配置持久化

2. **占位符扫描:** 无 TBD、TODO 或不完整部分

3. **类型一致性:** 前后端类型定义一致
   - AppConfig 结构体匹配
   - 快捷键类型定义一致
