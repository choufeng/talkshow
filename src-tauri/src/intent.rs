//! 语义路由:转写后由润色 LLM 调用同时识别意图,零额外 LLM 往返。
//!
//! 设计:润色 prompt 末尾追加意图路由指令,LLM 输出 JSON
//! `{"intent":"transcribe"|"translate","target_lang":..,"text":..,"result":..}`。
//! 解析失败/超时/未配置 → 回退普通转写(与关闭路由时行为一致)。

use crate::config::{
    GlossaryEntry, IntentRoutingConfig, ProviderConfig, SkillsConfig, TranscriptionConfig,
};
use crate::logger::Logger;
use crate::providers::ProviderContext;
use crate::skills;
use serde::Deserialize;

/// 合并调用比纯润色多做事(可能含翻译),加缓冲
// ponytail: 固定缓冲,若路由意图扩展(ask 等)改为按意图分级超时
const INTENT_EXTRA_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    Transcribe,
    Translate {
        target_lang: String,
    },
    /// 问询:开头说"问一下…"等,正文是问题,result 为首轮回答(可能缺失)
    Ask,
    /// 记录:开头说"记录…"等,正文整理后发送到 Hook URL
    Hook,
}

#[derive(Debug, Clone)]
pub struct IntentDecision {
    pub intent: Intent,
    /// 润色后正文(translate 时指令前缀已剥离)
    pub text: String,
    /// translate 且模型在同一次调用中完成翻译时的译文
    pub result: Option<String>,
}

impl IntentDecision {
    pub fn passthrough(text: impl Into<String>) -> Self {
        Self {
            intent: Intent::Transcribe,
            text: text.into(),
            result: None,
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawDecision {
    intent: String,
    target_lang: String,
    text: String,
    result: Option<String>,
}

/// 意图路由指令块,拼在 skills 基础 prompt 之后
pub fn intent_instruction() -> String {
    String::from(
        "\n\n【意图路由】(最高优先级指令)\n\
判断输入文本是否以明确的指令前缀开头(只看开头,句中提及不算):\n\
- \"翻译…\"、\"帮我翻译…\"、\"翻译成XX…\" 等 → 翻译意图\n\
- \"问一下…\"、\"帮我问…\"、\"问一问…\"、\"查一下…\"、\"帮我查…\" 等 → 问询意图\n\
- \"记录…\"、\"帮我记录…\"、\"记录一下…\"、\"记一下…\"、\"帮我记一下…\" 等 → 记录意图\n\n\
规则:\n\
1. 指令必须出现在文本开头才有效;文本中部或结尾出现\"翻译\"、\"问\"、\"记录\"等词不是指令\n\
2. 翻译意图时:text 剥离指令前缀;target_lang 用语言名称(如\"英文\"),用户未指明则留空字符串\n\
3. 问询意图时:text 剥离指令前缀作为问题\n\
4. 记录意图时:text 剥离指令前缀作为待记录内容\n\
5. 有歧义或不确定时,intent 用 \"transcribe\"\n\
6. 无指令时 intent 用 \"transcribe\",text 正常应用上方全部润色规则\n\
7. 陈述句、想粘贴的疑问句都不是问询意图;只有明确的问询指令前缀才算\n\n\
输出格式(严格 JSON,不要包裹代码块,不要输出 JSON 以外的任何内容):\n\
{\"intent\":\"transcribe\",\"text\":\"润色后的正文\"}\n\
或\n\
{\"intent\":\"translate\",\"target_lang\":\"英文\",\"text\":\"剥离指令后的正文\",\"result\":\"翻译结果\"}\n\
或\n\
{\"intent\":\"ask\",\"text\":\"剥离指令后的问题\"}\n\
或\n\
{\"intent\":\"hook\",\"text\":\"剥离指令后的内容\"}\n\
- translate 时必须同时给出 result:把 text 翻译为 target_lang,并应用上方全部润色规则与翻译规则\n",
    )
}

/// 从 LLM 输出中解析意图决策。任何失败 → 回退转写原文(保持旧路径行为)。
pub fn parse_llm_output(raw: &str) -> IntentDecision {
    let Some(json_str) = extract_json(raw) else {
        return IntentDecision::passthrough(raw);
    };
    let Ok(d) = serde_json::from_str::<RawDecision>(json_str) else {
        return IntentDecision::passthrough(raw);
    };

    let text = if d.text.trim().is_empty() {
        raw.to_string()
    } else {
        d.text
    };

    match d.intent.trim() {
        "translate" => {
            let lang = d.target_lang.trim().to_string();
            if lang.is_empty() {
                // ponytail: 未指明语言时回退转写,不猜默认语言;需要时用翻译快捷键的固定语言
                IntentDecision::passthrough(text)
            } else {
                IntentDecision {
                    intent: Intent::Translate { target_lang: lang },
                    text,
                    result: d.result.filter(|r| !r.trim().is_empty()),
                }
            }
        }
        "ask" => IntentDecision {
            intent: Intent::Ask,
            text,
            result: d.result.filter(|r| !r.trim().is_empty()),
        },
        "hook" => IntentDecision {
            intent: Intent::Hook,
            text,
            result: None,
        },
        _ => IntentDecision::passthrough(text),
    }
}

/// 提取 JSON 对象:剥代码围栏,取首个 '{' 到最后一个 '}' 之间的内容
fn extract_json(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(&raw[start..=end])
}

/// 语义路由入口。
/// - 路由关闭 → 委托 skills::process_with_skills(行为与旧版完全一致)
/// - 路由开启 → 单次 LLM 调用 = 润色 + 意图识别(+翻译),返回决策
#[allow(clippy::too_many_arguments)]
pub async fn process_with_intent(
    logger: &Logger,
    routing: &IntentRoutingConfig,
    skills_config: &SkillsConfig,
    transcription_config: &TranscriptionConfig,
    providers: &[ProviderConfig],
    transcription: &str,
    vertex_cache: &ProviderContext,
    glossary: &[GlossaryEntry],
) -> Result<IntentDecision, String> {
    if !routing.enabled {
        let text = skills::process_with_skills(
            logger,
            skills_config,
            transcription_config,
            providers,
            transcription,
            vertex_cache,
            glossary,
        )
        .await?;
        return Ok(IntentDecision::passthrough(text));
    }

    if transcription.is_empty() {
        return Ok(IntentDecision::passthrough(""));
    }

    let meaningful = transcription
        .chars()
        .filter(|c| c.is_alphanumeric())
        .count();
    if meaningful < 2 {
        logger.info("intent", "转写内容过短，跳过路由", None);
        return Ok(IntentDecision::passthrough(transcription));
    }

    if transcription_config.polish_provider_id.is_empty()
        || transcription_config.polish_model.is_empty()
    {
        logger.warn("intent", "润色 Provider 未配置，跳过路由", None);
        return Ok(IntentDecision::passthrough(transcription));
    }

    let Some(provider) = providers
        .iter()
        .find(|p| p.id == transcription_config.polish_provider_id)
    else {
        logger.warn(
            "intent",
            "未找到润色 Provider，跳过路由",
            Some(serde_json::json!({ "provider_id": transcription_config.polish_provider_id })),
        );
        return Ok(IntentDecision::passthrough(transcription));
    };

    let enabled_skills: Vec<&crate::config::Skill> =
        skills_config.skills.iter().filter(|s| s.enabled).collect();
    let skills_owned: Vec<crate::config::Skill> = enabled_skills.into_iter().cloned().collect();

    let (app_name, bundle_id) = tokio::task::spawn_blocking(skills::get_frontmost_app)
        .await
        .unwrap_or_else(|e| Err(format!("spawn_blocking error: {}", e)))
        .unwrap_or_else(|e| {
            logger.warn(
                "intent",
                "获取前台应用失败",
                Some(serde_json::json!({ "error": e })),
            );
            ("Unknown".to_string(), "unknown".to_string())
        });
    let (base_prompt, user_message) = skills::assemble_skills_prompt(
        &skills_owned,
        transcription,
        &app_name,
        &bundle_id,
        glossary,
    );
    let full_prompt = format!(
        "{}{}\n\n输入文本：\n{}",
        base_prompt,
        intent_instruction(),
        user_message
    );

    let timeout_secs = skills::llm_timeout_secs(skills_owned.len()) + INTENT_EXTRA_TIMEOUT_SECS;

    logger.info(
        "intent",
        "语义路由调用开始",
        Some(serde_json::json!({
            "skills_count": skills_owned.len(),
            "app_name": app_name,
            "timeout_secs": timeout_secs,
        })),
    );

    let start = std::time::Instant::now();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        crate::ai::send_text_prompt(
            logger,
            &full_prompt,
            &transcription_config.polish_model,
            provider,
            vertex_cache,
            crate::ai::ThinkingMode::Disabled,
        ),
    )
    .await;

    let elapsed_ms = start.elapsed().as_millis();
    match result {
        Ok(Ok(raw)) => {
            let decision = parse_llm_output(&raw);
            logger.info(
                "intent",
                "语义路由决策",
                Some(serde_json::json!({
                    "elapsed_ms": elapsed_ms,
                    "intent": match &decision.intent {
                        Intent::Transcribe => "transcribe".to_string(),
                        Intent::Translate { target_lang } => format!("translate:{}", target_lang),
                        Intent::Ask => "ask".to_string(),
                        Intent::Hook => "hook".to_string(),
                    },
                    "has_result": decision.result.is_some(),
                    "raw_preview": raw.chars().take(120).collect::<String>(),
                })),
            );
            Ok(decision)
        }
        Ok(Err(e)) => {
            logger.error(
                "intent",
                "LLM 调用失败，回退转写",
                Some(serde_json::json!({ "elapsed_ms": elapsed_ms, "error": e.to_string() })),
            );
            Ok(IntentDecision::passthrough(transcription))
        }
        Err(_) => {
            logger.error(
                "intent",
                "LLM 调用超时，回退转写",
                Some(serde_json::json!({ "elapsed_ms": elapsed_ms, "timeout_secs": timeout_secs })),
            );
            Ok(IntentDecision::passthrough(transcription))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_transcribe_json() {
        let d = parse_llm_output(r#"{"intent":"transcribe","text":"你好世界"}"#);
        assert_eq!(d.intent, Intent::Transcribe);
        assert_eq!(d.text, "你好世界");
        assert!(d.result.is_none());
    }

    #[test]
    fn parse_translate_with_result() {
        let d = parse_llm_output(
            r#"{"intent":"translate","target_lang":"英文","text":"你好世界","result":"Hello world"}"#,
        );
        assert_eq!(
            d.intent,
            Intent::Translate {
                target_lang: "英文".to_string()
            }
        );
        assert_eq!(d.text, "你好世界");
        assert_eq!(d.result.as_deref(), Some("Hello world"));
    }

    #[test]
    fn parse_translate_without_result_keeps_intent_for_fallback() {
        let d =
            parse_llm_output(r#"{"intent":"translate","target_lang":"英文","text":"你好世界"}"#);
        assert!(matches!(d.intent, Intent::Translate { .. }));
        assert!(d.result.is_none());
    }

    #[test]
    fn parse_translate_empty_lang_falls_back_to_transcribe() {
        let d = parse_llm_output(r#"{"intent":"translate","target_lang":"","text":"你好"}"#);
        assert_eq!(d.intent, Intent::Transcribe);
    }

    #[test]
    fn parse_fenced_json() {
        let d = parse_llm_output("```json\n{\"intent\":\"transcribe\",\"text\":\"好\"}\n```");
        assert_eq!(d.intent, Intent::Transcribe);
        assert_eq!(d.text, "好");
    }

    #[test]
    fn parse_prose_wrapped_json() {
        let d = parse_llm_output("结果如下：{\"intent\":\"transcribe\",\"text\":\"内容\"} 请查收");
        assert_eq!(d.intent, Intent::Transcribe);
        assert_eq!(d.text, "内容");
    }

    #[test]
    fn parse_invalid_falls_back_to_passthrough() {
        let d = parse_llm_output("这不是 JSON，只是润色后的文本");
        assert_eq!(d.intent, Intent::Transcribe);
        assert_eq!(d.text, "这不是 JSON，只是润色后的文本");
    }

    #[test]
    fn parse_empty_text_falls_back_to_raw() {
        let d = parse_llm_output(r#"{"intent":"transcribe","text":""}"#);
        assert_eq!(d.text, r#"{"intent":"transcribe","text":""}"#);
    }

    #[test]
    fn parse_unknown_intent_falls_back() {
        let d = parse_llm_output(r#"{"intent":"open_url","text":"你好"}"#);
        assert_eq!(d.intent, Intent::Transcribe);
        assert_eq!(d.text, "你好");
    }

    #[test]
    fn parse_translate_with_whitespace_only_result_treated_as_none() {
        let d = parse_llm_output(
            r#"{"intent":"translate","target_lang":"英文","text":"你好","result":"  "}"#,
        );
        assert!(matches!(d.intent, Intent::Translate { .. }));
        assert!(d.result.is_none());
    }

    #[test]
    fn parse_ask_with_answer() {
        let d = parse_llm_output(
            r#"{"intent":"ask","text":"蘑菇炒肉怎么做","result":"蘑菇炒肉做法如下…"}"#,
        );
        assert_eq!(d.intent, Intent::Ask);
        assert_eq!(d.text, "蘑菇炒肉怎么做");
        assert!(d.result.is_some());
    }

    #[test]
    fn parse_ask_without_answer_keeps_intent() {
        let d = parse_llm_output(r#"{"intent":"ask","text":"今天天气如何"}"#);
        assert_eq!(d.intent, Intent::Ask);
        assert!(d.result.is_none());
    }

    #[test]
    fn parse_hook_intent() {
        let d = parse_llm_output(r#"{"intent":"hook","text":"明天下午三点开会讨论Q3预算"}"#);
        assert_eq!(d.intent, Intent::Hook);
        assert_eq!(d.text, "明天下午三点开会讨论Q3预算");
        assert!(d.result.is_none());
    }

    #[test]
    fn intent_instruction_contains_schema() {
        let s = intent_instruction();
        assert!(s.contains("\"intent\""));
        assert!(s.contains("\"target_lang\""));
        assert!(s.contains("\"result\""));
        assert!(s.contains("只看开头"));
        assert!(s.contains("\"ask\""));
        assert!(s.contains("\"hook\""));
        assert!(s.contains("记录"));
    }
}
