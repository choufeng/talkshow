# 设计文档：Skills 与润色配置优化

**日期**: 2026-03-31
**状态**: 待实现
**相关文件**: 
- `src/routes/models/+page.svelte`
- `src/routes/skills/+page.svelte`
- `src-tauri/src/skills.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/config.rs`
- `src/lib/stores/config.ts`

---

## 背景

当前系统中存在两个配置问题：

1. **润色模型选择无关联**：当"启用润色"开关关闭时，润色模型选择框仍然可见，用户可能困惑于为何选择了模型却不生效。

2. **Skills 全局开关冗余**：Skills 功能的启用/禁用开关位于技能配置页，但实际的 Skills 管线执行逻辑依赖于润色功能的配置。这造成了概念上的混淆和维护上的复杂性。

## 目标

1. 改善润色配置的用户体验，使 UI 状态与功能状态一致
2. 简化 Skills 配置界面，移除冗余的全局开关
3. 调整后端逻辑，确保变更后的配置流正确工作

## 变更范围

### 前端变更

#### 1. 模型配置页 (`models/+page.svelte`)

**变更**：润色模型选择框添加条件渲染

- 使用 `{#if $config.features.transcription.polish_enabled}` 包裹润色模型选择部分
- 当"启用润色"关闭时，润色模型选择框完全隐藏
- 保留"启用润色"开关本身

**预期效果**：
```
启用润色: [开]
(润色模型选择框显示)

启用润色: [关]
(润色模型选择框隐藏)
```

#### 2. 技能配置页 (`skills/+page.svelte`)

**变更**：移除全局开关区块

- 移除第 123-141 行的"全局"开关区块（包含"Skills 功能"标题和 toggle 按钮）
- 移除 `handleGlobalToggle` 函数
- 保留所有其他功能：
  - Skill 列表展示
  - 单个 Skill 的启用/禁用开关
  - 添加/编辑/删除 Skill 功能
  - 编辑对话框

**预期效果**：
```
技能设置
├── 技能列表
│   ├── [开关] 语气词剔除 [预置] [编辑] [删除]
│   ├── [开关] 错别字修正 [预置] [编辑] [删除]
│   └── ...
└── [+ 添加自定义 Skill]
```

### 后端变更

#### 1. Skills 处理管线 (`skills.rs`)

**变更**：移除 `polish_enabled` 检查

- 删除第 100-103 行的 `polish_enabled` 检查逻辑
- 保留其他检查：
  - `skills_config.enabled` 检查
  - 启用的 Skill 列表检查
  - `polish_provider_id` 和 `polish_model` 配置检查

**变更前后对比**：

变更前：
```rust
if !transcription_config.polish_enabled {
    logger.info("skills", "润色功能未启用，跳过处理", None);
    return Ok(transcription.to_string());
}
```

变更后：
```rust
// 此检查已移除，Skills 管线不再依赖 polish_enabled
```

#### 2. 配置保存 (`lib.rs`)

**变更**：强制 `skills_config.enabled = true`

在 `save_skills_config` 命令中，确保 `enabled` 字段始终为 `true`：

```rust
#[tauri::command]
fn save_skills_config(app_handle: tauri::AppHandle, mut skills_config: config::SkillsConfig) -> Result<(), String> {
    skills_config.enabled = true; // 强制启用
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);
    app_config.features.skills = skills_config;
    config::save_config(&app_data_dir, &app_config)
}
```

### 数据流变更

**变更前的 Skills 管线检查流程**：
```
1. skills_config.enabled? → false → 返回原始文本
2. 有启用的 Skill? → 无 → 返回原始文本
3. polish_enabled? → false → 返回原始文本  ← 移除此检查
4. polish_provider/model 配置? → 无 → 返回原始文本
5. 调用 LLM 处理
```

**变更后的 Skills 管线检查流程**：
```
1. skills_config.enabled? → false → 返回原始文本 (始终为 true)
2. 有启用的 Skill? → 无 → 返回原始文本
3. polish_provider/model 配置? → 无 → 返回原始文本
4. 调用 LLM 处理
```

## 影响分析

| 变更项 | 影响范围 | 风险等级 | 缓解措施 |
|--------|---------|---------|---------|
| 润色模型条件渲染 | UI 层 | 低 | 仅影响显示逻辑，不改变数据流 |
| 移除 Skills 全局开关 | UI + 配置保存 | 低 | 后端强制 enabled=true，保持兼容性 |
| 移除 polish_enabled 检查 | Skills 管线逻辑 | 中 | 保留 polish_provider/model 检查作为安全网 |

## 向后兼容性

- 现有用户的 `skills_config.enabled` 配置值将被忽略（后端强制为 true）
- 现有用户的 `polish_enabled` 配置值仅影响 UI 显示，不影响 Skills 管线
- 现有用户的 Skill 列表和单个 Skill 的启用状态保持不变

## 测试策略

### 前端测试
1. 切换"启用润色"开关，验证润色模型选择框的显示/隐藏
2. 验证技能配置页不再显示全局开关
3. 验证单个 Skill 的开关仍然正常工作

### 后端测试
1. 验证 Skills 管线在 `polish_enabled=false` 时仍能正常执行
2. 验证 Skills 管线在 `polish_provider_id` 或 `polish_model` 为空时正确回退
3. 验证配置保存后 `skills_config.enabled` 始终为 true

## 实施顺序

1. 前端：添加润色模型条件渲染
2. 前端：移除 Skills 全局开关
3. 后端：移除 polish_enabled 检查
4. 后端：添加强制 enabled=true 逻辑
5. 测试验证

## 风险与注意事项

1. **Skills 管线执行条件变化**：移除 `polish_enabled` 检查后，Skills 管线将仅依赖 `polish_provider_id` 和 `polish_model` 配置。需确保用户在使用 Skills 前已正确配置润色模型。

2. **用户迁移**：现有用户如果关闭了 Skills 全局开关，更新后该设置将被忽略。可能需要在更新日志中说明此变更。

3. **配置一致性**：前端不再发送 `skills.enabled` 字段，后端需要处理这种情况（通过强制设为 true）。
