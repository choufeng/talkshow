# Provider Trait Architecture — 子任务总览

> 源计划: `docs/superpowers/plans/2026-04-06-provider-trait-architecture.md`

## 目标

用独立 Provider trait 实现替换 rig-core/rig-vertexai 依赖，移除 Add Provider UI，硬编码 Provider 端点。

## 子任务依赖图

```
Phase 1 (顺序):
  [A] ProviderConfig 简化 ──┐
  [B] Provider Trait 基础架构 ─┤ (A 先行，B 引用简化后的 ProviderConfig)
                             │
Phase 2 (可并行):             │
  [C] DashScope 实现  ───────┤
  [D] Vertex AI 实现  ───────┤ (依赖 B 的 stub 文件)
  [E] SenseVoice 实现 ───────┤
                             │
Phase 3 (可并行):             │
  [F] 后端集成 ──────────────┤ (依赖 A + B + C + D + E)
  [G] 前端更新 ──────────────┘ (独立于后端，仅修改 TS/Svelte)
                             │
Phase 4:
  [H] 最终集成验证 ───────────┘ (依赖所有)
```

## 子任务列表

| ID | 名称 | 涉及文件 | 依赖 | 预估复杂度 |
|----|------|----------|------|-----------|
| A | ProviderConfig 简化 | `config.rs` | 无 | 中 |
| B | Provider Trait 基础架构 | `providers/mod.rs`, stubs, `lib.rs` | A | 中 |
| C | DashScope Provider 实现 | `providers/dashscope.rs` | B | 高 |
| D | Vertex AI Provider 实现 | `providers/vertex.rs` | B | 高 |
| E | SenseVoice Provider 实现 | `providers/sensevoice.rs` | B | 中 |
| F | 后端集成 (移除 rig) | `Cargo.toml`, `ai.rs`, `lib.rs`, `skills.rs`, `translation.rs`, `llm_client.rs`, `real_llm_client.rs` | A-E | 高 |
| G | 前端更新 | `config.ts`, `+page.svelte`, `ProviderConfigStep.svelte` | 无 (可与后端并行) | 高 |
| H | 最终集成验证 | 所有 | A-G | 低 |

## 并行执行策略

```
时间轴:
t0:  [A]
t1:  [B]
t2:  [C] [D] [E]        ← 三个 Provider 可并行
t3:  [F]      [G]       ← 后端集成与前端并行
t4:  [H]                 ← 最终验证
```

## 文件冲突矩阵

无冲突 — 每个子任务操作不同的文件集合：

| 文件 | A | B | C | D | E | F | G | H |
|------|---|---|---|---|---|---|---|---|
| `config.rs` | ✏️ | | | | | | | |
| `providers/mod.rs` | | ✏️ | | | | | | |
| `providers/dashscope.rs` | | ✏️(stub) | ✏️ | | | | | |
| `providers/vertex.rs` | | ✏️(stub) | | ✏️ | | | | |
| `providers/sensevoice.rs` | | ✏️(stub) | | | ✏️ | | | |
| `lib.rs` | | ✏️(mod) | | | | ✏️ | | |
| `Cargo.toml` | | | | | | ✏️ | | |
| `ai.rs` | | | | | | ✏️ | | |
| `skills.rs` | | | | | | ✏️ | | |
| `translation.rs` | | | | | | ✏️ | | |
| `llm_client.rs` | | | | | | ✏️ | | |
| `real_llm_client.rs` | | | | | | ✏️ | | |
| `config.ts` | | | | | | | ✏️ | |
| `+page.svelte` | | | | | | | ✏️ | |
| `ProviderConfigStep.svelte` | | | | | | | ✏️ | |

## 与原计划任务映射

| 原任务 | 子任务 |
|--------|--------|
| Task 1 (Provider Trait + stubs) | B |
| Task 2 (DashScope) | C |
| Task 3 (Vertex AI) | D |
| Task 4 (SenseVoice) | E |
| Task 5 (Config 简化) | A |
| Task 6 (Remove rig) | F |
| Task 7 (Rewrite ai.rs) | F |
| Task 8 (Update lib.rs) | F |
| Task 9 (skills/translation/llm_client) | F |
| Task 10 (Frontend types) | G |
| Task 11 (Models page) | G |
| Task 12 (Onboarding) | G |
| Task 13 (Final verification) | H |
