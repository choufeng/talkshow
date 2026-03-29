# 日志系统 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 TalkShow 引入结构化日志系统，后端写入 JSON Lines 文件，前端提供可浏览、可过滤的日志页面。

**Architecture:** 后端新增 `logger.rs` 模块，作为 Tauri managed state 在全局可用。每次应用启动创建一个新的 `.jsonl` 日志文件，自动清理超出 10 个的旧文件。前端新增 `/logs` 路由页面，通过 Tauri 命令读取日志内容，支持按模块过滤。

**Tech Stack:** Rust (chrono, serde_json, std::fs) / Svelte 5 + TypeScript / Tauri v2 IPC

---

## File Structure

| 操作 | 文件 | 职责 |
|------|------|------|
| 新增 | `src-tauri/src/logger.rs` | Logger 结构体、文件管理、JSON Lines 写入、Tauri 命令 |
| 修改 | `src-tauri/src/lib.rs` | 注册 Logger state、新增 2 个 Tauri 命令到 handler、改造 test_model_connectivity |
| 新增 | `src/routes/logs/+page.svelte` | 日志页面（会话切换、模块过滤、日志列表） |
| 修改 | `src/routes/+layout.svelte` | 侧边栏新增日志导航项 |

---

### Task 1: 创建 logger.rs — 数据结构与文件管理

**Files:**
- Create: `src-tauri/src/logger.rs`

- [ ] **Step 1: 创建 logger.rs 基础结构**

```rust
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

const MAX_LOG_FILES: usize = 10;
const LOG_DIR_NAME: &str = "logs";

#[derive(Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub ts: String,
    pub module: String,
    pub level: String,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct LogSession {
    pub filename: String,
    pub size_bytes: u64,
    pub entry_count: usize,
    pub first_ts: Option<String>,
    pub is_current: bool,
}

pub struct Logger {
    log_dir: PathBuf,
    current_file: Mutex<fs::File>,
    current_filename: String,
}

impl Logger {
    pub fn new(app_data_dir: &std::path::Path) -> Result<Self, String> {
        let log_dir = app_data_dir.join(LOG_DIR_NAME);
        fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;

        let now = Local::now();
        let filename = format!("talkshow-{}.jsonl", now.format("%Y-%m-%d_%H-%M-%S"));
        let filepath = log_dir.join(&filename);
        let file = fs::File::options()
            .create_new(true)
            .append(true)
            .open(&filepath)
            .map_err(|e| format!("Failed to create log file: {}", e))?;

        cleanup_old_logs(&log_dir, MAX_LOG_FILES);

        Ok(Self {
            log_dir,
            current_file: Mutex::new(file),
            current_filename: filename,
        })
    }

    pub fn log(
        &self,
        module: &str,
        level: &str,
        msg: &str,
        meta: Option<serde_json::Value>,
    ) {
        let entry = LogEntry {
            ts: Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            module: module.to_string(),
            level: level.to_string(),
            msg: msg.to_string(),
            meta,
        };

        let mut line = match serde_json::to_string(&entry) {
            Ok(s) => s,
            Err(_) => return,
        };
        line.push('\n');

        if let Ok(mut file) = self.current_file.lock() {
            let _ = file.write_all(line.as_bytes());
        }
    }

    pub fn info(&self, module: &str, msg: &str, meta: Option<serde_json::Value>) {
        self.log(module, "info", msg, meta);
    }

    pub fn warn(&self, module: &str, msg: &str, meta: Option<serde_json::Value>) {
        self.log(module, "warn", msg, meta);
    }

    pub fn error(&self, module: &str, msg: &str, meta: Option<serde_json::Value>) {
        self.log(module, "error", msg, meta);
    }

    pub fn get_sessions(&self) -> Vec<LogSession> {
        let mut sessions: Vec<LogSession> = Vec::new();

        let entries = match fs::read_dir(&self.log_dir) {
            Ok(e) => e,
            Err(_) => return sessions,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let metadata = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let content = fs::read_to_string(&path).unwrap_or_default();
            let entry_count = content.lines().filter(|l| !l.is_empty()).count();

            let first_ts = content
                .lines()
                .next()
                .and_then(|line| serde_json::from_str::<LogEntry>(line).ok())
                .map(|e| e.ts);

            let is_current = filename == self.current_filename;

            sessions.push(LogSession {
                filename,
                size_bytes: metadata.len(),
                entry_count,
                first_ts,
                is_current,
            });
        }

        sessions.sort_by(|a, b| b.filename.cmp(&a.filename));
        sessions
    }

    pub fn get_content(&self, session_file: Option<&str>) -> Vec<LogEntry> {
        let filename = session_file.unwrap_or(&self.current_filename);
        let filepath = self.log_dir.join(filename);

        let content = match fs::read_to_string(&filepath) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        content
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|line| serde_json::from_str::<LogEntry>(line).ok())
            .collect()
    }
}

fn cleanup_old_logs(log_dir: &std::path::Path, max_files: usize) {
    let mut files: Vec<(String, std::time::SystemTime)> = Vec::new();

    let entries = match fs::read_dir(log_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let modified = entry.metadata().ok().and_then(|m| m.modified().ok());
        if let Some(modified) = modified {
            files.push((filename, modified));
        }
    }

    files.sort_by(|a, b| b.1.cmp(&a.1));

    if files.len() > max_files {
        for (_, filename) in files.iter().skip(max_files) {
            let _ = fs::remove_file(log_dir.join(filename));
        }
    }
}

#[tauri::command]
pub fn get_log_sessions(app_handle: tauri::AppHandle) -> Vec<LogSession> {
    let logger = app_handle.state::<Logger>();
    logger.get_sessions()
}

#[tauri::command]
pub fn get_log_content(
    app_handle: tauri::AppHandle,
    session_file: Option<String>,
) -> Vec<LogEntry> {
    let logger = app_handle.state::<Logger>();
    logger.get_content(session_file.as_deref())
}
```

- [ ] **Step 2: 验证编译**

Run: `cd /Users/jia.xia/development/talkshow/src-tauri && cargo check 2>&1 | head -20`

此时会报错因为 lib.rs 还没有引入模块和注册，这在后续 Task 中完成。但需确认 `logger.rs` 自身语法无误。

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/logger.rs
git commit -m "feat: add logger module with JSON Lines file management"
```

---

### Task 2: 集成 Logger 到 lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 lib.rs 顶部添加 mod logger 和 use 声明**

在 `lib.rs` 第 4 行 `mod recording;` 后添加：

```rust
mod logger;
```

在 `use` 块中添加（第 6 行附近，在现有 use 之后）：

```rust
use logger::Logger;
```

- [ ] **Step 2: 在 setup 闭包中初始化 Logger 并注册为 managed state**

在 `lib.rs` 的 `.setup(|app| {` 闭包中（约第 346 行 `let app_data_dir = ...` 之后），添加 Logger 初始化：

在以下代码之后：
```rust
let app_data_dir = app.path().app_data_dir().unwrap_or_default();
```

插入：
```rust
let logger = Logger::new(&app_data_dir)
    .expect("Failed to initialize logger");
```

然后在 setup 闭包的 `Ok(())` 之前，添加 managed state 注册：

```rust
app.manage(logger);
```

注意：这行必须在 setup 闭包内部，`Ok(())` 之前。

- [ ] **Step 3: 将 logger Tauri 命令注册到 invoke_handler**

修改 `lib.rs` 中的 `invoke_handler` 宏调用（约第 339-344 行）：

将：
```rust
.invoke_handler(tauri::generate_handler![
    get_config,
    update_shortcut,
    save_config_cmd,
    test_model_connectivity
])
```

改为：
```rust
.invoke_handler(tauri::generate_handler![
    get_config,
    update_shortcut,
    save_config_cmd,
    test_model_connectivity,
    logger::get_log_sessions,
    logger::get_log_content
])
```

- [ ] **Step 4: 验证编译通过**

Run: `cd /Users/jia.xia/development/talkshow/src-tauri && cargo check 2>&1`

Expected: 编译成功，无错误

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: integrate Logger as Tauri managed state with log commands"
```

---

### Task 3: 改造 test_model_connectivity 添加日志记录

**Files:**
- Modify: `src-tauri/src/lib.rs` (test_model_connectivity 函数，约第 238-309 行)

- [ ] **Step 1: 在 test_model_connectivity 中添加日志记录**

将整个 `test_model_connectivity` 函数（第 238-309 行）替换为以下代码：

```rust
#[tauri::command]
async fn test_model_connectivity(
    app_handle: tauri::AppHandle,
    provider_id: String,
    model_name: String,
) -> Result<TestResult, String> {
    let logger = app_handle.state::<Logger>();
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);

    let provider = app_config
        .ai
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?
        .clone();

    let model = provider
        .models
        .iter()
        .find(|m| m.name == model_name)
        .ok_or_else(|| format!("Model not found: {}", model_name))?
        .clone();

    let is_transcription = model.capabilities.iter().any(|c| c == "transcription");

    logger.info(
        "connectivity",
        &format!("开始测试模型连通性: {}/{}", provider_id, model_name),
        Some(serde_json::json!({
            "provider_id": provider_id,
            "model_name": model_name,
            "is_transcription": is_transcription,
        })),
    );

    let test_audio: &[u8] = include_bytes!("../assets/test.wav");

    let start = Instant::now();
    let result = if is_transcription {
        ai::send_audio_prompt_from_bytes(
            test_audio,
            "audio/wav",
            "请将这段音频转录为文字",
            &model_name,
            &provider,
        )
        .await
    } else {
        ai::send_text_prompt("Hi", &model_name, &provider).await
    };
    let latency = start.elapsed().as_millis() as u64;

    let (status, message) = match &result {
        Ok(text) => {
            let summary: String = text.chars().take(50).collect();
            logger.info(
                "connectivity",
                &format!("测试成功: {}/{}", provider_id, model_name),
                Some(serde_json::json!({
                    "provider_id": provider_id,
                    "model_name": model_name,
                    "latency_ms": latency,
                    "response_summary": summary,
                })),
            );
            ("ok".to_string(), summary)
        }
        Err(e) => {
            let error_str = e.to_string();
            logger.error(
                "connectivity",
                &format!("测试失败: {}/{}", provider_id, model_name),
                Some(serde_json::json!({
                    "provider_id": provider_id,
                    "model_name": model_name,
                    "latency_ms": latency,
                    "error": error_str,
                })),
            );
            ("error".to_string(), error_str)
        }
    };

    let verified = config::ModelVerified {
        status: status.clone(),
        tested_at: chrono::Utc::now().to_rfc3339(),
        latency_ms: Some(latency),
        message: if status == "error" {
            Some(message.clone())
        } else {
            None
        },
    };

    if let Some(p) = app_config.ai.providers.iter_mut().find(|p| p.id == provider_id) {
        if let Some(m) = p.models.iter_mut().find(|m| m.name == model_name) {
            m.verified = Some(verified);
        }
    }
    config::save_config(&app_data_dir, &app_config)?;

    Ok(TestResult {
        status,
        latency_ms: Some(latency),
        message,
    })
}
```

- [ ] **Step 2: 验证编译通过**

Run: `cd /Users/jia.xia/development/talkshow/src-tauri && cargo check 2>&1`

Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add connectivity test logging to test_model_connectivity"
```

---

### Task 4: 侧边栏添加日志导航项

**Files:**
- Modify: `src/routes/+layout.svelte`

- [ ] **Step 1: 添加 ScrollText 图标导入和路由判断**

修改 `+layout.svelte` 的 `<script>` 部分。

第 6 行，将：
```typescript
import { House, Settings, Bot } from 'lucide-svelte';
```
改为：
```typescript
import { House, Settings, Bot, ScrollText } from 'lucide-svelte';
```

第 10-13 行，将 `activeMenu` 的 derived 替换为：
```typescript
let activeMenu = $derived(
    $page.url.pathname === '/settings' ? 'settings' :
    $page.url.pathname === '/models' ? 'models' :
    $page.url.pathname === '/logs' ? 'logs' : 'home'
);
```

- [ ] **Step 2: 在侧边栏 nav 中添加日志按钮**

在设置按钮（`<Settings>` 那个 button）的结束标签 `</button>` 后面（约第 46 行后），添加：

```svelte
      <button
        class="flex items-center gap-2 px-5 py-2.5 w-full text-sm text-foreground text-left transition-colors {activeMenu === 'logs' ? 'bg-muted border-l-[3px] border-l-accent-foreground' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/logs')}
      >
        <ScrollText size={18} class="shrink-0" />
        <span>日志</span>
      </button>
```

- [ ] **Step 3: 验证前端无编译错误**

Run: `cd /Users/jia.xia/development/talkshow && npx svelte-check --output human 2>&1 | tail -5`

Expected: 无错误（`/logs` 路由页面将在下一个 Task 中创建，可能报路由缺失，可忽略）

- [ ] **Step 4: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "feat: add logs navigation item to sidebar"
```

---

### Task 5: 创建日志页面

**Files:**
- Create: `src/routes/logs/+page.svelte`

- [ ] **Step 1: 创建日志页面组件**

创建 `src/routes/logs/+page.svelte`，内容如下：

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { ScrollText } from 'lucide-svelte';

  interface LogEntry {
    ts: string;
    module: string;
    level: string;
    msg: string;
    meta?: Record<string, unknown>;
  }

  interface LogSession {
    filename: string;
    size_bytes: number;
    entry_count: number;
    first_ts?: string;
    is_current: boolean;
  }

  const MODULES = ['all', 'connectivity', 'recording', 'ai', 'system'] as const;
  const MODULE_COLORS: Record<string, string> = {
    connectivity: 'text-blue-400',
    recording: 'text-purple-400',
    ai: 'text-amber-400',
    system: 'text-muted-foreground',
  };

  let activeTab = $state<'current' | 'history'>('current');
  let selectedModule = $state<string>('all');
  let sessions = $state<LogSession[]>([]);
  let entries = $state<LogEntry[]>([]);
  let selectedSession = $state<string | null>(null);
  let loading = $state(false);

  let filteredEntries = $derived(
    selectedModule === 'all'
      ? entries
      : entries.filter((e) => e.module === selectedModule)
  );

  let currentSession = $derived(sessions.find((s) => s.is_current));

  onMount(async () => {
    await loadSessions();
    await loadCurrentLog();
  });

  async function loadSessions() {
    try {
      sessions = await invoke<LogSession[]>('get_log_sessions');
    } catch (e) {
      console.error('Failed to load log sessions:', e);
    }
  }

  async function loadCurrentLog() {
    loading = true;
    try {
      entries = await invoke<LogEntry[]>('get_log_content', { sessionFile: null });
    } catch (e) {
      console.error('Failed to load log content:', e);
    } finally {
      loading = false;
    }
  }

  async function loadSessionLog(filename: string) {
    loading = true;
    selectedSession = filename;
    try {
      entries = await invoke<LogEntry[]>('get_log_content', { sessionFile: filename });
    } catch (e) {
      console.error('Failed to load session log:', e);
    } finally {
      loading = false;
    }
  }

  async function switchTab(tab: 'current' | 'history') {
    activeTab = tab;
    if (tab === 'current') {
      selectedSession = null;
      await loadCurrentLog();
    } else {
      entries = [];
      selectedSession = null;
    }
  }

  function formatTimestamp(ts: string): string {
    try {
      const d = new Date(ts);
      const pad = (n: number) => n.toString().padStart(2, '0');
      return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
    } catch {
      return ts;
    }
  }

  function formatSessionName(filename: string): string {
    const match = filename.match(/talkshow-(\d{4})-(\d{2})-(\d{2})_(\d{2})-(\d{2})-(\d{2})/);
    if (match) {
      return `${match[1]}-${match[2]}-${match[3]} ${match[4]}:${match[5]}:${match[6]}`;
    }
    return filename;
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    return `${(bytes / 1024).toFixed(1)} KB`;
  }

  function levelColor(level: string): string {
    switch (level) {
      case 'error': return 'text-red-400';
      case 'warn': return 'text-yellow-400';
      default: return 'text-muted-foreground';
    }
  }

  function metaSummary(meta: Record<string, unknown> | undefined): string {
    if (!meta) return '';
    const parts: string[] = [];
    if (meta.provider_id) parts.push(String(meta.provider_id));
    if (meta.model_name) parts.push(String(meta.model_name));
    if (meta.latency_ms != null) parts.push(`${meta.latency_ms}ms`);
    if (meta.error) parts.push(String(meta.error));
    return parts.join(' · ');
  }
</script>

<div class="max-w-[960px]">
  <h2 class="text-xl font-semibold text-foreground m-0 mb-6">日志</h2>

  <div class="flex items-center gap-3 mb-4">
    <div class="flex gap-0.5 bg-muted rounded-md p-0.5">
      <button
        class="px-3 py-1 rounded text-xs transition-colors {activeTab === 'current' ? 'bg-background text-foreground font-medium' : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => switchTab('current')}
      >
        当前会话
      </button>
      <button
        class="px-3 py-1 rounded text-xs transition-colors {activeTab === 'history' ? 'bg-background text-foreground font-medium' : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => switchTab('history')}
      >
        历史记录
      </button>
    </div>

    <div class="flex gap-1">
      {#each MODULES as mod}
        <button
          class="px-2.5 py-1 rounded text-[11px] transition-colors {selectedModule === mod ? 'bg-primary text-primary-foreground' : 'border border-border text-muted-foreground hover:text-foreground'}"
          onclick={() => (selectedModule = mod)}
        >
          {mod === 'all' ? '全部' : mod}
        </button>
      {/each}
    </div>

    {#if currentSession && activeTab === 'current'}
      <span class="ml-auto text-[11px] text-muted-foreground">
        {filteredEntries.length} 条日志
      </span>
    {/if}
  </div>

  {#if activeTab === 'history' && !selectedSession}
    <div class="space-y-1.5">
      {#if sessions.length === 0}
        <div class="text-sm text-muted-foreground py-8 text-center">暂无历史日志</div>
      {:else}
        {#each sessions as session}
          <button
            class="w-full flex items-center gap-4 px-4 py-3 rounded-lg border border-border bg-background-alt text-left transition-colors hover:bg-muted/50"
            onclick={() => loadSessionLog(session.filename)}
          >
            <ScrollText size={16} class="shrink-0 text-muted-foreground" />
            <div class="flex-1 min-w-0">
              <div class="text-sm text-foreground font-medium">
                {formatSessionName(session.filename)}
                {#if session.is_current}
                  <span class="text-[10px] text-green-400 ml-2 font-normal">当前</span>
                {/if}
              </div>
              <div class="text-[11px] text-muted-foreground mt-0.5">
                {session.entry_count} 条 · {formatSize(session.size_bytes)}
              </div>
            </div>
          </button>
        {/each}
      {/if}
    </div>
  {:else}
    {#if loading}
      <div class="text-sm text-muted-foreground py-8 text-center">加载中...</div>
    {:else if filteredEntries.length === 0}
      <div class="text-sm text-muted-foreground py-8 text-center">暂无日志</div>
    {:else}
      <div class="border border-border rounded-lg overflow-hidden">
        <div class="max-h-[calc(100vh-200px)] overflow-y-auto font-mono text-xs">
          {#each filteredEntries as entry, i}
            <div class="flex gap-3 px-4 py-1.5 border-b border-border last:border-b-0 hover:bg-muted/30">
              <span class="text-muted-foreground whitespace-nowrap shrink-0">{formatTimestamp(entry.ts)}</span>
              <span class="{levelColor(entry.level)} shrink-0 w-4 text-center">
                {entry.level === 'error' ? '✕' : entry.level === 'warn' ? '!' : '·'}
              </span>
              <span class="{MODULE_COLORS[entry.module] || 'text-muted-foreground'} shrink-0 w-24">{entry.module}</span>
              <span class="{entry.level === 'error' ? 'text-red-400' : 'text-foreground'} truncate">{entry.msg}</span>
              <span class="text-muted-foreground text-[10px] ml-auto whitespace-nowrap shrink-0">
                {metaSummary(entry.meta as Record<string, unknown> | undefined)}
              </span>
            </div>
          {/each}
        </div>
      </div>

      {#if activeTab === 'history' && selectedSession}
        <button
          class="mt-3 text-xs text-muted-foreground hover:text-foreground transition-colors"
          onclick={() => { selectedSession = null; entries = []; }}
        >
          ← 返回历史列表
        </button>
      {/if}
    {/if}
  {/if}
</div>
```

- [ ] **Step 2: 验证前端编译**

Run: `cd /Users/jia.xia/development/talkshow && npx svelte-check --output human 2>&1 | tail -10`

Expected: 无错误

- [ ] **Step 3: Commit**

```bash
git add src/routes/logs/+page.svelte
git commit -m "feat: add logs page with session switching and module filtering"
```

---

### Task 6: 端到端验证

- [ ] **Step 1: 编译完整应用**

Run: `cd /Users/jia.xia/development/talkshow/src-tauri && cargo build 2>&1 | tail -5`

Expected: 编译成功

- [ ] **Step 2: 验证前端编译**

Run: `cd /Users/jia.xia/development/talkshow && npx svelte-check --output human 2>&1 | tail -10`

Expected: 无错误

- [ ] **Step 3: 手动功能验证清单**

启动应用后验证以下功能：

1. 侧边栏显示 4 个导航项，"日志"可点击
2. 点击"日志"进入 `/logs` 页面，显示当前会话（暂无日志）
3. 前往模型页，执行一次模型连通性测试
4. 返回日志页，查看是否有 connectivity 模块的日志记录
5. 点击模块过滤标签，验证过滤功能
6. 切换到"历史记录" tab，验证会话列表显示
7. 点击历史会话，验证日志内容加载
8. 检查 `{app_data_dir}/logs/` 目录下是否生成了 `.jsonl` 文件

- [ ] **Step 4: 最终 Commit**

```bash
git add -A
git commit -m "feat: complete logging system with backend logger and frontend logs page"
```
