# T1: 配置层扩展 — Skills 数据模型与持久化

## 所属项目
[Skills 文本处理系统](./2026-03-30-skills-system-design.md)

## 依赖
无（基础任务）

## 目标
定义 Skills 的数据模型，实现配置的持久化读写，并确保前后端接口对齐。这是所有后续任务的基础。

## 任务详情

### 1. Rust 后端 — config.rs

在 `FeaturesConfig` 中新增 `SkillsConfig` 字段：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub builtin: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    pub enabled: bool,
    pub skills: Vec<Skill>,
    pub provider_id: String,
    pub model: String,
}
```

修改 `FeaturesConfig`：
```rust
pub struct FeaturesConfig {
    pub transcription: TranscriptionConfig,
    pub skills: SkillsConfig,    // 新增
}
```

实现 `SkillsConfig::default()`：
- `enabled: true`
- `skills`: 包含 4 个预置 Skill（语气词剔除、错别字修正、口语润色、书面格式化）
- `provider_id: ""` (空，待用户配置)
- `model: ""` (空，待用户配置)

实现配置迁移：在 `load_config()` 中检测 `features.skills` 是否存在，不存在则合并默认值。

### 2. Rust 后端 — Tauri Commands

新增以下 Tauri command（在 `lib.rs` 或独立文件中）：

```rust
#[tauri::command]
fn get_skills_config(state: State<AppState>) -> Result<SkillsConfig, String>

#[tauri::command]
fn save_skills_config(state: State<AppState>, config: SkillsConfig) -> Result<(), String>

#[tauri::command]
fn update_skill(state: State<AppState>, skill: Skill) -> Result<(), String>

#[tauri::command]
fn delete_skill(state: State<AppState>, skill_id: String) -> Result<(), String>

#[tauri::command]
fn add_skill(state: State<AppState>, skill: Skill) -> Result<(), String>
```

### 3. 前端 — config.ts

在 `FeaturesConfig` 接口中新增 `skills` 字段，类型定义与 Rust 端对齐。

扩展 `config` store 的 `load()` 和 `save()` 方法以处理 SkillsConfig。

### 4. 配置迁移

当旧版 config.json 不含 `features.skills` 时，自动补全默认值。预置 Skill 的 UUID 应硬编码为固定值，便于后续版本识别和更新。

## 验收标准

- [ ] `FeaturesConfig` 包含 `skills` 字段，编译通过
- [ ] 前后端数据模型对齐，类型一致
- [ ] 首次加载时自动生成包含 4 个预置 Skill 的默认配置
- [ ] 旧版配置升级后 SkillsConfig 自动补全
- [ ] Tauri command 可正确读写 Skills 配置
- [ ] 预置 Skill 的 `builtin` 标记为 `true`，自定义为 `false`
