# TalkShow 测试架构设计

日期：2026-04-01

## 背景

TalkShow 当前没有任何测试基础设施（零测试文件、零测试框架、零测试配置）。项目包含 Svelte 5 前端和 Rust 后端，两者通过 Tauri IPC 通信。需要建立完整的测试架构，为项目长期稳定提供保障。

## 目标

- 建立前后端测试基础设施
- 优先覆盖纯逻辑模块，再逐步扩展到集成和组件测试
- 通过 trait 抽象使依赖外部系统的模块可测试
- 测试文件就近放置，维护成本低
- 仅命令行运行，暂不涉及 CI/CD

## 技术选型

| 层 | 工具 | 理由 |
|----|------|------|
| 前端测试运行器 | Vitest | Vite 生态事实标准，与 SvelteKit 无缝集成 |
| 前端组件测试 | @testing-library/svelte | Svelte 官方推荐的组件测试方案 |
| DOM 环境 | jsdom | Vitest 标准搭配 |
| DOM 断言 | @testing-library/jest-dom | 提供 toBeInTheDocument 等语义断言 |
| 后端测试 | Cargo test + #[cfg(test)] | Rust 社区标准 |
| 后端 Mock | mockall | trait 自动 mock，与 Rust trait 体系完美契合 |
| 临时文件 | tempfile | 配置持久化测试需要 |

## 文件放置约定

```
前端：测试文件与源文件同目录
  src/lib/utils/format.ts        → src/lib/utils/format.test.ts
  src/lib/utils/string.ts        → src/lib/utils/string.test.ts
  src/lib/ai/shared/config.ts    → src/lib/ai/shared/config.test.ts
  src/lib/hooks/use-dialog-state.svelte.ts → src/lib/hooks/use-dialog-state.test.ts
  src/lib/components/ui/toggle/  → src/lib/components/ui/toggle/index.test.ts

后端：单元测试在文件底部 #[cfg(test)]，集成测试在 tests/ 目录
  src-tauri/src/config.rs        → 文件内 mod tests { ... }
  src-tauri/src/recording.rs     → 文件内 mod tests { ... }
  src-tauri/tests/               → 集成测试（跨模块交互）
```

## 测试分层

```
        ╱  E2E  ╲           （暂不实施，预留）
       ╱ 组件测试  ╲          （P1）
      ╱  集成测试   ╲         （P1）
     ╱   单元测试    ╲        （P0 — 先从这里开始）
    ─────────────────
```

## P0 — 纯函数单元测试

| 模块 | 测试目标 |
|------|---------|
| `src/lib/utils/format.ts` | `formatTime()`、`formatTimestamp()`、`formatDate()` 边界值 |
| `src/lib/utils/string.ts` | `generateSlug()` 特殊字符、中文、空值 |
| `src/lib/ai/shared/config.ts` | `updateFeature()`、`updateNestedPath()` 深层路径更新 |
| `src-tauri/src/recording.rs` | `days_to_date()`、`is_leap_year()`、`generate_filename()` |
| `src-tauri/src/config.rs` | `migrate_models()`、`migrate_builtin_skills()`、`dedup_models()` |
| `src-tauri/src/sensevoice.rs` | `parse_cmvn()`、`apply_lfr()`、`apply_cmvn()`、`pad_features()` |

## P1 — 组件测试 + 集成测试

**前端组件测试：**
- `toggle`、`select`、`editable-field`、`tag-input` — 交互行为
- `use-dialog-state` hook — 状态机转换

**Rust 集成测试：**
- `ai.rs` — mock LlmClient，测试请求路由分发
- `skills.rs` — mock LlmClient，测试 skill 流水线编排
- `translation.rs` — mock LlmClient，测试超时处理
- `logger.rs` — 路径遍历防护、日志解析
- `lib.rs` — `parse_shortcut()` 快捷键解析

## P2 — 前后端类型一致性

确保 `src/lib/stores/config.ts` 的 `AppConfig` 类型与 `src-tauri/src/config.rs` 的 Rust 结构体字段、默认值保持同步。

## Rust Trait 抽象设计

```rust
// src-tauri/src/ai.rs
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn send_text(&self, prompt: &str, model: &str) -> Result<String>;
    async fn send_audio(&self, audio_path: &str, model: &str) -> Result<String>;
}
```

通过 Tauri managed state 注入，测试中用 mockall 替换：

| 模块 | 需要的改动 |
|------|-----------|
| `ai.rs` | 提取 `LlmClient` trait，现有代码变为 `RealLlmClient` |
| `skills.rs` | 改为接收 `&dyn LlmClient` 参数 |
| `translation.rs` | 同上 |

**原则**：trait 提取只针对需要 mock 的外部依赖，不改变公共 API。纯函数不需要 trait。

## 复杂度评估

| 阶段 | 复杂度 | 预估工作量 | 说明 |
|------|--------|-----------|------|
| 阶段 1：基础设施 + P0 纯函数测试 | 中 | 1-2 天 | 配置简单；config 迁移和 sensevoice 需要构造测试数据 |
| 阶段 2：Trait 抽象 + P1 集成测试 | 高 | 2-3 天 | 需要重构 ai.rs/skills.rs/translation.rs 生产代码 |
| 阶段 3：前端组件测试 | 中 | 1 天 | Svelte 5 Runes 语法需要适配 |

阶段 2 风险最高（生产代码重构），建议作为独立分支处理。

## 实施路线

### 阶段 1 — 基础设施搭建 + P0 纯函数测试
1. 安装前端测试依赖（vitest, @testing-library/svelte, jsdom）
2. 创建 vitest.config.ts
3. 添加 package.json test 脚本
4. 添加 Cargo.toml [dev-dependencies]（mockall, tempfile）
5. 编写 P0 前端测试（format, string, config 工具函数）
6. 编写 P0 Rust 测试（recording 纯函数, config 迁移, sensevoice 音频处理）

### 阶段 2 — Trait 抽象 + P1 集成测试
1. 从 ai.rs 提取 LlmClient trait
2. 重构 skills.rs、translation.rs 接收 trait 参数
3. 编写 ai.rs 集成测试（mock LlmClient）
4. 编写 skills.rs 集成测试（mock LlmClient）
5. 编写 translation.rs 集成测试（mock LlmClient）
6. 编写 logger.rs 测试
7. 编写 lib.rs parse_shortcut() 测试

### 阶段 3 — 前端组件测试
1. 编写 toggle、select、editable-field、tag-input 组件测试
2. 编写 use-dialog-state hook 测试
3. （可选）其余 UI 组件测试

## 运行命令

```bash
# 前端测试
npm run test

# 后端测试
cargo test --manifest-path src-tauri/Cargo.toml
```
