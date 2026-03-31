# 模型选择迁移实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将模型选择功能从技能设置页面迁移至模型管理页面，新增润色服务和启用开关。

**Architecture:** 方案 A（最小改动）。扩展 `TranscriptionConfig` 新增 polish 字段，简化 `SkillsConfig` 移除模型字段，更新后端调用链使用新配置。

**Tech Stack:** Svelte 5 + SvelteKit (前端), Tauri 2 + Rust (后端)

---

## 文件结构

| 文件 | 变更类型 | 职责 |
|------|---------|------|
| `src/lib/stores/config.ts` | 修改 | 前端类型定义和默认值 |
| `src/routes/models/+page.svelte` | 修改 | 添加润色模型选择和启用开关 |
| `src/routes/skills/+page.svelte` | 修改 | 移除 LLM 服务配置区域 |
| `src-tauri/src/config.rs` | 修改 | 后端类型定义和默认值 |
| `src-tauri/src/skills.rs` | 修改 | 函数签名和内部逻辑 |
| `src-tauri/src/lib.rs` | 修改 | 调用链更新 |

---

### Task 1: 后端数据结构变更 (`config.rs`)

**Files:**
- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: 扩展 `TranscriptionConfig` 结构体**

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct TranscriptionConfig {
    pub provider_id: String,
    pub model: String,
    pub polish_enabled: bool,
    pub polish_provider_id: String,
    pub polish_model: String,
}
```

- [ ] **Step 2: 简化 `SkillsConfig` 结构体**

移除 `provider_id` 和 `model` 字段：

```rust
#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SkillsConfig {
    pub enabled: bool,
    pub skills: Vec<Skill>,
}
```

- [ ] **Step 3: 更新 `SkillsConfig::default()` 实现**

移除 `provider_id` 和 `model` 的初始化：

```rust
impl Default for SkillsConfig {
    fn default() -> Self {
        SkillsConfig {
            enabled: true,
            skills: vec![
                Skill {
                    id: "builtin-fillers".to_string(),
                    name: "语气词剔除".to_string(),
                    description: "去除嗯、啊、那个、就是等无意义口头语气词".to_string(),
                    prompt: "去除中文口语中常见的无意义语气词和填充词，包括但不限于：\n\"嗯\"、\"啊\"、\"额\"、\"呃\"、\"那个\"、\"就是\"、\"然后\"、\"对吧\"、\"的话\"、\"怎么说呢\"。\n注意保留有实际语义的词语，例如\"然后\"在表示时间顺序时应保留。不要改变原文的语义和语气。".to_string(),
                    builtin: true,
                    enabled: true,
                },
                Skill {
                    id: "builtin-typos".to_string(),
                    name: "错别字修正".to_string(),
                    description: "修正错别字、同音错误和输入法错误".to_string(),
                    prompt: "识别并修正文本中的错别字、同音错误和常见输入法导致的文字错误。\n只修正明确的错误，不要对有歧义的内容做主观改动。\n常见的同音错误示例：\"的/地/得\"、\"做/作\"、\"在/再\"、\"已/以\"、\"即/既\"。".to_string(),
                    builtin: true,
                    enabled: true,
                },
                Skill {
                    id: "builtin-polish".to_string(),
                    name: "口语润色".to_string(),
                    description: "保持口语化风格，使表达更流畅自然".to_string(),
                    prompt: "保持口语化的表达风格，但使语句更流畅自然。\n具体做法：去除重复表达、调整语序使其更通顺、适当添加标点使句子结构更清晰。\n不要改变原文的口语化特征，不要转换为书面语。".to_string(),
                    builtin: true,
                    enabled: false,
                },
                Skill {
                    id: "builtin-formal".to_string(),
                    name: "书面格式化".to_string(),
                    description: "口语转书面表达，适合邮件和文档场景".to_string(),
                    prompt: "将口语化的表达转换为规范的书面表达，适合邮件、文档、报告等正式场景。\n具体做法：\n- 将口语化的词汇替换为正式表达\n- 调整句子结构使其符合书面语法\n- 适当分段和添加标点\n- 保持原文的完整语义，不添加或删除信息".to_string(),
                    builtin: true,
                    enabled: false,
                },
            ],
        }
    }
}
```

- [ ] **Step 4: 更新 `AppConfig::default()` 中的 `TranscriptionConfig` 默认值**

```rust
features: FeaturesConfig {
    transcription: TranscriptionConfig {
        provider_id: "vertex".to_string(),
        model: "gemini-2.0-flash".to_string(),
        polish_enabled: true,
        polish_provider_id: String::new(),
        polish_model: String::new(),
    },
    skills: SkillsConfig::default(),
},
```

- [ ] **Step 5: 编译验证**

```bash
cd src-tauri && cargo check
```
Expected: 编译通过（可能有未使用变量警告，后续任务会修复）

---

### Task 2: 后端 Skills 处理逻辑变更 (`skills.rs`)

**Files:**
- Modify: `src-tauri/src/skills.rs`

- [ ] **Step 1: 更新 import，引入 `TranscriptionConfig`**

```rust
use crate::config::{ProviderConfig, Skill, SkillsConfig, TranscriptionConfig};
```

- [ ] **Step 2: 更新 `process_with_skills` 函数签名**

```rust
pub async fn process_with_skills(
    logger: &Logger,
    transcription_config: &TranscriptionConfig,
    skills_config: &SkillsConfig,
    providers: &[ProviderConfig],
    transcription: &str,
    vertex_cache: &VertexClientCache,
) -> Result<String, String> {
```

- [ ] **Step 3: 更新启用检查逻辑**

替换 `skills_config.enabled` 检查为同时检查 skills 和 polish：

```rust
if !skills_config.enabled || !transcription_config.polish_enabled {
    return Ok(transcription.to_string());
}
```

- [ ] **Step 4: 更新 provider/model 检查逻辑**

替换第 99 行的检查：

```rust
if transcription_config.polish_provider_id.is_empty() || transcription_config.polish_model.is_empty() {
    logger.warn("skills", "润色模型未配置，跳过处理", None);
    return Ok(transcription.to_string());
}
```

- [ ] **Step 5: 更新 provider 查找逻辑**

替换第 132-146 行的 provider 查找：

```rust
let provider = match providers
    .iter()
    .find(|p| p.id == transcription_config.polish_provider_id)
{
    Some(p) => p,
    None => {
        logger.warn(
            "skills",
            "未找到润色 Provider，回退原始文字",
            Some(serde_json::json!({
                "provider_id": transcription_config.polish_provider_id,
            })),
        );
        return Ok(transcription.to_string());
    }
};
```

- [ ] **Step 6: 更新 LLM 调用逻辑**

替换第 156 行的模型调用：

```rust
let result = tokio::time::timeout(
    std::time::Duration::from_secs(timeout_secs),
    crate::ai::send_text_prompt(logger, &full_prompt, &transcription_config.polish_model, provider, vertex_cache),
)
.await;
```

- [ ] **Step 7: 编译验证**

```bash
cd src-tauri && cargo check
```
Expected: 编译通过（可能有警告，因为调用方签名未更新）

---

### Task 3: 后端主流程调用变更 (`lib.rs`)

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 更新 `process_with_skills` 调用**

找到第 238-244 行的调用，更新为传入 `transcription`：

```rust
let final_text = skills::process_with_skills(
    &logger,
    transcription,
    &skills_config,
    &skills_providers,
    &text,
    &h.state::<VertexClientState>().client,
)
.await
.unwrap_or_else(|e| {
    logger.error("skills", &format!("Skills 处理异常，使用原始文字: {}", e), None);
    text
});
```

- [ ] **Step 2: 编译验证**

```bash
cd src-tauri && cargo check
```
Expected: 编译通过，无错误

---

### Task 4: 前端数据结构变更 (`config.ts`)

**Files:**
- Modify: `src/lib/stores/config.ts`

- [ ] **Step 1: 扩展 `TranscriptionConfig` 接口**

```typescript
export interface TranscriptionConfig {
  provider_id: string;
  model: string;
  polish_enabled: boolean;
  polish_provider_id: string;
  polish_model: string;
}
```

- [ ] **Step 2: 简化 `SkillsConfig` 接口**

```typescript
export interface SkillsConfig {
  enabled: boolean;
  skills: Skill[];
}
```

- [ ] **Step 3: 更新 store 默认值**

找到 `createConfigStore()` 中的默认值，更新为：

```typescript
features: {
  transcription: {
    provider_id: 'vertex',
    model: 'gemini-2.0-flash',
    polish_enabled: true,
    polish_provider_id: '',
    polish_model: ''
  },
  skills: {
    enabled: true,
    skills: []
  }
}
```

---

### Task 5: 模型管理页面 UI 变更 (`models/+page.svelte`)

**Files:**
- Modify: `src/routes/models/+page.svelte`

- [ ] **Step 1: 更新 Features 区域标题和布局**

将第 371-385 行的 Features 区域改为"转写服务"，使用纵向布局：

```svelte
<section class="mb-10">
  <div class="text-xs text-muted-foreground uppercase tracking-wider mb-3">转写服务</div>
  <div class="rounded-xl border border-border bg-background-alt p-5">
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
          class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-2 {$config.features.transcription.polish_enabled ? 'bg-accent-foreground' : 'bg-border'}"
          role="switch"
          aria-checked={$config.features.transcription.polish_enabled}
          onclick={() => handlePolishEnabled(!$config.features.transcription.polish_enabled)}
        >
          <span class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {$config.features.transcription.polish_enabled ? 'translate-x-5' : 'translate-x-0'}"></span>
        </button>
      </div>

      <div>
        <label class="block text-sm text-foreground-alt mb-1.5">润色模型</label>
        <GroupedSelect
          value={getPolishValue()}
          groups={buildTranscriptionGroups()}
          placeholder="选择模型"
          onChange={handlePolishChange}
        />
      </div>
    </div>
  </div>
</section>
```

- [ ] **Step 2: 添加润色模型相关函数**

在 script 区域添加以下函数（放在 `handleTranscriptionChange` 函数之后）：

```typescript
function getPolishValue(): string {
  const t = $config.features.transcription;
  if (t.polish_provider_id && t.polish_model) {
    return `${t.polish_provider_id}::${t.polish_model}`;
  }
  return '';
}

function handlePolishChange(val: string) {
  const [providerId, model] = val.split('::');
  const newConfig: AppConfig = {
    ...$config,
    features: {
      ...$config.features,
      transcription: {
        ...$config.features.transcription,
        polish_provider_id: providerId,
        polish_model: model
      }
    }
  };
  config.save(newConfig);
}

function handlePolishEnabled(enabled: boolean) {
  const newConfig: AppConfig = {
    ...$config,
    features: {
      ...$config.features,
      transcription: {
        ...$config.features.transcription,
        polish_enabled: enabled
      }
    }
  };
  config.save(newConfig);
}
```

- [ ] **Step 3: 前端类型检查**

```bash
npx svelte-check
```
Expected: 无错误

---

### Task 6: 技能设置页面清理 (`skills/+page.svelte`)

**Files:**
- Modify: `src/routes/skills/+page.svelte`

- [ ] **Step 1: 移除 "LLM 服务" section**

删除第 190-214 行的整个 section：

```svelte
<!-- 删除以下内容 -->
<section class="mb-10">
  <div class="text-xs text-muted-foreground uppercase tracking-wider mb-3">LLM 服务</div>
  <div class="rounded-xl border border-border bg-background-alt p-5">
    <div class="grid grid-cols-2 gap-4">
      <div>
        <label class="block text-sm text-foreground-alt mb-1.5">Provider</label>
        <GroupedSelect
          value={$config.features.skills.provider_id}
          groups={buildProviderGroups()}
          placeholder="选择 Provider"
          onChange={handleProviderChange}
        />
      </div>
      <div>
        <label class="block text-sm text-foreground-alt mb-1.5">Model</label>
        <GroupedSelect
          value={$config.features.skills.model}
          groups={buildModelGroups()}
          placeholder="选择模型"
          onChange={handleModelChange}
        />
      </div>
    </div>
  </div>
</section>
```

- [ ] **Step 2: 移除相关函数**

删除以下函数：
- `buildProviderGroups()` (第 46-53 行)
- `buildModelGroups()` (第 55-66 行)
- `handleProviderChange()` (第 68-79 行)
- `handleModelChange()` (第 81-90 行)

- [ ] **Step 3: 清理未使用的 import**

移除不再需要的 `ProviderConfig` 和 `ModelConfig` import：

```typescript
import type { Skill, AppConfig } from '$lib/stores/config';
```

- [ ] **Step 4: 前端类型检查**

```bash
npx svelte-check
```
Expected: 无错误

---

### Task 7: 数据迁移逻辑

**Files:**
- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: 在 `load_config` 中添加迁移逻辑**

在 `load_config` 函数中，反序列化后添加迁移逻辑，将旧的 `skills.provider_id/model` 迁移到 `transcription.polish_provider_id/model`：

```rust
pub fn load_config(app_data_dir: &PathBuf) -> AppConfig {
    let path = config_file_path(app_data_dir);
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => {
                let mut raw: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
                migrate_models(&mut raw);

                // 数据迁移：将 skills.provider_id/model 迁移到 transcription.polish_*
                if let Some(features) = raw.get_mut("features") {
                    if let Some(skills) = features.get_mut("skills") {
                        if let Some(provider_id) = skills.get("provider_id").and_then(|v| v.as_str()) {
                            if !provider_id.is_empty() {
                                if let Some(transcription) = features.get_mut("transcription") {
                                    if let Some(polish) = transcription.get_mut("polish_provider_id") {
                                        *polish = serde_json::json!(provider_id);
                                    }
                                    if let Some(polish) = transcription.get_mut("polish_model") {
                                        if let Some(model) = skills.get("model").and_then(|v| v.as_str()) {
                                            *polish = serde_json::json!(model);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let mut config: AppConfig = serde_json::from_value(raw).unwrap_or_default();
                config.ai.providers = merge_builtin_providers(config.ai.providers);
                for provider in &mut config.ai.providers {
                    dedup_models(&mut provider.models);
                }
                config
            }
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
```

- [ ] **Step 2: 编译验证**

```bash
cd src-tauri && cargo check
```
Expected: 编译通过，无错误

---

### Task 8: 后端 Tauri 命令更新 (`lib.rs`)

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 更新 `save_skills_config` 命令**

由于 `SkillsConfig` 不再有 `provider_id` 和 `model` 字段，前端不再调用此命令保存模型配置。但此命令仍用于保存 Skills 开关和技能列表，保持不变。

- [ ] **Step 2: 添加新的 Tauri 命令用于保存润色配置**

在 `save_skills_config` 函数后添加：

```rust
#[tauri::command]
fn save_transcription_config(
    app_handle: tauri::AppHandle,
    transcription: config::TranscriptionConfig,
) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);
    app_config.features.transcription = transcription;
    config::save_config(&app_data_dir, &app_config)
}
```

- [ ] **Step 3: 注册新的 Tauri 命令**

在 `invoke_handler` 中添加 `save_transcription_config`：

```rust
.invoke_handler(tauri::generate_handler![
    get_config,
    update_shortcut,
    save_config_cmd,
    test_model_connectivity,
    get_vertex_env_info,
    get_skills_config,
    save_skills_config,
    save_transcription_config,
    add_skill,
    update_skill,
    delete_skill,
    sensevoice::get_sensevoice_status,
    sensevoice::download_sensevoice_model,
    sensevoice::delete_sensevoice_model,
    logger::get_log_sessions,
    logger::get_log_content
])
```

- [ ] **Step 4: 编译验证**

```bash
cd src-tauri && cargo check
```
Expected: 编译通过，无错误

---

### Task 9: 前端保存逻辑更新

**Files:**
- Modify: `src/routes/models/+page.svelte`

- [ ] **Step 1: 更新 `handleTranscriptionChange` 使用完整配置保存**

当前的 `handleTranscriptionChange` 只保存部分字段，需要确保保存完整的 `TranscriptionConfig`：

```typescript
function handleTranscriptionChange(val: string) {
  const [providerId, model] = val.split('::');
  const newConfig: AppConfig = {
    ...$config,
    features: {
      ...$config.features,
      transcription: {
        ...$config.features.transcription,
        provider_id: providerId,
        model
      }
    }
  };
  config.save(newConfig);
}
```

注意：由于前端已经有 `config.save(newConfig)` 保存完整配置，无需额外修改。

---

### Task 10: 最终验证

- [ ] **Step 1: 后端完整编译**

```bash
cd src-tauri && cargo build
```
Expected: 编译通过，无错误

- [ ] **Step 2: 前端类型检查**

```bash
npx svelte-check
```
Expected: 无错误

- [ ] **Step 3: 提交所有变更**

```bash
git add -A
git commit -m "feat: 迁移模型选择至模型管理页面，新增润色服务配置"
```
