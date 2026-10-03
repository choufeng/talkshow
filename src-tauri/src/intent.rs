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
    /// 歧义候选 kinds(仅 "hook"/"ask";已过滤主意图同名/translate/未知/被禁用)
    pub alternatives: Vec<String>,
}

impl IntentDecision {
    pub fn passthrough(text: impl Into<String>) -> Self {
        Self {
            intent: Intent::Transcribe,
            text: text.into(),
            result: None,
            alternatives: Vec::new(),
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
    alternatives: Vec<String>,
}

/// 意图路由指令块,拼在 skills 基础 prompt 之后(全部意图启用)
#[cfg(test)]
pub fn intent_instruction() -> String {
    intent_instruction_disabled(&[])
}

/// 同 [`intent_instruction`],但被禁用的意图不进入指令块;全部禁用 → 空串
/// (润色输出纯文本,parse 对纯文本天然回退 transcribe,无需路由指令)。
pub fn intent_instruction_disabled(disabled: &[String]) -> String {
    let on = |k: &str| !disabled.iter().any(|d| d == k);
    if !on("translate") && !on("ask") && !on("hook") {
        return String::new();
    }

    let mut s = String::from(
        "\n\n【意图路由】(最高优先级指令)\n\
判断输入文本是否以明确的指令前缀开头(只看开头,句中提及不算):\n",
    );
    let mut rules: Vec<String> =
        vec!["指令必须出现在文本开头才有效;文本中部或结尾出现的指令词不算".to_string()];
    let mut outputs: Vec<&str> =
        vec!["{\"intent\":\"transcribe\",\"text\":\"润色后的正文\",\"alternatives\":[]}"];
    let mut extra = String::new();

    if on("translate") {
        s += "- \"翻译…\"、\"帮我翻译…\"、\"翻译成XX…\" 等 → 翻译意图\n";
        rules.push(
            "翻译意图时:text 剥离指令前缀;target_lang 用语言名称(如\"英文\"),用户未指明则留空字符串"
                .to_string(),
        );
        outputs.push(
            "{\"intent\":\"translate\",\"target_lang\":\"英文\",\"text\":\"剥离指令后的正文\",\"result\":\"翻译结果\"}",
        );
        extra.push_str(
            "\n- translate 时必须同时给出 result:把 text 翻译为 target_lang,并应用上方全部润色规则与翻译规则",
        );
    }
    if on("ask") {
        s += "- \"问一下…\"、\"帮我问…\"、\"问一问…\"、\"查一下…\"、\"帮我查…\" 等 → 问询意图\n";
        rules.push(
            "问询意图时:text 剥离指令前缀作为问题;陈述句、想粘贴的疑问句都不是问询意图,只有明确的问询指令前缀才算"
                .to_string(),
        );
        outputs.push("{\"intent\":\"ask\",\"text\":\"剥离指令后的问题\"}");
    }
    if on("hook") {
        s += "- \"记录…\"、\"帮我记录…\"、\"记录一下…\"、\"记一下…\"、\"帮我记一下…\" 等 → 记录意图\n";
        rules.push("记录意图时:text 剥离指令前缀作为待记录内容".to_string());
        outputs.push("{\"intent\":\"hook\",\"text\":\"剥离指令后的内容\"}");
    }

    rules.push(
        "开头无明确指令、但句意可能指向记录或问询(如\"这个值得记一下\"),intent 判为 \"transcribe\",同时把可能的意图加入 alternatives 数组(如 [\"hook\"] 或 [\"ask\"]);开头有明确指令时 alternatives 为空数组或省略".to_string(),
    );
    rules.push("有歧义或不确定时,intent 用 \"transcribe\"".to_string());
    rules.push("无指令时 intent 用 \"transcribe\",text 正常应用上方全部润色规则".to_string());

    s += "\n\n规则:\n";
    for (i, r) in rules.iter().enumerate() {
        s += &format!("{}. {}\n", i + 1, r);
    }
    s += "\n\n输出格式(严格 JSON,不要包裹代码块,不要输出 JSON 以外的任何内容):\n";
    s += &outputs.join("\n或\n");
    s += &extra;
    s
}

/// 从 LLM 输出中解析意图决策。任何失败 → 回退转写原文(保持旧路径行为)。
#[cfg(test)]
pub fn parse_llm_output(raw: &str) -> IntentDecision {
    parse_llm_output_disabled(raw, &[])
}

/// 同 [`parse_llm_output`],但被禁用的意图(即便模型输出)也回退转写。
pub fn parse_llm_output_disabled(raw: &str, disabled: &[String]) -> IntentDecision {
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

    let mut decision = match d.intent.trim() {
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
                    alternatives: Vec::new(),
                }
            }
        }
        "ask" => IntentDecision {
            intent: Intent::Ask,
            text,
            result: d.result.filter(|r| !r.trim().is_empty()),
            alternatives: Vec::new(),
        },
        "hook" => IntentDecision {
            intent: Intent::Hook,
            text,
            result: None,
            alternatives: Vec::new(),
        },
        _ => IntentDecision::passthrough(text),
    };
    decision.alternatives = filter_alternatives(&d.alternatives, &decision.intent, disabled);
    let kind = match &decision.intent {
        Intent::Translate { .. } => "translate",
        Intent::Ask => "ask",
        Intent::Hook => "hook",
        Intent::Transcribe => return decision,
    };
    if disabled.iter().any(|d| d == kind) {
        return IntentDecision::passthrough(decision.text);
    }
    decision
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

/// 过滤 LLM 报告的歧义候选:只留 hook/ask,去重,剔除与主意图同 kind 及被禁用 kind;
/// translate 缺语言参数不参与歧义候选,直接忽略
fn filter_alternatives(raw: &[String], primary: &Intent, disabled: &[String]) -> Vec<String> {
    let primary_kind = match primary {
        Intent::Transcribe | Intent::Translate { .. } => "paste",
        Intent::Ask => "ask",
        Intent::Hook => "hook",
    };
    let mut out: Vec<String> = Vec::new();
    for a in raw {
        let a = a.trim();
        if (a == "hook" || a == "ask")
            && a != primary_kind
            && !disabled.iter().any(|d| d == a)
            && !out.iter().any(|x| x == a)
        {
            out.push(a.to_string());
        }
    }
    out
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
    let routing_instruction = intent_instruction_disabled(&routing.disabled);
    let full_prompt = format!(
        "{}{}\n\n输入文本：\n{}",
        base_prompt, routing_instruction, user_message
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
            let decision = parse_llm_output_disabled(&raw, &routing.disabled);
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

    #[test]
    fn instruction_disabled_translate_omits_translate() {
        let s = intent_instruction_disabled(&["translate".to_string()]);
        assert!(!s.contains("翻译意图"));
        assert!(!s.contains("\"translate\""));
        assert!(s.contains("问询意图"));
        assert!(s.contains("记录意图"));
        assert!(!s.contains("结尾多余的或"));
        assert!(s.ends_with("\"剥离指令后的内容\"}"));
    }

    #[test]
    fn instruction_all_disabled_is_empty() {
        let s = intent_instruction_disabled(
            &["translate", "ask", "hook"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        );
        assert!(s.is_empty());
    }

    #[test]
    fn parse_disabled_intent_falls_back_to_transcribe() {
        let d = parse_llm_output_disabled(
            r#"{"intent":"translate","target_lang":"英文","text":"你好","result":"Hello"}"#,
            &["translate".to_string()],
        );
        assert_eq!(d.intent, Intent::Transcribe);
        assert_eq!(d.text, "你好");
        assert!(d.result.is_none());

        let d = parse_llm_output_disabled(
            r#"{"intent":"ask","text":"今天天气"}"#,
            &["ask".to_string()],
        );
        assert_eq!(d.intent, Intent::Transcribe);

        let d = parse_llm_output_disabled(
            r#"{"intent":"hook","text":"明天开会"}"#,
            &["hook".to_string()],
        );
        assert_eq!(d.intent, Intent::Transcribe);
    }

    #[test]
    fn parse_disabled_other_kind_still_routed() {
        let d = parse_llm_output_disabled(
            r#"{"intent":"ask","text":"今天天气"}"#,
            &["translate".to_string()],
        );
        assert_eq!(d.intent, Intent::Ask);
    }

    #[test]
    fn parse_alternatives_field() {
        let d = parse_llm_output(
            r#"{"intent":"transcribe","text":"这条内容值得被记录下来","alternatives":["hook"]}"#,
        );
        assert_eq!(d.intent, Intent::Transcribe);
        assert_eq!(d.alternatives, vec!["hook".to_string()]);
    }

    #[test]
    fn parse_alternatives_filters_translate_and_unknown() {
        let d = parse_llm_output(
            r#"{"intent":"transcribe","text":"帮我看看","alternatives":["translate","open_url","ask","ask"]}"#,
        );
        assert_eq!(d.alternatives, vec!["ask".to_string()]);
    }

    #[test]
    fn parse_alternatives_drops_primary_kind() {
        let d = parse_llm_output(
            r#"{"intent":"hook","text":"明天开会","alternatives":["hook","ask"]}"#,
        );
        assert_eq!(d.intent, Intent::Hook);
        assert_eq!(d.alternatives, vec!["ask".to_string()]);
    }

    #[test]
    fn parse_without_alternatives_is_empty() {
        let d = parse_llm_output(r#"{"intent":"transcribe","text":"你好"}"#);
        assert!(d.alternatives.is_empty());
    }

    #[test]
    fn disabled_alternative_kind_is_dropped() {
        let d = parse_llm_output_disabled(
            r#"{"intent":"transcribe","text":"值得记录","alternatives":["hook"]}"#,
            &["hook".to_string()],
        );
        assert_eq!(d.intent, Intent::Transcribe);
        assert!(d.alternatives.is_empty());
    }

    #[test]
    fn intent_instruction_mentions_alternatives() {
        let s = intent_instruction();
        assert!(s.contains("alternatives"));
    }
}
