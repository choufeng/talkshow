# Phase 2: Trait 抽象 + P1 集成测试

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 AI 模块的外部依赖抽象为 trait，通过 mockall 编写集成测试覆盖 AI 路由、Skills 流水线、翻译、日志和快捷键解析。

**Architecture:** 从 ai.rs 提取 `LlmClient` trait，skills.rs 和 translation.rs 改为接收 `&dyn LlmClient` 参数。测试中使用 mockall 自动生成 mock 替换真实 LLM 调用。

**Tech Stack:** Rust, mockall, tempfile, tokio-test

**前置条件:** Phase 1 已完成（dev-dependencies 已安装）

---

### Task 1: 定义 LlmClient Trait

**Files:**
- Create: `src-tauri/src/llm_client.rs`
- Modify: `src-tauri/src/lib.rs`（添加 mod 声明）

- [ ] **Step 1: 创建 src-tauri/src/llm_client.rs**

```rust
use async_trait::async_trait;
use mockall::automock;

#[async_trait]
#[automock]
pub trait LlmClient: Send + Sync {
    async fn send_text(
        &self,
        prompt: &str,
        model_name: &str,
        provider_id: &str,
        endpoint: &str,
    ) -> Result<String, String>;

    async fn send_audio(
        &self,
        audio_bytes: &[u8],
        media_type: &str,
        text_prompt: &str,
        model_name: &str,
        provider_id: &str,
        endpoint: &str,
    ) -> Result<String, String>;
}
```

- [ ] **Step 2: 添加 mod 声明到 lib.rs**

在 `src-tauri/src/lib.rs` 的 mod 声明区域添加：

```rust
mod llm_client;
```

- [ ] **Step 3: 添加 async-trait 到 Cargo.toml dependencies**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 中添加：

```toml
async-trait = "0.1"
```

- [ ] **Step 4: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: 编译成功

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/llm_client.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: define LlmClient trait with mockall support"
```

---

### Task 2: 实现 RealLlmClient

**Files:**
- Create: `src-tauri/src/real_llm_client.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建 src-tauri/src/real_llm_client.rs**

```rust
use crate::ai::{send_audio_prompt_from_bytes, send_text_prompt, AiError, ThinkingMode};
use crate::config::ProviderConfig;
use crate::llm_client::LlmClient;
use crate::logger::Logger;
use std::sync::{Arc, Mutex};

type VertexClientCache = Arc<Mutex<Option<rig_vertexai::Client>>>;

pub struct RealLlmClient<'a> {
    logger: &'a Logger,
    vertex_cache: &'a VertexClientCache,
}

impl<'a> RealLlmClient<'a> {
    pub fn new(logger: &'a Logger, vertex_cache: &'a VertexClientCache) -> Self {
        Self { logger, vertex_cache }
    }
}

#[async_trait::async_trait]
impl LlmClient for RealLlmClient<'_> {
    async fn send_text(
        &self,
        prompt: &str,
        model_name: &str,
        provider_id: &str,
        endpoint: &str,
    ) -> Result<String, String> {
        let provider = ProviderConfig {
            id: provider_id.to_string(),
            provider_type: provider_id.to_string(),
            name: provider_id.to_string(),
            endpoint: endpoint.to_string(),
            api_key: None,
            models: vec![],
        };
        send_text_prompt(
            self.logger,
            prompt,
            model_name,
            &provider,
            self.vertex_cache,
            ThinkingMode::Disabled,
        )
        .await
        .map_err(|e| e.to_string())
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
        let provider = ProviderConfig {
            id: provider_id.to_string(),
            provider_type: provider_id.to_string(),
            name: provider_id.to_string(),
            endpoint: endpoint.to_string(),
            api_key: None,
            models: vec![],
        };
        send_audio_prompt_from_bytes(
            self.logger,
            audio_bytes,
            media_type,
            text_prompt,
            model_name,
            &provider,
            self.vertex_cache,
        )
        .await
        .map_err(|e| e.to_string())
    }
}
```

- [ ] **Step 2: 添加 mod 声明到 lib.rs**

```rust
mod real_llm_client;
```

- [ ] **Step 3: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: 编译成功

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/real_llm_client.rs src-tauri/src/lib.rs
git commit -m "feat: implement RealLlmClient wrapping existing ai module"
```

---

### Task 3: 重构 skills.rs 接收 LlmClient 参数

**Files:**
- Modify: `src-tauri/src/skills.rs`

- [ ] **Step 1: 添加新的 process_with_skills_client 函数**

在 `skills.rs` 中添加一个新函数，接收 `&dyn LlmClient`：

```rust
use crate::llm_client::LlmClient;

pub async fn process_with_skills_client(
    logger: &Logger,
    skills_config: &SkillsConfig,
    transcription_config: &crate::config::TranscriptionConfig,
    providers: &[ProviderConfig],
    transcription: &str,
    client: &dyn LlmClient,
    selected_text: Option<&str>,
) -> Result<String, String> {
    if !skills_config.enabled {
        return Ok(transcription.to_string());
    }

    let enabled_skills: Vec<&Skill> = skills_config
        .skills
        .iter()
        .filter(|s| s.enabled)
        .collect();

    if enabled_skills.is_empty() {
        logger.info("skills", "没有启用的 Skill，跳过处理", None);
        return Ok(transcription.to_string());
    }

    if transcription.is_empty() {
        logger.info("skills", "转写文字为空，跳过处理", None);
        return Ok(transcription.to_string());
    }

    if transcription_config.polish_provider_id.is_empty() || transcription_config.polish_model.is_empty() {
        logger.warn("skills", "Skills Provider 未配置，跳过处理", None);
        return Ok(transcription.to_string());
    }

    let provider = match providers
        .iter()
        .find(|p| p.id == transcription_config.polish_provider_id)
    {
        Some(p) => p,
        None => {
            logger.warn(
                "skills",
                "未找到 Skills Provider，回退原始文字",
                Some(serde_json::json!({
                    "provider_id": transcription_config.polish_provider_id,
                })),
            );
            return Ok(transcription.to_string());
        }
    };

    let skills_owned: Vec<Skill> = enabled_skills.into_iter().cloned().collect();
    let (app_name, bundle_id) = get_frontmost_app().unwrap_or(("Unknown".to_string(), "unknown".to_string()));
    let (system_prompt, user_message) =
        assemble_skills_prompt(&skills_owned, transcription, &app_name, &bundle_id, selected_text);

    let full_prompt = format!("{}\n\n输入文本：\n{}", system_prompt, user_message);

    let timeout_secs = SKILLS_BASE_TIMEOUT_SECS
        + (skills_owned.len() as u64) * SKILLS_PER_SKILL_TIMEOUT_SECS;

    let start = std::time::Instant::now();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        client.send_text(&full_prompt, &transcription_config.polish_model, &provider.id, &provider.endpoint),
    )
    .await;

    let elapsed_ms = start.elapsed().as_millis();

    match result {
        Ok(Ok(text)) => {
            logger.info("skills", "LLM 调用成功", Some(serde_json::json!({
                "elapsed_ms": elapsed_ms,
                "original_length": transcription.len(),
                "processed_length": text.len(),
            })));
            Ok(text)
        }
        Ok(Err(e)) => {
            logger.error("skills", "LLM 调用失败，回退原始文字", Some(serde_json::json!({
                "elapsed_ms": elapsed_ms, "error": e,
            })));
            Ok(transcription.to_string())
        }
        Err(_) => {
            logger.error("skills", "LLM 调用超时，回退原始文字", Some(serde_json::json!({
                "elapsed_ms": elapsed_ms, "timeout_secs": timeout_secs,
            })));
            Ok(transcription.to_string())
        }
    }
}
```

- [ ] **Step 2: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/skills.rs
git commit -m "refactor: add process_with_skills_client accepting LlmClient trait"
```

---

### Task 4: 重构 translation.rs 接收 LlmClient 参数

**Files:**
- Modify: `src-tauri/src/translation.rs`

- [ ] **Step 1: 添加新的 translate_text_client 函数**

```rust
use crate::llm_client::LlmClient;

pub async fn translate_text_client(
    logger: &Logger,
    text: &str,
    target_lang: &str,
    skills_config: &SkillsConfig,
    provider_id: &str,
    model_name: &str,
    endpoint: &str,
    client: &dyn LlmClient,
) -> Result<String, String> {
    let mut system_prompt = TRANSLATION_BASE_PROMPT.replace("{target_lang}", target_lang);

    if let Some(skill_prompt) = get_translation_skill_prompt(skills_config) {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&skill_prompt);
    }

    let full_prompt = format!("{}\n\n{}", system_prompt, text);

    logger.info(
        "translation",
        "翻译开始",
        Some(serde_json::json!({
            "target_lang": target_lang,
            "provider_id": provider_id,
            "model_name": model_name,
            "text_length": text.len(),
        })),
    );

    let start = std::time::Instant::now();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(TRANSLATION_TIMEOUT_SECS),
        client.send_text(&full_prompt, model_name, provider_id, endpoint),
    )
    .await;

    let elapsed_ms = start.elapsed().as_millis();

    match result {
        Ok(Ok(translated)) => {
            logger.info(
                "translation",
                "翻译成功",
                Some(serde_json::json!({
                    "elapsed_ms": elapsed_ms,
                    "original_length": text.len(),
                    "translated_length": translated.len(),
                })),
            );
            Ok(translated)
        }
        Ok(Err(e)) => {
            logger.error(
                "translation",
                "翻译失败",
                Some(serde_json::json!({ "elapsed_ms": elapsed_ms, "error": e })),
            );
            Err(format!("翻译失败: {}", e))
        }
        Err(_) => {
            logger.error(
                "translation",
                "翻译超时",
                Some(serde_json::json!({ "elapsed_ms": elapsed_ms })),
            );
            Err(format!("翻译超时 ({}s)", TRANSLATION_TIMEOUT_SECS))
        }
    }
}
```

- [ ] **Step 2: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/translation.rs
git commit -m "refactor: add translate_text_client accepting LlmClient trait"
```

---

### Task 5: 编写 Skills 集成测试

**Files:**
- Modify: `src-tauri/src/skills.rs`

- [ ] **Step 1: 在 skills.rs 末尾添加测试模块**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_client::MockLlmClient;
    use crate::config::{Skill, SkillsConfig, TranscriptionConfig};

    fn enabled_skills_config() -> SkillsConfig {
        SkillsConfig {
            enabled: true,
            skills: vec![
                Skill {
                    id: "builtin-fillers".to_string(),
                    name: "语气词剔除".to_string(),
                    description: "去除语气词".to_string(),
                    prompt: "去除语气词".to_string(),
                    builtin: true,
                    editable: false,
                    enabled: true,
                },
            ],
        }
    }

    fn test_transcription_config() -> TranscriptionConfig {
        TranscriptionConfig {
            provider_id: "test-provider".to_string(),
            model: "test-model".to_string(),
            polish_enabled: true,
            polish_provider_id: "test-provider".to_string(),
            polish_model: "test-model".to_string(),
        }
    }

    fn test_providers() -> Vec<ProviderConfig> {
        vec![ProviderConfig {
            id: "test-provider".to_string(),
            provider_type: "openai-compatible".to_string(),
            name: "Test".to_string(),
            endpoint: "https://example.com/v1".to_string(),
            api_key: Some("sk-test".to_string()),
            models: vec![],
        }]
    }

    fn create_test_logger() -> Logger {
        let dir = tempfile::tempdir().unwrap();
        Logger::new(dir.path()).unwrap()
    }

    #[tokio::test]
    async fn test_process_with_skills_returns_original_when_disabled() {
        let logger = create_test_logger();
        let mut config = enabled_skills_config();
        config.enabled = false;
        let tc = test_transcription_config();
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "你好世界", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "你好世界");
    }

    #[tokio::test]
    async fn test_process_with_skills_returns_original_when_empty() {
        let logger = create_test_logger();
        let config = enabled_skills_config();
        let tc = test_transcription_config();
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "");
    }

    #[tokio::test]
    async fn test_process_with_skills_returns_original_when_no_skills_enabled() {
        let logger = create_test_logger();
        let config = SkillsConfig { enabled: true, skills: vec![] };
        let tc = test_transcription_config();
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "你好世界", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "你好世界");
    }

    #[tokio::test]
    async fn test_process_with_skills_returns_original_when_no_provider_configured() {
        let logger = create_test_logger();
        let config = enabled_skills_config();
        let tc = TranscriptionConfig {
            provider_id: "".to_string(),
            model: "".to_string(),
            polish_enabled: true,
            polish_provider_id: "".to_string(),
            polish_model: "".to_string(),
        };
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "你好世界", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "你好世界");
    }

    #[tokio::test]
    async fn test_process_with_skills_returns_original_when_provider_not_found() {
        let logger = create_test_logger();
        let config = enabled_skills_config();
        let tc = TranscriptionConfig {
            provider_id: "nonexistent".to_string(),
            model: "test-model".to_string(),
            polish_enabled: true,
            polish_provider_id: "nonexistent".to_string(),
            polish_model: "test-model".to_string(),
        };
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "你好世界", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "你好世界");
    }

    #[tokio::test]
    async fn test_process_with_skills_calls_llm_and_returns_result() {
        let logger = create_test_logger();
        let config = enabled_skills_config();
        let tc = test_transcription_config();
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        mock.expect_send_text()
            .returning(|_, _, _, _| Ok("处理后的文本".to_string()));

        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "嗯那个你好世界啊", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "处理后的文本");
    }

    #[tokio::test]
    async fn test_process_with_skills_falls_back_on_llm_error() {
        let logger = create_test_logger();
        let config = enabled_skills_config();
        let tc = test_transcription_config();
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        mock.expect_send_text()
            .returning(|_, _, _, _| Err("LLM error".to_string()));

        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "你好世界", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "你好世界");
    }

    #[test]
    fn test_assemble_skills_prompt_includes_skill_name() {
        let skills = vec![Skill {
            id: "test".to_string(),
            name: "测试技能".to_string(),
            description: "".to_string(),
            prompt: "测试 prompt".to_string(),
            builtin: false,
            editable: true,
            enabled: true,
        }];
        let (system, user) = assemble_skills_prompt(&skills, "你好", "App", "com.app", None);
        assert!(system.contains("测试技能"));
        assert!(system.contains("测试 prompt"));
        assert_eq!(user, "你好");
    }

    #[test]
    fn test_assemble_skills_prompt_includes_app_context() {
        let skills = vec![];
        let (system, _) = assemble_skills_prompt(&skills, "你好", "Finder", "com.apple.finder", Some("选中文本"));
        assert!(system.contains("Finder"));
        assert!(system.contains("com.apple.finder"));
        assert!(system.contains("选中文本"));
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib skills::tests`

Expected: 全部通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/skills.rs
git commit -m "test: add skills integration tests with mock LlmClient"
```

---

### Task 6: 编写 Translation 集成测试

**Files:**
- Modify: `src-tauri/src/translation.rs`

- [ ] **Step 1: 在 translation.rs 末尾添加测试模块**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_client::MockLlmClient;
    use crate::config::{Skill, SkillsConfig};

    fn create_test_logger() -> Logger {
        let dir = tempfile::tempdir().unwrap();
        Logger::new(dir.path()).unwrap()
    }

    fn skills_with_translation() -> SkillsConfig {
        SkillsConfig {
            enabled: true,
            skills: vec![Skill {
                id: "builtin-translation".to_string(),
                name: "翻译优化".to_string(),
                description: "".to_string(),
                prompt: "保持原文语气".to_string(),
                builtin: true,
                editable: true,
                enabled: true,
            }],
        }
    }

    #[tokio::test]
    async fn test_translate_text_success() {
        let logger = create_test_logger();
        let skills = skills_with_translation();

        let mut mock = MockLlmClient::new();
        mock.expect_send_text()
            .returning(|_, _, _, _| Ok("Hello World".to_string()));

        let result = translate_text_client(
            &logger, "你好世界", "English", &skills,
            "test-provider", "test-model", "https://example.com/v1",
            &mut mock,
        ).await;
        assert_eq!(result.unwrap(), "Hello World");
    }

    #[tokio::test]
    async fn test_translate_text_llm_error() {
        let logger = create_test_logger();
        let skills = SkillsConfig { enabled: true, skills: vec![] };

        let mut mock = MockLlmClient::new();
        mock.expect_send_text()
            .returning(|_, _, _, _| Err("API error".to_string()));

        let result = translate_text_client(
            &logger, "你好", "English", &skills,
            "test-provider", "test-model", "https://example.com/v1",
            &mut mock,
        ).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("翻译失败"));
    }

    #[tokio::test]
    async fn test_translate_text_timeout() {
        let logger = create_test_logger();
        let skills = SkillsConfig { enabled: true, skills: vec![] };

        let mut mock = MockLlmClient::new();
        mock.expect_send_text()
            .returning(|_, _, _, _| {
                std::thread::sleep(std::time::Duration::from_secs(30));
                Ok("late".to_string())
            });

        let result = translate_text_client(
            &logger, "你好", "English", &skills,
            "test-provider", "test-model", "https://example.com/v1",
            &mut mock,
        ).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("超时"));
    }

    #[test]
    fn test_get_translation_skill_prompt_found() {
        let skills = skills_with_translation();
        let prompt = get_translation_skill_prompt(&skills);
        assert!(prompt.is_some());
        assert_eq!(prompt.unwrap(), "保持原文语气");
    }

    #[test]
    fn test_get_translation_skill_prompt_not_found() {
        let skills = SkillsConfig { enabled: true, skills: vec![] };
        let prompt = get_translation_skill_prompt(&skills);
        assert!(prompt.is_none());
    }

    #[test]
    fn test_get_translation_skill_prompt_disabled() {
        let skills = SkillsConfig {
            enabled: true,
            skills: vec![Skill {
                id: "builtin-translation".to_string(),
                name: "翻译优化".to_string(),
                description: "".to_string(),
                prompt: "test".to_string(),
                builtin: true,
                editable: true,
                enabled: false,
            }],
        };
        let prompt = get_translation_skill_prompt(&skills);
        assert!(prompt.is_none());
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib translation::tests`

Expected: 全部通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/translation.rs
git commit -m "test: add translation integration tests with mock LlmClient"
```

---

### Task 7: 编写 Logger 测试

**Files:**
- Modify: `src-tauri/src/logger.rs`

- [ ] **Step 1: 在 logger.rs 末尾添加测试模块**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_logger() -> Logger {
        let dir = tempfile::tempdir().unwrap();
        Logger::new(dir.path()).unwrap()
    }

    #[test]
    fn test_log_and_get_content() {
        let logger = create_logger();
        logger.info("test-module", "test message", None);
        let entries = logger.get_content(None);
        assert!(entries.len() >= 1);
        assert_eq!(entries[0].module, "test-module");
        assert_eq!(entries[0].level, "info");
        assert_eq!(entries[0].msg, "test message");
    }

    #[test]
    fn test_get_content_path_traversal_blocked() {
        let logger = create_logger();
        let entries = logger.get_content(Some("../etc/passwd"));
        assert!(entries.is_empty());
    }

    #[test]
    fn test_get_content_absolute_path_blocked() {
        let logger = create_logger();
        let entries = logger.get_content(Some("/etc/passwd"));
        assert!(entries.is_empty());
    }

    #[test]
    fn test_get_content_backslash_blocked() {
        let logger = create_logger();
        let entries = logger.get_content(Some("..\\windows\\system32"));
        assert!(entries.is_empty());
    }

    #[test]
    fn test_get_sessions() {
        let logger = create_logger();
        logger.info("test", "message", None);
        let sessions = logger.get_sessions();
        assert!(sessions.len() >= 1);
        let current = sessions.iter().find(|s| s.is_current);
        assert!(current.is_some());
        assert!(current.unwrap().entry_count >= 1);
    }

    #[test]
    fn test_log_with_meta() {
        let logger = create_logger();
        logger.warn("test", "warning with meta", Some(serde_json::json!({ "key": "value" })));
        let entries = logger.get_content(None);
        let entry = entries.iter().find(|e| e.level == "warn").unwrap();
        assert_eq!(entry.meta.as_ref().unwrap()["key"], "value");
    }

    #[test]
    fn test_cleanup_old_logs() {
        let dir = tempfile::tempdir().unwrap();
        let log_dir = dir.path().join(LOG_DIR_NAME);
        std::fs::create_dir_all(&log_dir).unwrap();

        for i in 0..15 {
            let path = log_dir.join(format!("talkshow-2026-01-{:02}_00-00-00.jsonl", i + 1));
            std::fs::write(&path, format!("{{\"ts\":\"2026-01-{:02}T00:00:00Z\",\"module\":\"t\",\"level\":\"info\",\"msg\":\"log {}\"}}\n", i + 1, i + 1)).unwrap();
            let mtime = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs((15 - i) as u64 * 86400);
            filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(mtime)).unwrap();
        }

        cleanup_old_logs(&log_dir, 10);
        let remaining: Vec<_> = std::fs::read_dir(&log_dir).unwrap().flatten().collect();
        assert_eq!(remaining.len(), 10);
    }
}
```

注意：`test_cleanup_old_logs` 需要添加 `filetime` 到 dev-dependencies。如果不想引入额外依赖，可以简化这个测试或删除它。

**备选方案（不引入 filetime）：** 删除 `test_cleanup_old_logs` 测试，或将其标记为 `#[ignore]`。

- [ ] **Step 2: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib logger::tests`

Expected: 全部通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/logger.rs
git commit -m "test: add logger tests (path traversal, sessions, content)"
```

---

### Task 8: 编写 parse_shortcut 测试

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 lib.rs 末尾添加测试模块**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shortcut_control_shift_quote() {
        let result = parse_shortcut("Control+Shift+Quote");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_shortcut_control_backslash() {
        let result = parse_shortcut("Control+Backslash");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_shortcut_single_key() {
        let result = parse_shortcut("KeyA");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_shortcut_empty_string() {
        let result = parse_shortcut("");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_shortcut_only_modifiers() {
        let result = parse_shortcut("Control+Shift");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_shortcut_command_super_alias() {
        let cmd_result = parse_shortcut("Command+KeyA");
        let super_result = parse_shortcut("Super+KeyA");
        assert_eq!(cmd_result.is_some(), super_result.is_some());
    }

    #[test]
    fn test_format_elapsed() {
        let start = Instant::now();
        assert!(format_elapsed(&start).contains("录音中"));
    }

    #[test]
    fn test_recording_mode_constants() {
        assert_eq!(RECORDING_MODE_NONE, 0);
        assert_eq!(RECORDING_MODE_TRANSCRIPTION, 1);
        assert_eq!(RECORDING_MODE_TRANSLATION, 2);
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tests`

Expected: 全部通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "test: add parse_shortcut and lib.rs utility tests"
```

---

### Task 9: 验证全部测试通过

- [ ] **Step 1: 运行全部后端测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`

Expected: 全部通过（包含 Phase 1 和 Phase 2 测试）

- [ ] **Step 2: 运行全部前端测试**

Run: `npx vitest run`

Expected: 全部通过

- [ ] **Step 3: 最终 Commit**

```bash
git add -A
git commit -m "test: phase 2 complete - trait abstraction + P1 integration tests"
```
