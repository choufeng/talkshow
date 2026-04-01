# Phase 1: 测试基础设施搭建 + P0 纯函数测试

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 搭建前后端测试基础设施，并覆盖所有零外部依赖的纯函数模块。

**Architecture:** 前端使用 Vitest + @testing-library/svelte + jsdom，测试文件与源文件同目录放置。后端使用 Cargo 内置测试框架 + mockall + tempfile，单元测试放在 `#[cfg(test)]` 模块中。

**Tech Stack:** Vitest, @testing-library/svelte, @testing-library/jest-dom, jsdom, Cargo test, mockall, tempfile

---

### Task 1: 安装前端测试依赖

**Files:**
- Modify: `package.json`

- [ ] **Step 1: 安装依赖**

Run: `npm install -D vitest @testing-library/svelte @testing-library/jest-dom jsdom`

Expected: 安装成功，无错误

- [ ] **Step 2: 验证安装**

Run: `npx vitest --version`

Expected: 输出 vitest 版本号

- [ ] **Step 3: Commit**

```bash
git add package.json package-lock.json
git commit -m "chore: add frontend test dependencies (vitest, testing-library, jsdom)"
```

---

### Task 2: 创建 Vitest 配置

**Files:**
- Create: `vitest.config.ts`

- [ ] **Step 1: 创建 vitest.config.ts**

```typescript
import { defineConfig } from 'vitest/config';
import { sveltekit } from '@sveltejs/kit/vite';

export default defineConfig({
  plugins: [sveltekit()],
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,js}'],
    setupFiles: [],
  },
});
```

- [ ] **Step 2: 添加 test 脚本到 package.json**

在 `package.json` 的 `scripts` 中添加：

```json
"test": "vitest run",
"test:watch": "vitest"
```

- [ ] **Step 3: 验证配置**

Run: `npx vitest run`

Expected: 输出 "No test files found"（因为还没有测试文件），无配置错误

- [ ] **Step 4: Commit**

```bash
git add vitest.config.ts package.json
git commit -m "chore: add vitest config and test scripts"
```

---

### Task 3: 添加 Rust dev-dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 添加 [dev-dependencies]**

在 `src-tauri/Cargo.toml` 末尾添加：

```toml
[dev-dependencies]
mockall = "0.13"
tempfile = "3"
```

- [ ] **Step 2: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml --tests`

Expected: 编译成功，无错误

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore: add Rust test dev-dependencies (mockall, tempfile)"
```

---

### Task 4: 前端 — format 工具函数测试

**Files:**
- Create: `src/lib/utils/format.test.ts`
- Test: `src/lib/utils/format.ts`

- [ ] **Step 1: 编写 formatTime 测试**

```typescript
import { describe, it, expect } from 'vitest';
import { formatTime, formatTimestamp, formatDate } from './format';

describe('formatTime', () => {
  it('formats 0 seconds as 00:00', () => {
    expect(formatTime(0)).toBe('00:00');
  });

  it('formats seconds only', () => {
    expect(formatTime(5)).toBe('00:05');
    expect(formatTime(59)).toBe('00:59');
  });

  it('formats minutes and seconds', () => {
    expect(formatTime(60)).toBe('01:00');
    expect(formatTime(90)).toBe('01:30');
    expect(formatTime(3599)).toBe('59:59');
  });

  it('formats large values', () => {
    expect(formatTime(3600)).toBe('60:00');
    expect(formatTime(3661)).toBe('61:01');
  });
});
```

- [ ] **Step 2: 编写 formatTimestamp 测试**

在同一个文件中追加：

```typescript
describe('formatTimestamp', () => {
  it('formats a valid ISO timestamp', () => {
    const result = formatTimestamp('2026-04-01T12:30:45.000Z');
    expect(result).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
  });

  it('returns original string for invalid input', () => {
    expect(formatTimestamp('not-a-date')).toBe('not-a-date');
  });

  it('returns original string for empty string', () => {
    expect(formatTimestamp('')).toBe('');
  });
});
```

- [ ] **Step 3: 编写 formatDate 测试**

在同一个文件中追加：

```typescript
describe('formatDate', () => {
  it('formats a valid ISO date string', () => {
    const result = formatDate('2026-04-01T00:00:00.000Z');
    expect(result).toBeTruthy();
    expect(result.length).toBeGreaterThan(0);
  });

  it('returns empty string for invalid input', () => {
    expect(formatDate('not-a-date')).toBe('');
  });

  it('returns empty string for empty string', () => {
    expect(formatDate('')).toBe('');
  });
});
```

- [ ] **Step 4: 运行测试**

Run: `npx vitest run src/lib/utils/format.test.ts`

Expected: 全部通过

- [ ] **Step 5: Commit**

```bash
git add src/lib/utils/format.test.ts
git commit -m "test: add format utility tests"
```

---

### Task 5: 前端 — string 工具函数测试

**Files:**
- Create: `src/lib/utils/string.test.ts`
- Test: `src/lib/utils/string.ts`

- [ ] **Step 1: 编写 generateSlug 测试**

```typescript
import { describe, it, expect } from 'vitest';
import { generateSlug } from './string';

describe('generateSlug', () => {
  it('converts simple names', () => {
    expect(generateSlug('My Provider')).toBe('my-provider');
  });

  it('handles special characters', () => {
    expect(generateSlug('Hello, World!')).toBe('hello-world');
  });

  it('handles multiple spaces', () => {
    expect(generateSlug('  foo   bar  ')).toBe('foo-bar');
  });

  it('handles Chinese characters', () => {
    expect(generateSlug('阿里云')).toBe('');
  });

  it('handles empty string', () => {
    expect(generateSlug('')).toBe('');
  });

  it('handles already slug-like input', () => {
    expect(generateSlug('my-provider')).toBe('my-provider');
  });

  it('handles mixed alphanumeric', () => {
    expect(generateSlug('Provider 123')).toBe('provider-123');
  });
});
```

- [ ] **Step 2: 运行测试**

Run: `npx vitest run src/lib/utils/string.test.ts`

Expected: 全部通过

- [ ] **Step 3: Commit**

```bash
git add src/lib/utils/string.test.ts
git commit -m "test: add string utility tests"
```

---

### Task 6: 前端 — config 更新工具测试

**Files:**
- Create: `src/lib/ai/shared/config.test.ts`
- Test: `src/lib/ai/shared/config.ts`

- [ ] **Step 1: 编写 updateFeature 测试**

```typescript
import { describe, it, expect } from 'vitest';
import { updateFeature, updateNestedPath } from './config';
import type { AppConfig, FeaturesConfig } from '$lib/stores/config';

function createTestConfig(): AppConfig {
  return {
    shortcut: 'Control+Shift+Quote',
    recording_shortcut: 'Control+Backslash',
    translate_shortcut: 'Control+Shift+T',
    ai: { providers: [] },
    features: {
      transcription: {
        provider_id: 'vertex',
        model: 'gemini-2.0-flash',
        polish_enabled: true,
        polish_provider_id: '',
        polish_model: '',
      },
      translation: { target_lang: 'English' },
      skills: { enabled: true, skills: [] },
      recording: { auto_mute: false },
    },
  };
}

describe('updateFeature', () => {
  it('updates a nested feature without mutating original', () => {
    const config = createTestConfig();
    const updated = updateFeature(config, 'recording', (r) => ({
      ...r,
      auto_mute: true,
    }));
    expect(updated.features.recording.auto_mute).toBe(true);
    expect(config.features.recording.auto_mute).toBe(false);
  });

  it('preserves other features', () => {
    const config = createTestConfig();
    const updated = updateFeature(config, 'translation', (t) => ({
      ...t,
      target_lang: '中文',
    }));
    expect(updated.features.translation.target_lang).toBe('中文');
    expect(updated.features.recording.auto_mute).toBe(false);
    expect(updated.features.skills.enabled).toBe(true);
  });
});
```

- [ ] **Step 2: 编写 updateNestedPath 测试**

在同一个文件中追加：

```typescript
describe('updateNestedPath', () => {
  it('updates a shallow path', () => {
    const obj = { a: 1, b: 2 };
    const updated = updateNestedPath(obj, ['a'], () => 10);
    expect(updated.a).toBe(10);
    expect(updated.b).toBe(2);
    expect(obj.a).toBe(1);
  });

  it('updates a deep path', () => {
    const obj = { a: { b: { c: 'old' } } };
    const updated = updateNestedPath(obj, ['a', 'b', 'c'], () => 'new');
    expect(updated.a.b.c).toBe('new');
    expect(obj.a.b.c).toBe('old');
  });

  it('preserves sibling keys at each level', () => {
    const obj = { a: { b: 1, c: 2 }, x: 3 };
    const updated = updateNestedPath(obj, ['a', 'b'], () => 10);
    expect(updated.a.b).toBe(10);
    expect(updated.a.c).toBe(2);
    expect(updated.x).toBe(3);
  });
});
```

- [ ] **Step 3: 运行测试**

Run: `npx vitest run src/lib/ai/shared/config.test.ts`

Expected: 全部通过

- [ ] **Step 4: Commit**

```bash
git add src/lib/ai/shared/config.test.ts
git commit -m "test: add config update utility tests"
```

---

### Task 7: Rust — recording 纯函数测试

**Files:**
- Modify: `src-tauri/src/recording.rs`

- [ ] **Step 1: 在 recording.rs 末尾添加测试模块**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_leap_year() {
        assert!(!is_leap_year(2025));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(2100));
        assert!(is_leap_year(0));
        assert!(is_leap_year(4));
    }

    #[test]
    fn test_days_to_date_epoch() {
        assert_eq!(days_to_date(0), (1970, 1, 1));
    }

    #[test]
    fn test_days_to_date_2024_new_year() {
        let days_from_epoch: u64 = 19723;
        let (year, month, day) = days_to_date(days_from_epoch);
        assert_eq!(year, 2024);
        assert_eq!(month, 1);
        assert_eq!(day, 1);
    }

    #[test]
    fn test_days_to_date_leap_year_feb() {
        let days_from_epoch: u64 = 19723 + 31 + 28;
        let (year, month, day) = days_to_date(days_from_epoch);
        assert_eq!(year, 2024);
        assert_eq!(month, 2);
        assert_eq!(day, 29);
    }

    #[test]
    fn test_days_to_date_year_boundary() {
        let days_in_2023: u64 = 365;
        let days_2023_start: u64 = 19358;
        let (_, month, day) = days_to_date(days_2023_start);
        assert_eq!(month, 1);
        assert_eq!(day, 1);

        let (_, month, day) = days_to_date(days_2023_start + days_in_2023 - 1);
        assert_eq!(month, 12);
        assert_eq!(day, 31);
    }

    #[test]
    fn test_generate_filename_format() {
        let filename = generate_filename();
        assert!(filename.starts_with("talkshow_"));
        assert!(filename.ends_with(".flac"));
        let parts: Vec<&str> = filename.strip_prefix("talkshow_").unwrap().strip_suffix(".flac").unwrap().split('_').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 6);
    }

    #[test]
    fn test_recordings_dir() {
        let dir = recordings_dir();
        assert!(dir.ends_with("talkshow"));
    }

    #[test]
    fn test_ensure_recordings_dir() {
        let dir = ensure_recordings_dir().unwrap();
        assert!(dir.exists());
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib recording::tests`

Expected: 全部通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/recording.rs
git commit -m "test: add recording pure function tests (days_to_date, is_leap_year, generate_filename)"
```

---

### Task 8: Rust — config 迁移逻辑测试

**Files:**
- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: 在 config.rs 末尾添加测试模块**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_providers() -> Vec<ProviderConfig> {
        vec![
            ProviderConfig {
                id: "vertex".to_string(),
                provider_type: "vertex".to_string(),
                name: "Vertex AI".to_string(),
                endpoint: String::new(),
                api_key: None,
                models: vec![
                    ModelConfig { name: "gemini-2.0-flash".to_string(), capabilities: vec!["transcription".to_string()], verified: None },
                ],
            },
            ProviderConfig {
                id: "custom".to_string(),
                provider_type: "openai-compatible".to_string(),
                name: "Custom".to_string(),
                endpoint: "https://example.com/v1".to_string(),
                api_key: Some("sk-test".to_string()),
                models: vec![
                    ModelConfig { name: "model-a".to_string(), capabilities: vec!["transcription".to_string()], verified: None },
                    ModelConfig { name: "model-a".to_string(), capabilities: vec!["chat".to_string()], verified: None },
                ],
            },
        ]
    }

    #[test]
    fn test_dedup_models_removes_duplicate_and_merges_capabilities() {
        let mut models = vec![
            ModelConfig { name: "model-a".to_string(), capabilities: vec!["transcription".to_string()], verified: None },
            ModelConfig { name: "model-a".to_string(), capabilities: vec!["chat".to_string()], verified: None },
            ModelConfig { name: "model-b".to_string(), capabilities: vec!["transcription".to_string()], verified: None },
        ];
        dedup_models(&mut models);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].name, "model-a");
        assert!(models[0].capabilities.contains(&"transcription".to_string()));
        assert!(models[0].capabilities.contains(&"chat".to_string()));
        assert_eq!(models[1].name, "model-b");
    }

    #[test]
    fn test_dedup_models_no_duplicates() {
        let mut models = vec![
            ModelConfig { name: "a".to_string(), capabilities: vec!["t".to_string()], verified: None },
            ModelConfig { name: "b".to_string(), capabilities: vec!["c".to_string()], verified: None },
        ];
        dedup_models(&mut models);
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn test_dedup_models_empty() {
        let mut models: Vec<ModelConfig> = vec![];
        dedup_models(&mut models);
        assert!(models.is_empty());
    }

    #[test]
    fn test_migrate_models_string_to_object() {
        let mut value = serde_json::json!({
            "ai": {
                "providers": [
                    {
                        "id": "test",
                        "type": "openai-compatible",
                        "name": "Test",
                        "endpoint": "",
                        "models": ["old-model-1", "old-model-2"]
                    }
                ]
            }
        });
        migrate_models(&mut value);
        let models = value["ai"]["providers"][0]["models"].as_array().unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0]["name"], "old-model-1");
        assert_eq!(models[0]["capabilities"], serde_json::json!([]));
        assert_eq!(models[1]["name"], "old-model-2");
    }

    #[test]
    fn test_migrate_models_object_unchanged() {
        let mut value = serde_json::json!({
            "ai": {
                "providers": [
                    {
                        "id": "test",
                        "type": "openai-compatible",
                        "name": "Test",
                        "endpoint": "",
                        "models": [{"name": "model-a", "capabilities": ["transcription"]}]
                    }
                ]
            }
        });
        migrate_models(&mut value);
        let models = value["ai"]["providers"][0]["models"].as_array().unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0]["name"], "model-a");
        assert_eq!(models[0]["capabilities"], serde_json::json!(["transcription"]));
    }

    #[test]
    fn test_migrate_builtin_skills_resets_modified_prompts() {
        let mut value = serde_json::json!({
            "features": {
                "skills": {
                    "enabled": true,
                    "skills": [
                        {
                            "id": "builtin-fillers",
                            "name": "语气词剔除",
                            "prompt": "MODIFIED PROMPT",
                            "builtin": true,
                            "editable": false,
                            "enabled": true,
                        }
                    ]
                }
            }
        });
        migrate_builtin_skills(&mut value);
        let skills = value["features"]["skills"]["skills"].as_array().unwrap();
        let filler = skills.iter().find(|s| s["id"] == "builtin-fillers").unwrap();
        assert_ne!(filler["prompt"].as_str().unwrap(), "MODIFIED PROMPT");
    }

    #[test]
    fn test_migrate_builtin_skills_adds_missing_defaults() {
        let mut value = serde_json::json!({
            "features": {
                "skills": {
                    "enabled": true,
                    "skills": []
                }
            }
        });
        migrate_builtin_skills(&mut value);
        let skills = value["features"]["skills"]["skills"].as_array().unwrap();
        let ids: Vec<&str> = skills.iter().map(|s| s["id"].as_str().unwrap()).collect();
        assert!(ids.contains(&"builtin-fillers"));
        assert!(ids.contains(&"builtin-typos"));
        assert!(ids.contains(&"builtin-polish"));
        assert!(ids.contains(&"builtin-formal"));
        assert!(ids.contains(&"builtin-translation"));
    }

    #[test]
    fn test_migrate_builtin_skills_preserves_editable() {
        let mut value = serde_json::json!({
            "features": {
                "skills": {
                    "enabled": true,
                    "skills": [
                        {
                            "id": "builtin-translation",
                            "name": "翻译优化",
                            "prompt": "CUSTOM TRANSLATION PROMPT",
                            "builtin": true,
                            "editable": true,
                            "enabled": true,
                        }
                    ]
                }
            }
        });
        migrate_builtin_skills(&mut value);
        let skills = value["features"]["skills"]["skills"].as_array().unwrap();
        let translation = skills.iter().find(|s| s["id"] == "builtin-translation").unwrap();
        assert_eq!(translation["prompt"].as_str().unwrap(), "CUSTOM TRANSLATION PROMPT");
    }

    #[test]
    fn test_merge_builtin_providers_adds_missing() {
        let providers = vec![
            ProviderConfig {
                id: "custom".to_string(),
                provider_type: "openai-compatible".to_string(),
                name: "Custom".to_string(),
                endpoint: "https://example.com".to_string(),
                api_key: Some("key".to_string()),
                models: vec![],
            },
        ];
        let result = merge_builtin_providers(providers);
        let ids: Vec<&str> = result.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"vertex"));
        assert!(ids.contains(&"dashscope"));
        assert!(ids.contains(&"sensevoice"));
        assert!(ids.contains(&"custom"));
    }

    #[test]
    fn test_merge_builtin_providers_corrects_existing() {
        let providers = vec![
            ProviderConfig {
                id: "dashscope".to_string(),
                provider_type: "openai-compatible".to_string(),
                name: "阿里云".to_string(),
                endpoint: "https://wrong-url.com".to_string(),
                api_key: Some("key".to_string()),
                models: vec![],
            },
        ];
        let result = merge_builtin_providers(providers);
        let dashscope = result.iter().find(|p| p.id == "dashscope").unwrap();
        assert_eq!(dashscope.endpoint, "https://dashscope.aliyuncs.com/compatible-mode/v1");
    }

    #[test]
    fn test_load_and_save_config_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let app_data_dir = dir.path().to_path_buf();

        let config = load_config(&app_data_dir);
        assert_eq!(config.shortcut, DEFAULT_SHORTCUT);
        assert!(config.ai.providers.len() >= 3);

        config.ai.providers[0].name = "Modified".to_string();
        save_config(&app_data_dir, &config).unwrap();

        let loaded = load_config(&app_data_dir);
        assert_eq!(loaded.ai.providers[0].name, "Modified");
    }

    #[test]
    fn test_load_config_creates_default_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let app_data_dir = dir.path().to_path_buf();
        let config = load_config(&app_data_dir);
        assert_eq!(config.shortcut, DEFAULT_SHORTCUT);
        assert!(app_data_dir.join("config.json").exists());
    }

    #[test]
    fn test_default_app_config() {
        let config = AppConfig::default();
        assert_eq!(config.shortcut, "Control+Shift+Quote");
        assert_eq!(config.features.transcription.provider_id, "vertex");
        assert_eq!(config.features.skills.enabled, true);
        assert_eq!(config.features.recording.auto_mute, false);
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib config::tests`

Expected: 全部通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "test: add config migration and persistence tests"
```

---

### Task 9: Rust — sensevoice 音频处理函数测试

**Files:**
- Modify: `src-tauri/src/sensevoice.rs`

- [ ] **Step 1: 在 sensevoice.rs 末尾添加测试模块**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_parse_cmvn_valid_format() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "<AddShift>\n").unwrap();
        write!(file, "[\n").unwrap();
        write!(file, "  1.0 2.0 3.0\n").unwrap();
        write!(file, "]\n").unwrap();
        write!(file, "<Rescale>\n").unwrap();
        write!(file, "[\n").unwrap();
        write!(file, "  0.5 1.0 1.5\n").unwrap();
        write!(file, "]\n").unwrap();

        let (means, vars) = parse_cmvn(&file.path().to_path_buf()).unwrap();
        assert_eq!(means.len(), 3);
        assert_eq!(means[0], 1.0);
        assert_eq!(means[1], 2.0);
        assert_eq!(means[2], 3.0);
        assert_eq!(vars.len(), 3);
        assert_eq!(vars[0], 0.5);
    }

    #[test]
    fn test_parse_cmvn_multiline_values() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "<AddShift>\n").unwrap();
        write!(file, "[\n").unwrap();
        write!(file, "  1.0\n").unwrap();
        write!(file, "  2.0\n").unwrap();
        write!(file, "  3.0\n").unwrap();
        write!(file, "]\n").unwrap();
        write!(file, "<Rescale>\n").unwrap();
        write!(file, "[\n").unwrap();
        write!(file, "  1.0\n").unwrap();
        write!(file, "  2.0\n").unwrap();
        write!(file, "  3.0\n").unwrap();
        write!(file, "]\n").unwrap();

        let (means, vars) = parse_cmvn(&file.path().to_path_buf()).unwrap();
        assert_eq!(means.len(), 3);
        assert_eq!(vars.len(), 3);
    }

    #[test]
    fn test_postprocess_removes_special_tokens() {
        assert_eq!(postprocess("<|nospeech|>"), "");
        assert_eq!(postprocess("<|zh|><|en|>Hello"), "Hello");
        assert_eq!(postprocess("Hello <|ja|> World"), "Hello  World");
        assert_eq!(postprocess("plain text"), "plain text");
    }

    #[test]
    fn test_postprocess_trims_whitespace() {
        assert_eq!(postprocess("  <|zh|>hello  "), "hello");
        assert_eq!(postprocess(""), "");
    }

    #[test]
    fn test_apply_lfr_basic() {
        let feat = Array2::from_shape_vec((10, 80), vec![1.0f32; 10 * 80]).unwrap();
        let result = apply_lfr(&feat);
        let (t_lfr, lfr_dim) = result.dim();
        assert_eq!(lfr_dim, 80 * 7);
        assert!(t_lfr > 0);
    }

    #[test]
    fn test_apply_lfr_empty_input() {
        let feat = Array2::from_shape_vec((0, 80), vec![]).unwrap();
        let result = apply_lfr(&feat);
        assert_eq!(result.dim(), (0, 80 * 7));
    }

    #[test]
    fn test_apply_cmvn_dimensions() {
        let feat = Array2::from_shape_vec((5, 3), vec![1.0f32; 15]).unwrap();
        let means = Array1::from_vec(vec![0.0f64; 3]);
        let vars = Array1::from_vec(vec![1.0f64; 3]);
        let result = apply_cmvn(&feat, &means, &vars);
        assert_eq!(result.dim(), (1, 5, 3));
    }

    #[test]
    fn test_apply_cmvn_applies_transform() {
        let feat = Array2::from_shape_vec((1, 2), vec![0.0f32, 0.0]).unwrap();
        let means = Array1::from_vec(vec![10.0f64, 20.0f64]);
        let vars = Array1::from_vec(vec![2.0f64, 0.5f64]);
        let result = apply_cmvn(&feat, &means, &vars);
        assert_eq!(result[[0, 0, 0]], (0.0 + 10.0) * 2.0);
        assert_eq!(result[[0, 0, 1]], (0.0 + 20.0) * 0.5);
    }

    #[test]
    fn test_pad_features_returns_correct_length() {
        let feat = Array3::from_shape_vec((1, 10, 560), vec![0.0f32; 10 * 560]).unwrap();
        let (padded, len) = pad_features(&feat);
        assert_eq!(len, 10);
        assert_eq!(padded.dim(), (1, 10, 560));
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib sensevoice::tests`

Expected: 全部通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/sensevoice.rs
git commit -m "test: add sensevoice audio processing function tests"
```

---

### Task 10: 验证全部测试通过

- [ ] **Step 1: 运行全部前端测试**

Run: `npx vitest run`

Expected: 全部通过

- [ ] **Step 2: 运行全部后端测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`

Expected: 全部通过

- [ ] **Step 3: 最终 Commit（如有修复）**

```bash
git add -A
git commit -m "test: phase 1 complete - infrastructure + P0 pure function tests"
```
