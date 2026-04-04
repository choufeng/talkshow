# 集成测试完善设计

日期：2026-04-05

## 背景

TalkShow 已有测试基础设施（Vitest + cargo test），Phase 1（单元测试）和部分 Phase 2（Trait 抽象）已完成。但集成测试覆盖不足，缺少：
- Rust 跨模块集成测试（`src-tauri/tests/` 目录为空）
- 前端多组件协同场景测试
- Tauri IPC 命令契约验证
- CI/CD 自动化验证

## 目标

- 构建完整的三层集成测试架构（Rust 模块集成 + IPC 契约 + 前端组件集成）
- 配置 GitHub Actions CI，所有分支 push 和 PR 均自动运行
- 所有集成测试必须通过才能合并代码

## 技术选型

| 层级 | 工具 | 理由 |
|------|------|------|
| Rust 集成测试 | cargo test + mockall | 与 Phase 2 已有的 LlmClient trait 体系一致 |
| IPC 契约测试 | cargo test | 直接测试 Tauri 命令处理函数，无需启动完整应用 |
| 前端组件集成 | Vitest + @testing-library/svelte + jsdom | 复用现有测试基础设施 |
| CI/CD | GitHub Actions | 项目已在使用 |

## 架构设计

```
                    集成测试架构
    ┌─────────────────────────────────────────┐
    │          前端组件集成测试 (Vitest)        │
    │  设置页面、录音控制、AI 结果展示           │
    ├─────────────────────────────────────────┤
    │        IPC 契约测试 (cargo test)         │
    │  核心 Tauri 命令输入输出验证               │
    ├─────────────────────────────────────────┤
    │     Rust 模块集成测试 (cargo test)       │
    │  AI 路由、Skills 流水线、翻译、日志、快捷键 │
    └─────────────────────────────────────────┘
```

### 三层职责

| 层级 | 测试什么 | 不测试什么 |
|------|---------|-----------|
| Rust 模块集成 | 跨模块业务逻辑（LLM 调用路由、Skills 编排、翻译超时处理） | 真实网络请求、文件系统副作用 |
| IPC 契约 | Tauri 命令的参数校验、返回值格式、错误码 | UI 渲染、前端状态管理 |
| 前端组件集成 | 多组件交互（表单提交 → 对话框 → 状态更新） | 真实后端响应、网络延迟 |

## Rust 模块集成测试

### 目录结构

```
src-tauri/
├── src/
│   ├── ai.rs              # 已有单元测试
│   ├── skills.rs          # 已有测试模块
│   ├── translation.rs     # 已有测试模块
│   ├── logger.rs          # 已有测试模块
│   ├── lib.rs             # 已有 parse_shortcut 测试
│   ├── llm_client.rs      # LlmClient trait + mockall
│   └── real_llm_client.rs # 真实 LLM 客户端
└── tests/                 # 新增：集成测试目录
    ├── ai_routing.rs      # AI 请求路由分发
    ├── skills_pipeline.rs # Skills 流水线编排
    ├── translation_flow.rs# 翻译完整流程
    └── common/
        └── mod.rs         # 测试辅助函数
```

### 测试覆盖场景

**`ai_routing.rs`**
- 文本请求正确路由到对应 provider
- 音频请求正确路由到对应 provider
- 无效 provider 返回错误
- Vertex AI 缓存复用逻辑

**`skills_pipeline.rs`**
- 空转录跳过处理
- 无启用 skill 返回原文
- 单个 skill 正常处理
- 多个 skill 串行处理
- LLM 超时回退原文
- LLM 错误回退原文

**`translation_flow.rs`**
- 正常翻译返回结果
- 带 translation skill 的翻译
- 超时返回错误
- LLM 错误传播

### Mock 策略

```rust
use crate::llm_client::MockLlmClient;

// 测试中按需配置行为
mock.expect_send_text()
    .times(1)
    .returning(|_, _, _, _| Ok("mocked response".to_string()));
```

**不 mock 的组件：**
- `Logger` — 使用 `tempfile` 创建临时目录
- `Config` — 使用测试配置结构体
- 纯函数（`parse_shortcut`、`generate_filename` 等）

## IPC 契约测试

### 测试策略

不启动完整 Tauri 应用，直接测试 Tauri 命令处理函数：

```rust
use talkshow::*;

#[tokio::test]
async fn test_ai_text_command_valid_input() {
    let app = create_test_app();
    let result = ai_text_command(&app, AiTextRequest {
        prompt: "Hello".to_string(),
        model: "gpt-4".to_string(),
        provider_id: "openai".to_string(),
    }).await;
    
    assert!(result.is_ok());
}
```

### 覆盖的核心命令

| 命令 | 测试点 |
|------|--------|
| `ai:text` | 有效输入、空 prompt、无效 provider、LLM 错误 |
| `ai:audio` | 有效音频路径、文件不存在、转写错误 |
| `recording:start` | 正常启动、重复启动、权限拒绝 |
| `recording:stop` | 正常停止、未录音时停止 |
| `settings:save` | 有效配置、缺少必填字段、序列化错误 |
| `settings:load` | 配置文件存在、配置文件不存在、迁移逻辑 |

### 测试辅助函数

```rust
// tests/common/mod.rs
pub fn create_test_app() -> AppHandle {
    let temp_dir = tempfile::tempdir().unwrap();
    // ... 初始化配置、日志等
}

pub fn mock_llm_response(response: Result<String, String>) -> MockLlmClient {
    let mut mock = MockLlmClient::new();
    mock.expect_send_text()
        .returning(move |_, _, _, _| response.clone());
    mock
}
```

## 前端组件集成测试

### 测试场景

| 测试文件 | 场景 |
|---------|------|
| `settings-page.integration.test.ts` | 设置页面：修改 API Key → 保存 → 验证状态更新 |
| `recording-flow.integration.test.ts` | 录音控制：点击录音 → 状态变更 → 指示器显示 |
| `ai-result-display.integration.test.ts` | AI 结果：接收转录文本 → Skills 处理 → 结果展示 |
| `provider-config.integration.test.ts` | Provider 配置：添加新 Provider → 表单验证 → 保存 |

### 测试模式

```typescript
describe('Settings Page Integration', () => {
  it('should save API key and update store', async () => {
    const { getByLabelText, getByText } = render(SettingsPage);
    
    const apiKeyInput = getByLabelText('API Key');
    await fireEvent.input(apiKeyInput, { target: { value: 'sk-test-123' } });
    
    const saveButton = getByText('保存');
    await fireEvent.click(saveButton);
    
    expect(get(config).providers[0].apiKey).toBe('sk-test-123');
  });
});
```

### Mock 策略

- **Tauri IPC 调用**：mock `@tauri-apps/api/core` 的 `invoke`
- **LLM 响应**：返回预设的 JSON 响应
- **文件系统**：mock 配置读写

### 文件放置

```
src/
├── lib/
│   └── components/
│       └── settings/
│           └── settings-page.integration.test.ts
└── routes/
    └── +page.integration.test.ts  # 路由级集成测试
```

### vitest 配置扩展

```typescript
test: {
  environment: "jsdom",
  include: [
    "src/**/*.test.{ts,js}",           // 单元测试
    "src/**/*.integration.test.{ts,js}" // 集成测试
  ],
}
```

## CI/CD 配置

### 触发策略

```yaml
on:
  push:
    branches: ['**']      # 所有分支
  pull_request:
    branches: [main]
```

**双重保险：**
- **Push 时**：任何分支推送都立即运行测试，尽早发现问题
- **PR 时**：PR 合并前再次验证，配合 Branch Protection 阻止合入

### Workflow

```yaml
name: CI
on:
  push:
    branches: ['**']
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: 'npm'
      
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install dependencies
        run: npm ci
      
      - name: Frontend tests
        run: npm run test
      
      - name: Rust tests
        run: npm run test:rust
      
      - name: Type check
        run: npm run check
      
      - name: Rust lint
        run: npm run lint:rust
```

### Branch Protection Rules

```
Settings → Branches → main → Add rule:
  ☑ Require status checks to pass before merging
  ☑ Require branches to be up to date before merging
  Status checks: "CI / test"
```

### 失败处理

| 场景 | 行为 |
|------|------|
| Push 到开发分支失败 | CI 标记失败，开发者收到通知 |
| PR 测试失败 | 无法合并，必须修复 |
| 测试超时 | 重试一次，仍然失败则标记为失败 |

## 运行命令

```bash
# 前端测试（单元 + 集成）
npm run test

# Rust 测试（单元 + 集成 + IPC 契约）
npm run test:rust

# 全部检查
npm run ci
```

## 工作量估算

| 层级 | 测试文件数 | 测试用例数 | 预估工作量 |
|------|-----------|-----------|-----------|
| Rust 模块集成 | 3 个 | ~25 个 | 1-2 天 |
| IPC 契约测试 | 1 个 | ~15 个 | 0.5-1 天 |
| 前端组件集成 | 4 个 | ~20 个 | 1-2 天 |
| CI/CD 配置 | 1 个 workflow | - | 0.5 天 |
| **合计** | **9 个** | **~60 个** | **3-5.5 天** |

## 关键设计决策

1. Rust 集成测试放在 `src-tauri/tests/` 目录，与 `#[cfg(test)]` 单元测试分离
2. 前端集成测试使用 `.integration.test.ts` 后缀与单元测试区分
3. 所有外部依赖（LLM、文件系统、网络）都通过 mock 隔离
4. 与现有 Phase 2 计划完全兼容，复用 `LlmClient` trait 和 mockall
5. CI 在所有分支 push 时运行，PR 时再次验证，双重保障
6. 主分支通过 Branch Protection 禁止直接 push
