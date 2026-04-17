# 启动健康检查 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 应用启动时自动检查可选外部依赖（当前为 ONNX Runtime），以警告形式通知用户，不阻止启动。

**Architecture:** 新增 `health` 模块，定义 `HealthCheck` trait 和通用检查框架。在 `lib.rs` setup 中执行检查，结果通过 managed state 暴露。前端通过 Tauri command 获取结果，在主布局中以通知条展示。

**Tech Stack:** Rust (Tauri), Svelte 5, TypeScript

---

### Task 1: 提取 `find_onnxruntime_dylib` 为 `pub(crate)`

**Files:**
- Modify: `src-tauri/src/sensevoice/engine.rs:10`

- [ ] **Step 1: 修改 `find_onnxruntime_dylib` 可见性**

将 `src-tauri/src/sensevoice/engine.rs` 第 10 行的 `fn find_onnxruntime_dylib()` 改为 `pub(crate) fn find_onnxruntime_dylib()`：

```rust
pub(crate) fn find_onnxruntime_dylib() -> Option<PathBuf> {
```

- [ ] **Step 2: 验证编译通过**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译成功，无错误

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/sensevoice/engine.rs
git commit -m "refactor: make find_onnxruntime_dylib pub(crate) for health check reuse"
```

---

### Task 2: 创建 health 模块

**Files:**
- Create: `src-tauri/src/health/mod.rs`

- [ ] **Step 1: 创建 health 模块文件**

创建 `src-tauri/src/health/mod.rs`：

```rust
use crate::sensevoice::engine::find_onnxruntime_dylib;

pub enum HealthStatus {
    Ok,
    Warning {
        message: String,
        fix_hint: String,
    },
}

impl serde::Serialize for HealthStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        match self {
            HealthStatus::Ok => {
                map.serialize_entry("status", "ok")?;
            }
            HealthStatus::Warning { message, fix_hint } => {
                map.serialize_entry("status", "warning")?;
                map.serialize_entry("message", message)?;
                map.serialize_entry("fix_hint", fix_hint)?;
            }
        }
        map.end()
    }
}

pub struct HealthCheckResult {
    pub id: String,
    pub name: String,
    pub status: HealthStatus,
}

impl serde::Serialize for HealthCheckResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("id", &self.id)?;
        map.serialize_entry("name", &self.name)?;
        map.serialize_entry("status", &self.status)?;
        map.end()
    }
}

pub struct HealthState {
    pub checks: Vec<HealthCheckResult>,
}

pub trait HealthCheck {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn check(&self) -> HealthStatus;
}

pub struct OnnxRuntimeCheck;

impl HealthCheck for OnnxRuntimeCheck {
    fn id(&self) -> &str {
        "onnx_runtime"
    }

    fn name(&self) -> &str {
        "ONNX Runtime"
    }

    fn check(&self) -> HealthStatus {
        match find_onnxruntime_dylib() {
            Some(_) => HealthStatus::Ok,
            None => HealthStatus::Warning {
                message: "ONNX Runtime 动态库未找到，SenseVoice 本地转写功能将不可用。".to_string(),
                fix_hint: "请通过 brew install onnxruntime 或 pip3 install onnxruntime 安装。".to_string(),
            },
        }
    }
}

pub fn run_health_checks() -> Vec<HealthCheckResult> {
    let checks: Vec<Box<dyn HealthCheck>> = vec![
        Box::new(OnnxRuntimeCheck),
    ];
    checks
        .iter()
        .map(|c| HealthCheckResult {
            id: c.id().to_string(),
            name: c.name().to_string(),
            status: c.check(),
        })
        .collect()
}

#[tauri::command]
pub fn get_health_status(state: tauri::State<HealthState>) -> Vec<HealthCheckResult> {
    state.checks.clone()
}
```

- [ ] **Step 2: 在 lib.rs 中注册 health 模块**

在 `src-tauri/src/lib.rs` 第 17 行（`mod sensevoice;` 之前）添加：

```rust
mod health;
```

- [ ] **Step 3: 在 setup 中执行健康检查并注册 state**

在 `src-tauri/src/lib.rs` 中，找到 `let sensevoice_state = SenseVoiceState {`（约第 547 行），在其之前插入：

```rust
            let health_checks = health::run_health_checks();
            for check in &health_checks {
                if let health::HealthStatus::Warning { message, .. } = &check.status {
                    log::warn!("[health] {} 检查警告: {}", check.name, message);
                }
            }
            let health_state = health::HealthState {
                checks: health_checks,
            };
            app.manage(health_state);
```

- [ ] **Step 4: 注册 Tauri command**

在 `src-tauri/src/lib.rs` 的 `invoke_handler` 宏中（约第 75 行），在 `logger::get_log_content` 之后添加：

```rust
            health::get_health_status,
```

- [ ] **Step 5: 验证编译通过**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译成功

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/health/mod.rs src-tauri/src/lib.rs
git commit -m "feat: add startup health check module with ONNX Runtime check"
```

---

### Task 3: 前端获取并展示健康检查结果

**Files:**
- Create: `src/lib/components/health-banner/index.svelte`
- Modify: `src/routes/+layout.svelte`

- [ ] **Step 1: 创建健康检查通知条组件**

创建 `src/lib/components/health-banner/index.svelte`：

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeWithError } from '$lib/ai/shared';
  import { TriangleAlert, X } from 'lucide-svelte';

  interface HealthCheckResult {
    id: string;
    name: string;
    status: { status: string; message?: string; fix_hint?: string };
  }

  let warnings = $state<HealthCheckResult[]>([]);
  let dismissed = $state(false);

  onMount(async () => {
    const result = await invokeWithError<HealthCheckResult[]>('get_health_status');
    if (result) {
      warnings = result.filter((r) => r.status.status === 'warning');
    }
  });

  function dismiss() {
    dismissed = true;
  }
</script>

{#if warnings.length > 0 && !dismissed}
  <div class="bg-yellow-500/10 border-b border-yellow-500/30 px-6 py-3">
    {#each warnings as warning (warning.id)}
      <div class="flex items-start gap-3">
        <TriangleAlert size={18} class="text-yellow-500 shrink-0 mt-0.5" />
        <div class="flex-1 min-w-0">
          <p class="text-sm text-foreground">{warning.status.message}</p>
          <p class="text-xs text-muted-foreground mt-1">{warning.status.fix_hint}</p>
        </div>
        <button onclick={dismiss} class="text-muted-foreground hover:text-foreground shrink-0">
          <X size={16} />
        </button>
      </div>
    {/each}
  </div>
{/if}
```

- [ ] **Step 2: 在主布局中集成通知条**

修改 `src/routes/+layout.svelte`，在 `{:else}` 分支内（正常布局），在 `<div class="flex h-screen ...">` 外层包裹通知条。

将第 43-91 行：

```svelte
{:else}
<div class="flex h-screen w-screen overflow-hidden">
  <aside class="w-52 bg-background-alt border-r border-border flex flex-col">
    ...
  </aside>
  <main class="flex-1 p-8 overflow-y-auto bg-background">
    {@render children()}
  </main>
</div>
{/if}
```

改为：

```svelte
{:else}
<div class="flex flex-col h-screen w-screen overflow-hidden">
  {@const import HealthBanner from '$lib/components/health-banner/index.svelte'}
  <HealthBanner />
  <div class="flex flex-1 min-h-0">
    <aside class="w-52 bg-background-alt border-r border-border flex flex-col">
      ...
    </aside>
    <main class="flex-1 p-8 overflow-y-auto bg-background">
      {@render children()}
    </main>
  </div>
</div>
{/if}
```

注意：Svelte 不支持 `@const import`。正确做法是在 `<script>` 块中导入。

实际修改方式——在 `<script>` 块（第 9 行 `import OnboardingWizard` 之后）添加：

```typescript
import HealthBanner from '$lib/components/health-banner/index.svelte';
```

然后将第 44 行的 `<div class="flex h-screen w-screen overflow-hidden">` 改为 `<div class="flex flex-col h-screen w-screen overflow-hidden">`，在其下一行添加 `<HealthBanner />`，并将外层 div 改为 `flex flex-col`，内层内容包裹在 `<div class="flex flex-1 min-h-0">` 中：

最终布局结构：

```svelte
{:else}
<div class="flex flex-col h-screen w-screen overflow-hidden">
  <HealthBanner />
  <div class="flex flex-1 min-h-0">
    <aside class="w-52 bg-background-alt border-r border-border flex flex-col">
      ... (保持不变)
    </aside>
    <main class="flex-1 p-8 overflow-y-auto bg-background">
      {@render children()}
    </main>
  </div>
</div>
{/if}
```

- [ ] **Step 3: 验证前端编译通过**

Run: `npm run check` 或 `npx svelte-check --threshold error`
Expected: 无类型错误

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/health-banner/index.svelte src/routes/+layout.svelte
git commit -m "feat: show health check warnings in main layout banner"
```

---

### Task 4: 端到端验证

- [ ] **Step 1: 启动应用验证**

Run: `npm run tauri dev`

验证：
1. 应用正常启动（不被阻止）
2. 如果 ONNX Runtime 未安装，顶部显示黄色警告条，内容包含 "ONNX Runtime 动态库未找到" 和安装提示
3. 点击 X 可以关闭警告条
4. 如果 ONNX Runtime 已安装，不显示警告条
5. 侧边栏和主内容区布局正常

- [ ] **Step 2: 检查日志输出**

在终端日志中搜索 `[health]`，确认健康检查结果被正确记录。

- [ ] **Step 3: Final commit (if any fixups needed)**

```bash
git add -A
git commit -m "fix: address review feedback on health check"
```
