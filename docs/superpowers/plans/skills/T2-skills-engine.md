# T2: Skills 核心引擎 — Prompt 组装与 LLM 调用

## 所属项目
[Skills 文本处理系统](../../specs/2026-03-30-skills-system-design.md)

## 依赖
- T1: 配置层扩展（需要 SkillsConfig 数据模型）

## 目标
实现 Skills 处理的核心逻辑：获取前台应用信息、组装合并 Prompt、调用 LLM 并返回处理后的文本。

## 任务详情

### 1. 新建 skills.rs

创建 `src-tauri/src/skills.rs`，包含以下核心函数：

#### 获取前台应用信息

```rust
fn get_frontmost_app() -> Result<(String, String), String>
// 返回 (app_name, bundle_id)
// macOS: 通过 NSWorkspace API 获取
//   - NSWorkspace.shared.frontmostApplication
//   - 获取 localizedName 和 bundleIdentifier
// 其他平台: 返回 ("Unknown", "unknown")
```

#### Prompt 组装

```rust
fn assemble_skills_prompt(
    skills: &[Skill],
    transcription: &str,
    app_name: &str,
    bundle_id: &str,
) -> (String, String)
// 返回 (system_prompt, user_message)
//
// system_prompt 结构：
//   1. 角色定义 + 输出格式约束
//   2. 当前应用上下文 (app_name + bundle_id)
//   3. 各 Skill 指令（遍历所有 enabled 的 Skill，逐条嵌入）
//
// user_message：
//   转写文字原文
```

合并 Prompt 的具体模板：

```
[system prompt]
你是一个语音转文字的文本处理助手。请根据以下规则处理用户的输入文本。

当前用户正在使用的应用是：{app_name} ({bundle_id})
请仅应用与当前场景相关的规则，跳过不适用的规则。

---
【{skill_1.name}】
{skill_1.prompt}
---
【{skill_2.name}】
{skill_2.prompt}
---
...

请只输出处理后的纯文本。不要添加任何解释、标注或前缀。

[user message]
{transcription_text}
```

#### Skills 处理主函数

```rust
pub async fn process_with_skills(
    ai_state: &AiState,
    skills_config: &SkillsConfig,
    providers: &[ProviderConfig],
    transcription: &str,
) -> Result<String, String>
// 流程：
// 1. 检查是否有 enabled 的 Skill，没有则直接返回原文
// 2. 获取前台应用信息 (get_frontmost_app)
// 3. 过滤出 enabled 的 Skill 列表
// 4. 组装 prompt (assemble_skills_prompt)
// 5. 根据 skills_config.provider_id 找到对应 Provider
// 6. 调用 ai::send_text_prompt (复用现有能力)
// 7. 返回 LLM 输出的处理后文本
// 错误时：记录日志，返回原始 transcription（降级策略）
```

### 2. 复用 ai.rs

`ai.rs` 中现有的 `send_text_prompt` 已支持 Vertex AI 和 OpenAI-Compatible 的纯文本调用。Skills 引擎直接复用此函数，传入：
- `provider`: 从 `skills_config.provider_id` 查找对应 ProviderConfig
- `model`: `skills_config.model`
- `system_prompt`: 合并后的 Skill 指令
- `user_message`: 转写文字

如需为 Skills 调用添加超时控制（建议 10 秒），可在 `process_with_skills` 中使用 `tokio::time::timeout` 包裹调用。

### 3. 日志集成

在 `logger.rs` 的模块枚举中新增 `skills` 变体，用于记录：
- Skills 处理开始（含启用的 Skill 列表、当前应用）
- LLM 调用成功（含处理前后文字对比摘要）
- LLM 调用失败（含错误信息、回退到原始文字）

## 验收标准

- [ ] `skills.rs` 编译通过，无外部依赖问题
- [ ] `get_frontmost_app()` 在 macOS 上能正确返回前台应用信息
- [ ] `assemble_skills_prompt()` 生成的 prompt 符合设计规范
- [ ] 多个 Skill 的指令能正确合并到单个 system prompt 中
- [ ] LLM 调用成功时返回处理后的文本
- [ ] LLM 调用失败时回退返回原始转写文字，不 panic
- [ ] 超时控制正常工作（默认 10 秒）
- [ ] 日志正确记录 Skills 处理过程
