//! 历史记录与回溯分析。
//! 流水线完成后追加 history.jsonl；回溯分析用 LLM 找疑点并给出术语表建议。

use crate::ai::{self, ThinkingMode};
use crate::config::ProviderConfig;
use crate::logger::Logger;
use crate::providers::ProviderContext;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const HISTORY_FILE: &str = "history.jsonl";
const ANALYZE_TIMEOUT_SECS: u64 = 120;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct HistoryEntry {
    /// ISO 时间戳
    pub ts: String,
    /// "transcription" | "translation"
    pub mode: String,
    /// 原始转写文本（skills 处理前）
    pub raw_text: String,
    /// 最终输出文本（翻译/润色后）
    pub final_text: String,
    /// 音频文件路径（保留供二期音频复核）
    pub audio_path: String,
}

fn history_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(HISTORY_FILE)
}

pub fn append(app_data_dir: &Path, entry: &HistoryEntry) -> Result<(), String> {
    use std::io::Write;
    let path = history_path(app_data_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut line = serde_json::to_string(entry).map_err(|e| e.to_string())?;
    line.push('\n');
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    f.write_all(line.as_bytes()).map_err(|e| e.to_string())
}

/// 读取最近 n 条（按文件顺序，返回时最新在前）
pub fn load_recent(app_data_dir: &Path, n: usize) -> Vec<HistoryEntry> {
    let content = std::fs::read_to_string(history_path(app_data_dir)).unwrap_or_default();
    let mut entries: Vec<HistoryEntry> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    if entries.len() > n {
        entries.drain(..entries.len() - n);
    }
    entries.reverse();
    entries
}

// ---------- 回溯分析 ----------

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Suspect {
    pub snippet: String,
    pub issue: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct GlossarySuggestion {
    pub wrong: String,
    pub correct: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct AnalyzeResult {
    pub suspects: Vec<Suspect>,
    pub glossary_suggestions: Vec<GlossarySuggestion>,
}

const ANALYZE_PROMPT: &str = r#"你是语音转写质量审查员。下面是用户近期的语音转写历史记录（JSON 数组，每条含原始转写 raw_text 和最终输出 final_text）。

请分析两类问题：

1. suspects（疑点）：转写可能错误的地方——前后不连贯、可疑的专有名词形态、同音错别字、翻译中的术语错误。每条引用原文片段并说明问题。最多 10 条，只列较有把握的。

2. glossary_suggestions（术语表建议）：疑似反复被错误转写的专有名词，给出 wrong（错误形态）→ correct（正确写法）的猜测。只在有一定把握时给出，宁缺毋滥。

只输出 JSON，不要任何解释或 markdown 代码块标记，格式：
{"suspects":[{"snippet":"...","issue":"..."}],"glossary_suggestions":[{"wrong":"...","correct":"..."}]}

历史记录：
"#;

/// 从 LLM 返回文本中提取 JSON（容忍 ```json 包裹或前后杂文）
pub fn extract_json(text: &str) -> Option<&str> {
    let t = text.trim();
    if let Some(start) = t.find('{')
        && let Some(end_rel) = t[start..].rfind('}')
    {
        return Some(&t[start..start + end_rel + 1]);
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub async fn analyze_history(
    logger: &Logger,
    app_data_dir: &Path,
    n: usize,
    provider_id: &str,
    model_name: &str,
    providers: &[ProviderConfig],
    vertex_cache: &ProviderContext,
) -> Result<AnalyzeResult, String> {
    let entries = load_recent(app_data_dir, n);
    if entries.is_empty() {
        return Err("没有历史记录可分析".to_string());
    }

    // 只送文本字段，去掉 audio_path 噪音
    let slim: Vec<_> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "mode": e.mode,
                "raw_text": e.raw_text,
                "final_text": e.final_text,
            })
        })
        .collect();
    let history_json = serde_json::to_string(&slim).map_err(|e| e.to_string())?;

    let prompt = format!("{}{}", ANALYZE_PROMPT, history_json);
    let provider = providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("分析模型提供商未找到: {}", provider_id))?;

    logger.info(
        "review",
        "回溯分析开始",
        Some(serde_json::json!({
            "entries": entries.len(),
            "provider_id": provider_id,
            "model": model_name,
        })),
    );

    let raw = tokio::time::timeout(
        std::time::Duration::from_secs(ANALYZE_TIMEOUT_SECS),
        ai::send_text_prompt(
            logger,
            &prompt,
            model_name,
            provider,
            vertex_cache,
            ThinkingMode::Disabled,
        ),
    )
    .await
    .map_err(|_| format!("回溯分析超时 ({}s)", ANALYZE_TIMEOUT_SECS))?
    .map_err(|e| format!("回溯分析失败: {}", e))?;

    let json_str = extract_json(&raw).ok_or_else(|| {
        logger.error(
            "review",
            "LLM 返回中未找到 JSON",
            Some(serde_json::json!({ "raw_preview": raw.chars().take(200).collect::<String>() })),
        );
        "LLM 返回格式异常，未找到 JSON".to_string()
    })?;

    let result: AnalyzeResult =
        serde_json::from_str(json_str).map_err(|e| format!("分析结果解析失败: {}", e))?;

    logger.info(
        "review",
        "回溯分析完成",
        Some(serde_json::json!({
            "suspects": result.suspects.len(),
            "suggestions": result.glossary_suggestions.len(),
        })),
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("talkshow-hist-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for i in 0..5 {
            append(
                &dir,
                &HistoryEntry {
                    ts: format!("2026-01-0{}", i),
                    mode: "transcription".into(),
                    raw_text: format!("raw{}", i),
                    final_text: format!("final{}", i),
                    audio_path: String::new(),
                },
            )
            .unwrap();
        }
        let all = load_recent(&dir, 10);
        assert_eq!(all.len(), 5);
        // 最新在前
        assert_eq!(all[0].final_text, "final4");
        assert_eq!(all[4].raw_text, "raw0");
        let recent = load_recent(&dir, 2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].final_text, "final4");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_json_tolerates_wrappers() {
        assert_eq!(extract_json(r#"前置杂文 {"a":1} 后置"#), Some(r#"{"a":1}"#));
        assert_eq!(extract_json("```json\n{\"a\":1}\n```"), Some("{\"a\":1}"));
        assert_eq!(extract_json("没有 json"), None);
    }
}
