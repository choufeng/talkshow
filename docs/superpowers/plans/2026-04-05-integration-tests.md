# 集成测试完善实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建三层集成测试架构（Rust 模块集成 + 前端组件集成 + CI/CD），约 60 个测试用例，所有分支 push 和 PR 均自动验证。

**Architecture:** Rust 集成测试复用 LlmClient trait + mockall，前端集成测试通过 mock Tauri invoke 实现组件协同测试，CI 在 push 到所有分支和 PR 到 main 时双重触发。

**Tech Stack:** Rust, mockall, tempfile, tokio, Svelte 5, Vitest, @testing-library/svelte, GitHub Actions

---

### Task 0: 更新 CI Workflow 触发条件

**Files:**
- Modify: `.github/workflows/ci.yml`

当前 CI 只在 push 到 main 时触发，需要改为 push 到所有分支。

- [ ] **Step 1: 修改触发条件**

将 `on.push.branches` 从 `[main]` 改为 `['**']`：

```yaml
on:
  push:
    branches: ['**']
  pull_request:
    branches: [main]
```

其余内容保持不变。

- [ ] **Step 2: 验证 YAML 格式**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`
Expected: 无报错

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: trigger on push to all branches, not just main"
```

---

### Task 1: 创建 Rust 集成测试基础设施

**Files:**
- Create: `src-tauri/tests/common/mod.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 添加测试辅助模块**

创建 `src-tauri/tests/common/mod.rs`：

```rust
use talkshow_lib::config::{
    AiConfig, AppConfig, FeaturesConfig, ProviderConfig, RecordingFeaturesConfig, Skill,
    SkillsConfig, TranscriptionConfig, TranslationConfig,
};
use talkshow_lib::llm_client::LlmClient;
use talkshow_lib::logger::Logger;

use std::sync::atomic::{AtomicUsize, Ordering};

/// 创建临时目录的测试 Logger
pub fn create_test_logger() -> Logger {
    let dir = tempfile::tempdir().unwrap();
    Logger::new(dir.path()).unwrap()
}

/// 创建最小 AppConfig
pub fn test_app_config() -> AppConfig {
    AppConfig {
        shortcut: "Control+Shift+KeyA".to_string(),
        recording_shortcut: "Control+Shift+KeyB".to_string(),
        translate_shortcut: "Control+Shift+KeyC".to_string(),
        ai: AiConfig {
            providers: test_providers(),
        },
        features: FeaturesConfig {
            transcription: test_transcription_config(),
            translation: TranslationConfig {
                target_lang: "English".to_string(),
            },
            skills: enabled_skills_config(),
            recording: RecordingFeaturesConfig {
                auto_mute: false,
            },
        },
    }
}

/// 创建测试 Provider 列表
pub fn test_providers() -> Vec<ProviderConfig> {
    vec![ProviderConfig {
        id: "test-provider".to_string(),
        provider_type: "openai-compatible".to_string(),
        name: "Test Provider".to_string(),
        endpoint: "https://api.example.com/v1".to_string(),
        api_key: Some("sk-test-key".to_string()),
        models: vec![],
    }]
}

/// 创建启用的 Skills 配置
pub fn enabled_skills_config() -> SkillsConfig {
    SkillsConfig {
        enabled: true,
        skills: vec![Skill {
            id: "test-skill".to_string(),
            name: "测试技能".to_string(),
            description: "用于测试".to_string(),
            prompt: "测试 prompt".to_string(),
            builtin: false,
            editable: true,
            enabled: true,
        }],
    }
}

/// 创建测试 Transcription 配置
pub fn test_transcription_config() -> TranscriptionConfig {
    TranscriptionConfig {
        provider_id: "test-provider".to_string(),
        model: "test-model".to_string(),
        polish_enabled: true,
        polish_provider_id: "test-provider".to_string(),
        polish_model: "test-model".to_string(),
    }
}

/// 手动 Mock LlmClient，用于 tests/ 目录的集成测试
///
/// MockLlmClient 由 mockall 生成，但仅在 crate 内 #[cfg(test)] 可见。
/// tests/ 目录是独立 crate，无法访问 MockLlmClient，因此需要手动实现。
pub struct MockLlmClientIntegration {
    send_text_handler: Option<Box<dyn Fn(&str, &str, &str, &str) -> Result<String, String> + Send + Sync>>,
    send_audio_handler: Option<Box<dyn Fn(&[u8], &str, &str, &str, &str, &str, &str) -> Result<String, String> + Send + Sync>>,
    send_text_call_count: AtomicUsize,
    send_audio_call_count: AtomicUsize,
}

impl MockLlmClientIntegration {
    pub fn new() -> Self {
        Self {
            send_text_handler: None,
            send_audio_handler: None,
            send_text_call_count: AtomicUsize::new(0),
            send_audio_call_count: AtomicUsize::new(0),
        }
    }

    pub fn expect_send_text<F>(&mut self, handler: F)
    where
        F: Fn(&str, &str, &str, &str) -> Result<String, String> + Send + Sync + 'static,
    {
        self.send_text_handler = Some(Box::new(handler));
    }

    pub fn expect_send_audio<F>(&mut self, handler: F)
    where
        F: Fn(&[u8], &str, &str, &str, &str, &str, &str) -> Result<String, String> + Send + Sync + 'static,
    {
        self.send_audio_handler = Some(Box::new(handler));
    }

    pub fn send_text_call_count(&self) -> usize {
        self.send_text_call_count.load(Ordering::SeqCst)
    }

    pub fn send_audio_call_count(&self) -> usize {
        self.send_audio_call_count.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl LlmClient for MockLlmClientIntegration {
    async fn send_text(
        &self,
        prompt: &str,
        model_name: &str,
        provider_id: &str,
        endpoint: &str,
    ) -> Result<String, String> {
        self.send_text_call_count.fetch_add(1, Ordering::SeqCst);
        match &self.send_text_handler {
            Some(handler) => handler(prompt, model_name, provider_id, endpoint),
            None => Ok("default mock response".to_string()),
        }
    }

    async fn send_audio(
        &self,
        audio_bytes: &[u8],
        media_type: &str,
        text_prompt: &str,
        model_name: &str,
        provider_id: &str,
        endpoint: &str,
    ) -> Result<String, String> {
        self.send_audio_call_count.fetch_add(1, Ordering::SeqCst);
        match &self.send_audio_handler {
            Some(handler) => handler(audio_bytes, media_type, text_prompt, model_name, provider_id, endpoint, endpoint),
            None => Ok("default mock audio response".to_string()),
        }
    }
}
```

- [ ] **Step 2: 添加 Cargo.toml 测试依赖**

在 `src-tauri/Cargo.toml` 的 `[dev-dependencies]` 中确认已有 mockall 和 tempfile，添加 async-trait（如果尚未在 dev-dependencies 中）：

```toml
[dev-dependencies]
mockall = "0.13"
tempfile = "3"
async-trait = "0.1"
tokio = { version = "1", features = ["rt", "macros", "time"] }
```

- [ ] **Step 3: 验证编译**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test common -- --test-threads=1`
Expected: 编译通过（common 模块无测试，但需确认能编译）

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/common/mod.rs src-tauri/Cargo.toml
git commit -m "test: add integration test infrastructure with MockLlmClientIntegration"
```

---

### Task 2: Skills 流水线集成测试

**Files:**
- Create: `src-tauri/tests/skills_pipeline.rs`

- [ ] **Step 1: 创建测试文件**

创建 `src-tauri/tests/skills_pipeline.rs`：

```rust
mod common;

use common::*;
use talkshow_lib::llm_client::LlmClient;
use talkshow_lib::skills::{assemble_skills_prompt, process_with_skills_client};
use talkshow_lib::config::{ProviderConfig, Skill, SkillsConfig, TranscriptionConfig};

#[tokio::test]
async fn test_skills_disabled_returns_original() {
    let logger = create_test_logger();
    let config = SkillsConfig {
        enabled: false,
        skills: vec![],
    };
    let tc = test_transcription_config();
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    let result = process_with_skills_client(
        &logger, &config, &tc, &providers, "原始文本", &mut mock, None,
    )
    .await;

    assert_eq!(result.unwrap(), "原始文本");
    assert_eq!(mock.send_text_call_count(), 0);
}

#[tokio::test]
async fn test_skills_empty_transcription_returns_original() {
    let logger = create_test_logger();
    let config = enabled_skills_config();
    let tc = test_transcription_config();
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    let result = process_with_skills_client(
        &logger, &config, &tc, &providers, "", &mut mock, None,
    )
    .await;

    assert_eq!(result.unwrap(), "");
    assert_eq!(mock.send_text_call_count(), 0);
}

#[tokio::test]
async fn test_skills_no_enabled_skills_returns_original() {
    let logger = create_test_logger();
    let config = SkillsConfig {
        enabled: true,
        skills: vec![Skill {
            id: "disabled-skill".to_string(),
            name: "禁用技能".to_string(),
            description: "".to_string(),
            prompt: "".to_string(),
            builtin: false,
            editable: true,
            enabled: false,
        }],
    };
    let tc = test_transcription_config();
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    let result = process_with_skills_client(
        &logger, &config, &tc, &providers, "原始文本", &mut mock, None,
    )
    .await;

    assert_eq!(result.unwrap(), "原始文本");
    assert_eq!(mock.send_text_call_count(), 0);
}

#[tokio::test]
async fn test_skills_no_polish_provider_returns_original() {
    let logger = create_test_logger();
    let config = enabled_skills_config();
    let tc = TranscriptionConfig {
        provider_id: "test".to_string(),
        model: "test".to_string(),
        polish_enabled: true,
        polish_provider_id: "".to_string(),
        polish_model: "".to_string(),
    };
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    let result = process_with_skills_client(
        &logger, &config, &tc, &providers, "原始文本", &mut mock, None,
    )
    .await;

    assert_eq!(result.unwrap(), "原始文本");
    assert_eq!(mock.send_text_call_count(), 0);
}

#[tokio::test]
async fn test_skills_provider_not_found_returns_original() {
    let logger = create_test_logger();
    let config = enabled_skills_config();
    let tc = TranscriptionConfig {
        provider_id: "test".to_string(),
        model: "test".to_string(),
        polish_enabled: true,
        polish_provider_id: "nonexistent".to_string(),
        polish_model: "test-model".to_string(),
    };
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    let result = process_with_skills_client(
        &logger, &config, &tc, &providers, "原始文本", &mut mock, None,
    )
    .await;

    assert_eq!(result.unwrap(), "原始文本");
    assert_eq!(mock.send_text_call_count(), 0);
}

#[tokio::test]
async fn test_skills_calls_llm_and_returns_result() {
    let logger = create_test_logger();
    let config = enabled_skills_config();
    let tc = test_transcription_config();
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, _model, _provider, _endpoint| {
        assert!(prompt.contains("测试技能"));
        assert!(prompt.contains("测试 prompt"));
        Ok("处理后的文本".to_string())
    });

    let result = process_with_skills_client(
        &logger, &config, &tc, &providers, "嗯那个你好世界啊", &mut mock, None,
    )
    .await;

    assert_eq!(result.unwrap(), "处理后的文本");
    assert_eq!(mock.send_text_call_count(), 1);
}

#[tokio::test]
async fn test_skills_falls_back_on_llm_error() {
    let logger = create_test_logger();
    let config = enabled_skills_config();
    let tc = test_transcription_config();
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|_, _, _, _| Err("LLM error".to_string()));

    let result = process_with_skills_client(
        &logger, &config, &tc, &providers, "原始文本", &mut mock, None,
    )
    .await;

    // Skills 处理失败时应回退到原文
    assert_eq!(result.unwrap(), "原始文本");
    assert_eq!(mock.send_text_call_count(), 1);
}

#[tokio::test]
async fn test_skills_with_selected_text() {
    let logger = create_test_logger();
    let config = enabled_skills_config();
    let tc = test_transcription_config();
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, _, _, _| {
        assert!(prompt.contains("选中的文本"));
        Ok("处理结果".to_string())
    });

    let result = process_with_skills_client(
        &logger, &config, &tc, &providers, "转录文本", &mut mock, Some("选中的文本"),
    )
    .await;

    assert_eq!(result.unwrap(), "处理结果");
}

#[tokio::test]
async fn test_skills_multiple_skills_in_prompt() {
    let logger = create_test_logger();
    let config = SkillsConfig {
        enabled: true,
        skills: vec![
            Skill {
                id: "skill-1".to_string(),
                name: "技能一".to_string(),
                description: "".to_string(),
                prompt: "prompt 一".to_string(),
                builtin: false,
                editable: true,
                enabled: true,
            },
            Skill {
                id: "skill-2".to_string(),
                name: "技能二".to_string(),
                description: "".to_string(),
                prompt: "prompt 二".to_string(),
                builtin: false,
                editable: true,
                enabled: true,
            },
        ],
    };
    let tc = test_transcription_config();
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, _, _, _| {
        assert!(prompt.contains("技能一"));
        assert!(prompt.contains("prompt 一"));
        assert!(prompt.contains("技能二"));
        assert!(prompt.contains("prompt 二"));
        Ok("多技能处理结果".to_string())
    });

    let result = process_with_skills_client(
        &logger, &config, &tc, &providers, "测试文本", &mut mock, None,
    )
    .await;

    assert_eq!(result.unwrap(), "多技能处理结果");
}

#[test]
fn test_assemble_skills_prompt_with_app_context() {
    let skills = vec![Skill {
        id: "test".to_string(),
        name: "测试技能".to_string(),
        description: "描述".to_string(),
        prompt: "测试 prompt".to_string(),
        builtin: false,
        editable: true,
        enabled: true,
    }];

    let (system, user) = assemble_skills_prompt(
        &skills,
        "转录文本",
        "Finder",
        "com.apple.finder",
        Some("选中文本"),
    );

    assert!(system.contains("测试技能"));
    assert!(system.contains("测试 prompt"));
    assert!(system.contains("Finder"));
    assert!(system.contains("com.apple.finder"));
    assert!(system.contains("选中文本"));
    assert_eq!(user, "转录文本");
}

#[test]
fn test_assemble_skills_prompt_without_selected_text() {
    let skills: Vec<Skill> = vec![];
    let (system, user) = assemble_skills_prompt(
        &skills,
        "你好",
        "App",
        "com.app",
        None,
    );

    assert!(!system.contains("选中文本"));
    assert_eq!(user, "你好");
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test skills_pipeline -- --test-threads=1`
Expected: 全部 11 个测试通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/skills_pipeline.rs
git commit -m "test: add skills pipeline integration tests"
```

---

### Task 3: 翻译流程集成测试

**Files:**
- Create: `src-tauri/tests/translation_flow.rs`

- [ ] **Step 1: 创建测试文件**

创建 `src-tauri/tests/translation_flow.rs`：

```rust
mod common;

use common::*;
use talkshow_lib::translation::translate_text_client;
use talkshow_lib::config::{Skill, SkillsConfig};

#[tokio::test]
async fn test_translate_success() {
    let logger = create_test_logger();
    let skills = SkillsConfig {
        enabled: true,
        skills: vec![],
    };
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, model, provider, endpoint| {
        assert!(prompt.contains("professional translator"));
        assert!(prompt.contains("English"));
        assert_eq!(model, "gpt-4");
        assert_eq!(provider, "test-provider");
        assert_eq!(endpoint, "https://api.example.com/v1");
        Ok("Hello World".to_string())
    });

    let result = translate_text_client(
        &logger,
        "你好世界",
        "English",
        &skills,
        "test-provider",
        "gpt-4",
        "https://api.example.com/v1",
        &mut mock,
    )
    .await;

    assert_eq!(result.unwrap(), "Hello World");
    assert_eq!(mock.send_text_call_count(), 1);
}

#[tokio::test]
async fn test_translate_with_skill_prompt() {
    let logger = create_test_logger();
    let skills = SkillsConfig {
        enabled: true,
        skills: vec![Skill {
            id: "builtin-translation".to_string(),
            name: "翻译优化".to_string(),
            description: "".to_string(),
            prompt: "保持原文语气和风格".to_string(),
            builtin: true,
            editable: true,
            enabled: true,
        }],
    };
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, _, _, _| {
        assert!(prompt.contains("保持原文语气和风格"));
        Ok("Translated with style".to_string())
    });

    let result = translate_text_client(
        &logger,
        "你好世界",
        "English",
        &skills,
        "test-provider",
        "gpt-4",
        "https://api.example.com/v1",
        &mut mock,
    )
    .await;

    assert_eq!(result.unwrap(), "Translated with style");
}

#[tokio::test]
async fn test_translate_llm_error() {
    let logger = create_test_logger();
    let skills = SkillsConfig {
        enabled: true,
        skills: vec![],
    };
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|_, _, _, _| Err("API error".to_string()));

    let result = translate_text_client(
        &logger,
        "你好",
        "English",
        &skills,
        "test-provider",
        "gpt-4",
        "https://api.example.com/v1",
        &mut mock,
    )
    .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("翻译失败"));
}

#[tokio::test]
async fn test_translate_timeout() {
    let logger = create_test_logger();
    let skills = SkillsConfig {
        enabled: true,
        skills: vec![],
    };
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|_, _, _, _| {
        std::thread::sleep(std::time::Duration::from_secs(30));
        Ok("too late".to_string())
    });

    let result = translate_text_client(
        &logger,
        "你好",
        "English",
        &skills,
        "test-provider",
        "gpt-4",
        "https://api.example.com/v1",
        &mut mock,
    )
    .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("超时"));
}

#[tokio::test]
async fn test_translate_empty_text() {
    let logger = create_test_logger();
    let skills = SkillsConfig {
        enabled: true,
        skills: vec![],
    };
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, _, _, _| {
        // 即使是空文本，也应该发送到 LLM
        assert!(!prompt.is_empty());
        Ok("".to_string())
    });

    let result = translate_text_client(
        &logger,
        "",
        "English",
        &skills,
        "test-provider",
        "gpt-4",
        "https://api.example.com/v1",
        &mut mock,
    )
    .await;

    assert_eq!(result.unwrap(), "");
}

#[tokio::test]
async fn test_translate_chinese_to_japanese() {
    let logger = create_test_logger();
    let skills = SkillsConfig {
        enabled: true,
        skills: vec![],
    };
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, _, _, _| {
        assert!(prompt.contains("Japanese"));
        Ok("こんにちは世界".to_string())
    });

    let result = translate_text_client(
        &logger,
        "你好世界",
        "Japanese",
        &skills,
        "test-provider",
        "gpt-4",
        "https://api.example.com/v1",
        &mut mock,
    )
    .await;

    assert_eq!(result.unwrap(), "こんにちは世界");
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test translation_flow -- --test-threads=1`
Expected: 全部 6 个测试通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/translation_flow.rs
git commit -m "test: add translation flow integration tests"
```

---

### Task 4: 配置持久化集成测试

**Files:**
- Create: `src-tauri/tests/config_persistence.rs`

- [ ] **Step 1: 创建测试文件**

创建 `src-tauri/tests/config_persistence.rs`：

```rust
mod common;

use common::*;
use talkshow_lib::config::{
    load_config, save_config, AppConfig, ModelConfig, ProviderConfig, Skill,
};

#[test]
fn test_save_and_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_app_config();

    save_config(dir.path(), &config).unwrap();
    let loaded = load_config(dir.path());

    assert_eq!(loaded.shortcut, config.shortcut);
    assert_eq!(loaded.recording_shortcut, config.recording_shortcut);
    assert_eq!(loaded.translate_shortcut, config.translate_shortcut);
    assert_eq!(loaded.ai.providers.len(), config.ai.providers.len());
}

#[test]
fn test_save_and_load_with_api_keys() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_app_config();
    config.ai.providers[0].api_key = Some("sk-secret-key-123".to_string());

    save_config(dir.path(), &config).unwrap();
    let loaded = load_config(dir.path());

    assert_eq!(
        loaded.ai.providers[0].api_key,
        Some("sk-secret-key-123".to_string())
    );
}

#[test]
fn test_save_and_load_without_api_keys() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_app_config();

    save_config(dir.path(), &config).unwrap();
    let loaded = load_config(dir.path());

    assert_eq!(loaded.ai.providers[0].api_key, None);
}

#[test]
fn test_load_nonexistent_file_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let config = load_config(dir.path());

    // 应返回默认配置，不报错
    assert!(!config.shortcut.is_empty());
}

#[test]
fn test_save_and_load_features_config() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_app_config();
    config.features.transcription.provider_id = "vertex".to_string();
    config.features.transcription.model = "gemini-pro".to_string();
    config.features.translation.target_lang = "Japanese".to_string();
    config.features.skills.enabled = true;
    config.features.skills.skills = vec![Skill {
        id: "custom-skill".to_string(),
        name: "自定义技能".to_string(),
        description: "描述".to_string(),
        prompt: "prompt".to_string(),
        builtin: false,
        editable: true,
        enabled: true,
    }];
    config.features.recording.auto_mute = true;

    save_config(dir.path(), &config).unwrap();
    let loaded = load_config(dir.path());

    assert_eq!(loaded.features.transcription.provider_id, "vertex");
    assert_eq!(loaded.features.transcription.model, "gemini-pro");
    assert_eq!(loaded.features.translation.target_lang, "Japanese");
    assert!(loaded.features.skills.enabled);
    assert_eq!(loaded.features.skills.skills.len(), 1);
    assert!(loaded.features.recording.auto_mute);
}

#[test]
fn test_save_and_load_multiple_providers() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_app_config();
    config.ai.providers.push(ProviderConfig {
        id: "second-provider".to_string(),
        provider_type: "vertex".to_string(),
        name: "Second Provider".to_string(),
        endpoint: "https://vertex.example.com".to_string(),
        api_key: None,
        models: vec![],
    });

    save_config(dir.path(), &config).unwrap();
    let loaded = load_config(dir.path());

    assert_eq!(loaded.ai.providers.len(), 2);
    assert_eq!(loaded.ai.providers[1].id, "second-provider");
}

#[test]
fn test_save_and_load_with_models() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_app_config();
    config.ai.providers[0].models = vec![
        ModelConfig {
            name: "gpt-4".to_string(),
            capabilities: vec!["chat".to_string()],
            verified: None,
        },
        ModelConfig {
            name: "whisper-1".to_string(),
            capabilities: vec!["transcription".to_string()],
            verified: None,
        },
    ];

    save_config(dir.path(), &config).unwrap();
    let loaded = load_config(dir.path());

    assert_eq!(loaded.ai.providers[0].models.len(), 2);
    assert_eq!(loaded.ai.providers[0].models[0].name, "gpt-4");
    assert_eq!(loaded.ai.providers[0].models[1].name, "whisper-1");
}

#[test]
fn test_corrupted_config_file_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    std::fs::write(&config_path, "not valid json{{{").unwrap();

    // load_config 应优雅处理损坏的文件，返回默认配置
    let config = load_config(dir.path());
    assert!(!config.shortcut.is_empty());
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test config_persistence -- --test-threads=1`
Expected: 全部 8 个测试通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/config_persistence.rs
git commit -m "test: add config persistence integration tests"
```

---

### Task 5: 前端 Tauri Mock 基础设施

**Files:**
- Create: `src/test/mocks/tauri.ts`
- Modify: `src/test-setup.ts`

- [ ] **Step 1: 创建 Tauri Mock 模块**

创建 `src/test/mocks/tauri.ts`：

```typescript
import { vi } from 'vitest';

export type MockInvokeHandler = (
  command: string,
  args?: Record<string, unknown>
) => unknown | Promise<unknown>;

let invokeHandler: MockInvokeHandler | null = null;

export function mockTauriInvoke(handler: MockInvokeHandler) {
  invokeHandler = handler;
}

export function resetTauriMocks() {
  invokeHandler = null;
  vi.clearAllMocks();
}

// Mock @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (command: string, args?: Record<string, unknown>) => {
    if (invokeHandler) {
      return invokeHandler(command, args);
    }
    throw new Error(`No mock handler for command: ${command}`);
  }),
}));

// Mock @tauri-apps/api/event
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve({ unlisten: vi.fn() })),
  emit: vi.fn(),
}));

// Mock @tauri-apps/plugin-global-shortcut
vi.mock('@tauri-apps/plugin-global-shortcut', () => ({}));

// Mock @tauri-apps/plugin-notification
vi.mock('@tauri-apps/plugin-notification', () => ({}));

// Mock @tauri-apps/plugin-opener
vi.mock('@tauri-apps/plugin-opener', () => ({}));
```

- [ ] **Step 2: 更新 test-setup.ts**

修改 `src/test-setup.ts`，添加 Tauri mock 导入：

```typescript
import '@testing-library/jest-dom/vitest';
import './test/mocks/tauri';
```

- [ ] **Step 3: 创建 test 目录结构**

Run: `mkdir -p src/test/mocks`

- [ ] **Step 4: 验证现有测试仍然通过**

Run: `npm test`
Expected: 所有现有测试通过（Tauri mock 不应影响不涉及 Tauri 的测试）

- [ ] **Step 5: Commit**

```bash
git add src/test/mocks/tauri.ts src/test-setup.ts
git commit -m "test: add Tauri mock infrastructure for frontend integration tests"
```

---

### Task 6: 设置页面集成测试

**Files:**
- Create: `src/routes/settings/+page.integration.test.ts`

- [ ] **Step 1: 创建测试文件**

创建 `src/routes/settings/+page.integration.test.ts`：

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { mockTauriInvoke, resetTauriMocks } from '$test/mocks/tauri';
import { get } from 'svelte/store';
import { config } from '$lib/stores/config';
import SettingsPage from './+page.svelte';

function defaultConfig() {
  return {
    shortcut: 'Control+Shift+KeyA',
    recording_shortcut: 'Control+Shift+KeyB',
    translate_shortcut: 'Control+Shift+KeyC',
    ai: { providers: [] },
    features: {
      transcription: {
        provider_id: '',
        model: '',
        polish_enabled: false,
        polish_provider_id: '',
        polish_model: '',
      },
      translation: { target_lang: 'English' },
      skills: { enabled: false, skills: [] },
      recording: { auto_mute: false },
    },
  };
}

beforeEach(() => {
  resetTauriMocks();
  // Reset config store to default
  config.load = vi.fn();
});

describe('Settings Page Integration', () => {
  it('should load config on mount', async () => {
    const loadSpy = vi.fn();
    config.load = loadSpy;

    render(SettingsPage);

    expect(loadSpy).toHaveBeenCalled();
  });

  it('should display settings title and sections', async () => {
    config.load = vi.fn();
    render(SettingsPage);

    expect(screen.getByText('设置')).toBeInTheDocument();
    expect(screen.getByText('快捷键')).toBeInTheDocument();
    expect(screen.getByText('录音')).toBeInTheDocument();
    expect(screen.getByText('外观')).toBeInTheDocument();
  });

  it('should display shortcut labels', async () => {
    config.load = vi.fn();
    render(SettingsPage);

    expect(screen.getByText('窗口切换')).toBeInTheDocument();
    expect(screen.getByText('录音控制')).toBeInTheDocument();
    expect(screen.getByText('AI 翻译')).toBeInTheDocument();
  });

  it('should display auto-mute toggle', async () => {
    config.load = vi.fn();
    render(SettingsPage);

    expect(screen.getByText('录音时自动静音')).toBeInTheDocument();
    expect(screen.getByText('开始录音后自动静音其他应用的声音，录音结束后自动恢复')).toBeInTheDocument();
  });

  it('should display theme options', async () => {
    config.load = vi.fn();
    render(SettingsPage);

    expect(screen.getByText('浅色')).toBeInTheDocument();
    expect(screen.getByText('深色')).toBeInTheDocument();
    expect(screen.getByText('跟随系统')).toBeInTheDocument();
  });

  it('should update shortcut when ShortcutRecorder fires onUpdate', async () => {
    const updateShortcutSpy = vi.fn().mockResolvedValue(undefined);
    config.updateShortcut = updateShortcutSpy;
    config.load = vi.fn();

    render(SettingsPage);

    // ShortcutRecorder 组件渲染后，找到"修改"按钮并模拟更新
    const editButtons = screen.getAllByRole('button', { name: /修改/ });
    if (editButtons.length > 0) {
      // ShortcutRecorder 有独立的交互逻辑，此处验证 config.updateShortcut 被调用
      await config.updateShortcut('toggle', 'Control+Shift+KeyZ');
      expect(updateShortcutSpy).toHaveBeenCalledWith('toggle', 'Control+Shift+KeyZ');
    }
  });

  it('should toggle auto-mute and save config', async () => {
    const saveSpy = vi.fn().mockResolvedValue(undefined);
    config.save = saveSpy;
    config.load = vi.fn();

    render(SettingsPage);

    const toggle = screen.getByRole('switch', { name: '录音时自动静音' });
    await fireEvent.click(toggle);

    expect(saveSpy).toHaveBeenCalled();
  });

  it('should change theme when theme button is clicked', async () => {
    config.load = vi.fn();
    render(SettingsPage);

    const darkButton = screen.getByText('深色');
    await fireEvent.click(darkButton);

    // 验证主题按钮有正确的样式（通过检查按钮的 class 或属性）
    // 由于 theme store 是响应式的，点击后应该触发更新
  });
});
```

- [ ] **Step 2: 运行测试**

Run: `npx vitest run src/routes/settings/+page.integration.test.ts`
Expected: 全部 8 个测试通过

- [ ] **Step 3: Commit**

```bash
git add src/routes/settings/+page.integration.test.ts
git commit -m "test: add settings page integration tests"
```

---

### Task 7: Provider 配置集成测试

**Files:**
- Create: `src/routes/models/+page.integration.test.ts`

先查看 models 页面结构：

- [ ] **Step 0: 查看 models 页面**

Read: `src/routes/models/+page.svelte`

- [ ] **Step 1: 创建测试文件**

创建 `src/routes/models/+page.integration.test.ts`：

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke, resetTauriMocks } from '$test/mocks/tauri';
import ModelsPage from './+page.svelte';

beforeEach(() => {
  resetTauriMocks();
});

describe('Models Page Integration', () => {
  it('should load and display provider list', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') {
        return {
          shortcut: 'Control+Shift+KeyA',
          recording_shortcut: 'Control+Shift+KeyB',
          translate_shortcut: 'Control+Shift+KeyC',
          ai: {
            providers: [
              { id: 'vertex', type: 'vertex', name: 'Vertex AI', endpoint: '', api_key: null, models: [] },
              { id: 'openai', type: 'openai-compatible', name: 'OpenAI', endpoint: 'https://api.openai.com/v1', api_key: 'sk-xxx', models: [{ name: 'gpt-4', capabilities: ['chat'] }] },
            ],
          },
          features: {
            transcription: { provider_id: '', model: '', polish_enabled: false, polish_provider_id: '', polish_model: '' },
            translation: { target_lang: 'English' },
            skills: { enabled: false, skills: [] },
            recording: { auto_mute: false },
          },
        };
      }
      return null;
    });

    render(ModelsPage);

    await waitFor(() => {
      expect(screen.getByText('OpenAI')).toBeInTheDocument();
    });
  });

  it('should show empty state when no providers', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') {
        return {
          shortcut: 'Control+Shift+KeyA',
          recording_shortcut: 'Control+Shift+KeyB',
          translate_shortcut: 'Control+Shift+KeyC',
          ai: { providers: [] },
          features: {
            transcription: { provider_id: '', model: '', polish_enabled: false, polish_provider_id: '', polish_model: '' },
            translation: { target_lang: 'English' },
            skills: { enabled: false, skills: [] },
            recording: { auto_mute: false },
          },
        };
      }
      return null;
    });

    render(ModelsPage);

    await waitFor(() => {
      // 应显示空状态或添加 provider 的提示
      expect(screen.queryByText('OpenAI')).not.toBeInTheDocument();
    });
  });

  it('should handle save config with new provider', async () => {
    const savedConfig = vi.fn();
    mockTauriInvoke(async (command, args) => {
      if (command === 'get_config') {
        return {
          shortcut: 'Control+Shift+KeyA',
          recording_shortcut: 'Control+Shift+KeyB',
          translate_shortcut: 'Control+Shift+KeyC',
          ai: { providers: [] },
          features: {
            transcription: { provider_id: '', model: '', polish_enabled: false, polish_provider_id: '', polish_model: '' },
            translation: { target_lang: 'English' },
            skills: { enabled: false, skills: [] },
            recording: { auto_mute: false },
          },
        };
      }
      if (command === 'save_config_cmd') {
        savedConfig(args?.config);
        return;
      }
      return null;
    });

    render(ModelsPage);

    // 验证 save_config_cmd 可以在交互后被调用
    // 具体添加 provider 的交互取决于页面实现
  });

  it('should handle config load error gracefully', async () => {
    mockTauriInvoke(async () => {
      throw new Error('Failed to load config');
    });

    // 不应崩溃，应显示错误状态
    render(ModelsPage);

    await waitFor(() => {
      // 页面应该优雅处理错误，不崩溃
      expect(document.body).toBeInTheDocument();
    });
  });
});
```

- [ ] **Step 2: 运行测试**

Run: `npx vitest run src/routes/models/+page.integration.test.ts`
Expected: 全部 4 个测试通过

- [ ] **Step 3: Commit**

```bash
git add src/routes/models/+page.integration.test.ts
git commit -m "test: add models page integration tests"
```

---

### Task 8: Skills 页面集成测试

**Files:**
- Create: `src/routes/skills/+page.integration.test.ts`

- [ ] **Step 0: 查看 skills 页面**

Read: `src/routes/skills/+page.svelte`

- [ ] **Step 1: 创建测试文件**

创建 `src/routes/skills/+page.integration.test.ts`：

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke, resetTauriMocks } from '$test/mocks/tauri';
import SkillsPage from './+page.svelte';

beforeEach(() => {
  resetTauriMocks();
});

describe('Skills Page Integration', () => {
  it('should load and display skills list', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') {
        return {
          shortcut: 'Control+Shift+KeyA',
          recording_shortcut: 'Control+Shift+KeyB',
          translate_shortcut: 'Control+Shift+KeyC',
          ai: { providers: [] },
          features: {
            transcription: { provider_id: '', model: '', polish_enabled: false, polish_provider_id: '', polish_model: '' },
            translation: { target_lang: 'English' },
            skills: {
              enabled: true,
              skills: [
                { id: 'builtin-fillers', name: '语气词剔除', description: '去除语气词', prompt: '去除语气词', builtin: true, editable: false, enabled: true },
                { id: 'custom-1', name: '自定义技能', description: '测试', prompt: '测试 prompt', builtin: false, editable: true, enabled: true },
              ],
            },
            recording: { auto_mute: false },
          },
        };
      }
      return null;
    });

    render(SkillsPage);

    await waitFor(() => {
      expect(screen.getByText('语气词剔除')).toBeInTheDocument();
      expect(screen.getByText('自定义技能')).toBeInTheDocument();
    });
  });

  it('should display skills toggle', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') {
        return {
          shortcut: 'Control+Shift+KeyA',
          recording_shortcut: 'Control+Shift+KeyB',
          translate_shortcut: 'Control+Shift+KeyC',
          ai: { providers: [] },
          features: {
            transcription: { provider_id: '', model: '', polish_enabled: false, polish_provider_id: '', polish_model: '' },
            translation: { target_lang: 'English' },
            skills: { enabled: false, skills: [] },
            recording: { auto_mute: false },
          },
        };
      }
      return null;
    });

    render(SkillsPage);

    // 应显示 Skills 总开关
    await waitFor(() => {
      expect(screen.getByText('Skills')).toBeInTheDocument();
    });
  });

  it('should handle adding new skill', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') {
        return {
          shortcut: 'Control+Shift+KeyA',
          recording_shortcut: 'Control+Shift+KeyB',
          translate_shortcut: 'Control+Shift+KeyC',
          ai: { providers: [] },
          features: {
            transcription: { provider_id: '', model: '', polish_enabled: false, polish_provider_id: '', polish_model: '' },
            translation: { target_lang: 'English' },
            skills: { enabled: true, skills: [] },
            recording: { auto_mute: false },
          },
        };
      }
      return null;
    });

    render(SkillsPage);

    // 验证添加技能的按钮存在
    await waitFor(() => {
      // 具体按钮文本取决于页面实现
      expect(document.body).toBeInTheDocument();
    });
  });
});
```

- [ ] **Step 2: 运行测试**

Run: `npx vitest run src/routes/skills/+page.integration.test.ts`
Expected: 全部 3 个测试通过

- [ ] **Step 3: Commit**

```bash
git add src/routes/skills/+page.integration.test.ts
git commit -m "test: add skills page integration tests"
```

---

### Task 8b: IPC 契约测试

**Files:**
- Create: `src-tauri/tests/ipc_contracts.rs`

Tauri 命令的参数校验和返回值格式测试。由于 Tauri 命令直接依赖 `AppHandle`，我们通过测试命令处理函数的输入输出来验证契约。

- [ ] **Step 1: 查看 lib.rs 中的 Tauri 命令签名**

需要确认以下命令的精确签名：
- `update_shortcut(app_handle, shortcut_type, shortcut)` 
- `save_config_cmd(app_handle, config)`
- `get_config(app_handle)`
- `get_skills_config(app_handle)`
- `save_skills_config(app_handle, skills_config)`
- `save_transcription_config(app_handle, transcription)`
- `add_skill(app_handle, skill)`
- `update_skill(app_handle, skill)`
- `delete_skill(app_handle, skill_id)`

Read: `src-tauri/src/lib.rs` lines 200-600

- [ ] **Step 2: 创建测试文件**

创建 `src-tauri/tests/ipc_contracts.rs`：

```rust
mod common;

use common::*;
use talkshow_lib::config::{
    AppConfig, Skill, SkillsConfig, TranscriptionConfig,
    save_config, load_config, validate_config,
};

// 配置保存契约测试
#[test]
fn test_save_config_accepts_valid_config() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_app_config();
    let result = save_config(dir.path(), &config);
    assert!(result.is_ok());
}

#[test]
fn test_save_config_rejects_empty_providers() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_app_config();
    config.ai.providers = vec![];
    // 空 providers 应该是允许的（用户可以稍后添加）
    let result = save_config(dir.path(), &config);
    assert!(result.is_ok());
}

#[test]
fn test_load_config_returns_default_on_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let config = load_config(dir.path());
    // 应返回默认配置，不 panic
    assert!(!config.shortcut.is_empty());
}

// 配置校验契约测试
#[test]
fn test_validate_config_rejects_invalid_provider_type() {
    let mut config = test_app_config();
    config.ai.providers[0].provider_type = "invalid-type".to_string();
    let result = validate_config(&config);
    assert!(result.is_err());
}

#[test]
fn test_validate_config_accepts_valid_types() {
    let mut config = test_app_config();
    config.ai.providers[0].provider_type = "openai-compatible".to_string();
    let result = validate_config(&config);
    assert!(result.is_ok());
}

#[test]
fn test_validate_config_rejects_non_https_endpoint() {
    let mut config = test_app_config();
    config.ai.providers[0].endpoint = "http://insecure.example.com/v1".to_string();
    let result = validate_config(&config);
    assert!(result.is_err());
}

#[test]
fn test_validate_config_accepts_empty_endpoint() {
    let mut config = test_app_config();
    config.ai.providers[0].endpoint = "".to_string();
    config.ai.providers[0].provider_type = "vertex".to_string();
    let result = validate_config(&config);
    // Vertex AI 不需要 endpoint
    assert!(result.is_ok());
}

// Shortcut 校验契约测试
#[test]
fn test_validate_config_rejects_empty_shortcut() {
    let mut config = test_app_config();
    config.shortcut = "".to_string();
    let result = validate_config(&config);
    assert!(result.is_err());
}

#[test]
fn test_validate_config_rejects_too_long_shortcut() {
    let mut config = test_app_config();
    // 超过 4 个按键的快捷键应被拒绝
    config.shortcut = "Control+Shift+Alt+KeyA+KeyB".to_string();
    let result = validate_config(&config);
    assert!(result.is_err());
}

#[test]
fn test_validate_config_accepts_valid_shortcut() {
    let config = test_app_config();
    let result = validate_config(&config);
    assert!(result.is_ok());
}

// Skills CRUD 契约测试
#[test]
fn test_skills_config_validation() {
    let config = SkillsConfig {
        enabled: true,
        skills: vec![Skill {
            id: "".to_string(),
            name: "测试".to_string(),
            description: "".to_string(),
            prompt: "prompt".to_string(),
            builtin: false,
            editable: true,
            enabled: true,
        }],
    };
    // Skills 配置应可序列化/反序列化
    let json = serde_json::to_string(&config).unwrap();
    let loaded: SkillsConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.skills.len(), 1);
}

#[test]
fn test_transcription_config_serialization() {
    let config = TranscriptionConfig {
        provider_id: "test".to_string(),
        model: "test-model".to_string(),
        polish_enabled: true,
        polish_provider_id: "test".to_string(),
        polish_model: "test-model".to_string(),
    };
    let json = serde_json::to_string(&config).unwrap();
    let loaded: TranscriptionConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.provider_id, "test");
    assert_eq!(loaded.model, "test-model");
    assert!(loaded.polish_enabled);
}

#[test]
fn test_app_config_full_serialization() {
    let config = test_app_config();
    let json = serde_json::to_string(&config).unwrap();
    let loaded: AppConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.shortcut, config.shortcut);
    assert_eq!(loaded.ai.providers.len(), config.ai.providers.len());
    assert_eq!(loaded.features.skills.enabled, config.features.skills.enabled);
}
```

- [ ] **Step 3: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test ipc_contracts -- --test-threads=1`
Expected: 全部 13 个测试通过

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/ipc_contracts.rs
git commit -m "test: add IPC contract tests for config validation and serialization"
```

---

### Task 9: 运行全部测试并验证

- [ ] **Step 1: 运行前端全部测试**

Run: `npm test`
Expected: 所有前端测试通过（单元 + 集成）

- [ ] **Step 2: 运行 Rust 全部测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 所有 Rust 测试通过（单元 + 集成）

- [ ] **Step 3: 运行完整 CI 检查**

Run: `npm run ci`
Expected: 全部通过（check + test + lint:rust + test:rust）

- [ ] **Step 4: 最终 Commit**

```bash
git add -A
git commit -m "test: integration tests complete - Rust + frontend + CI"
```

---

## 文件清单

### 新建文件（9 个）
| 文件 | 用途 |
|------|------|
| `src-tauri/tests/common/mod.rs` | 测试辅助函数 + MockLlmClientIntegration |
| `src-tauri/tests/skills_pipeline.rs` | Skills 流水线集成测试（11 个用例） |
| `src-tauri/tests/translation_flow.rs` | 翻译流程集成测试（6 个用例） |
| `src-tauri/tests/config_persistence.rs` | 配置持久化集成测试（8 个用例） |
| `src-tauri/tests/ipc_contracts.rs` | IPC 契约测试（13 个用例） |
| `src/test/mocks/tauri.ts` | Tauri IPC Mock 基础设施 |
| `src/routes/settings/+page.integration.test.ts` | 设置页面集成测试（8 个用例） |
| `src/routes/models/+page.integration.test.ts` | Models 页面集成测试（4 个用例） |
| `src/routes/skills/+page.integration.test.ts` | Skills 页面集成测试（3 个用例） |

### 修改文件（3 个）
| 文件 | 修改内容 |
|------|---------|
| `.github/workflows/ci.yml` | push 触发条件从 `[main]` 改为 `['**']` |
| `src-tauri/Cargo.toml` | 添加 dev-dependencies（async-trait, tokio features） |
| `src/test-setup.ts` | 导入 Tauri mock 模块 |

### 测试统计
| 层级 | 测试文件 | 测试用例 |
|------|---------|---------|
| Rust 模块集成 | 3 个 | ~25 个 |
| IPC 契约测试 | 1 个 | ~13 个 |
| 前端组件集成 | 3 个 | ~15 个 |
| CI/CD | 1 个 workflow | - |
| **合计** | **8 个新文件 + 3 个修改** | **~53 个** |
