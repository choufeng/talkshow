//! Hook 扩展执行:"记录…"意图命中后,LLM 整理正文 → 模板渲染 → POST 到配置的后端。
//! 首个预设:滴答清单 Open API(POST /open/v1/task,Bearer PAT 静态令牌)。

use crate::config::{HooksConfig, ProviderConfig, SkillsConfig, TranscriptionConfig};
use crate::logger::Logger;
use crate::providers::ProviderContext;
use std::collections::HashMap;

const SEND_TIMEOUT_SECS: u64 = 15;
const REFINE_EXTRA_TIMEOUT_SECS: u64 = 30;

pub struct HookPayload {
    pub raw: String,
    /// 整理(或润色)后的正文
    pub text: String,
    pub app: String,
}

/// 模板渲染:{{raw}}/{{text}}/{{ts}}/{{app}} 替换,值做 JSON 字符串转义。
// ponytail: 顺序 replace,若值本身含 "{{var}}" 字面量会被二次替换(语音内容概率≈0);
// 需要时改单遍扫描
pub fn render_template(template: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = template.to_string();
    for (k, v) in vars {
        let placeholder = format!("{{{{{}}}}}", k);
        let escaped = serde_json::to_string(v).unwrap_or_default();
        let escaped = escaped
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(&escaped);
        out = out.replace(&placeholder, escaped);
    }
    out
}

/// 解析 headers 配置:每行一个 "Key: Value",跳过空行/缺冒号行
pub fn parse_headers(headers: &str) -> Vec<(String, String)> {
    headers
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let (k, v) = line.split_once(':')?;
            let k = k.trim();
            if k.is_empty() {
                return None;
            }
            Some((k.to_string(), v.trim().to_string()))
        })
        .collect()
}

/// 组装整理 prompt:hooks.prompt + 关联 skills 的 prompt;两者皆空 → None(跳过 LLM)
pub fn assemble_refine_prompt(hooks: &HooksConfig, skills_config: &SkillsConfig) -> Option<String> {
    let skill_prompts: Vec<&str> = skills_config
        .skills
        .iter()
        .filter(|s| s.enabled && hooks.skill_ids.contains(&s.id))
        .map(|s| s.prompt.as_str())
        .collect();
    if hooks.prompt.trim().is_empty() && skill_prompts.is_empty() {
        return None;
    }
    let mut p = String::new();
    if !hooks.prompt.trim().is_empty() {
        p.push_str(hooks.prompt.trim());
    }
    for sp in skill_prompts {
        p.push_str("\n\n");
        p.push_str(sp);
    }
    p.push_str("\n\n请只输出处理后的结果文本，不要添加任何解释、标注或前缀。");
    Some(p)
}

/// LLM 整理正文。未配置整理指令 → 原样透传;LLM 失败 → 由调用方决定回退。
#[allow(clippy::too_many_arguments)]
pub async fn refine_content(
    logger: &Logger,
    hooks: &HooksConfig,
    skills_config: &SkillsConfig,
    transcription_config: &TranscriptionConfig,
    providers: &[ProviderConfig],
    text: &str,
    vertex_cache: &ProviderContext,
) -> Result<String, String> {
    let Some(instruction) = assemble_refine_prompt(hooks, skills_config) else {
        return Ok(text.to_string());
    };

    let Some(provider) = providers
        .iter()
        .find(|p| p.id == transcription_config.polish_provider_id)
    else {
        return Err(format!(
            "未找到润色 Provider {}",
            transcription_config.polish_provider_id
        ));
    };

    let full_prompt = format!("{}\n\n输入文本：\n{}", instruction, text);
    let timeout_secs = crate::skills::llm_timeout_secs(0) + REFINE_EXTRA_TIMEOUT_SECS;

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

    match result {
        Ok(Ok(refined)) => {
            let refined = refined.trim().to_string();
            if refined.is_empty() {
                return Err("LLM 返回空内容".to_string());
            }
            Ok(refined)
        }
        Ok(Err(e)) => Err(format!("LLM 调用失败: {}", e)),
        Err(_) => Err(format!("LLM 调用超时({}s)", timeout_secs)),
    }
}

/// 渲染模板并 POST JSON 到 Hook URL。2xx 视为成功。
pub async fn send_to_hook(
    logger: &Logger,
    hooks: &HooksConfig,
    payload: &HookPayload,
) -> Result<(), String> {
    if hooks.url.trim().is_empty() {
        return Err("Hook URL 未配置".to_string());
    }

    let mut vars = HashMap::new();
    vars.insert("raw", payload.raw.clone());
    vars.insert("text", payload.text.clone());
    vars.insert("ts", chrono::Local::now().to_rfc3339());
    vars.insert("app", payload.app.clone());
    let body = render_template(&hooks.body_template, &vars);

    // 模板由用户配置,渲染后校验一次 JSON,避免把坏 body 打到端点
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("模板渲染结果不是合法 JSON: {} | body={}", e, body))?;

    let client = reqwest::Client::new();
    let mut req = client
        .post(hooks.url.trim())
        .timeout(std::time::Duration::from_secs(SEND_TIMEOUT_SECS))
        .json(&json);
    for (k, v) in parse_headers(&hooks.headers) {
        req = req.header(&k, &v);
    }

    logger.info(
        "hook",
        "Hook 请求发送",
        Some(serde_json::json!({ "url": hooks.url, "body_preview": body.chars().take(200).collect::<String>() })),
    );

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Hook 请求失败: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().await.unwrap_or_default();
        return Err(format!(
            "Hook 返回 {}: {}",
            status,
            body_text.chars().take(200).collect::<String>()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Skill;

    fn vars<'a>(pairs: &'a [(&'a str, &'a str)]) -> HashMap<&'a str, String> {
        pairs.iter().map(|(k, v)| (*k, v.to_string())).collect()
    }

    #[test]
    fn render_template_replaces_all_vars() {
        let v = vars(&[("text", "买牛奶"), ("raw", "记录 买牛奶")]);
        let out = render_template(r#"{"title":"{{text}}","content":"{{raw}}"}"#, &v);
        assert_eq!(out, r#"{"title":"买牛奶","content":"记录 买牛奶"}"#);
    }

    #[test]
    fn render_template_escapes_json_specials() {
        let v = vars(&[("text", "他说\"你好\"\n第二行")]);
        let out = render_template(r#"{"t":"{{text}}"}"#, &v);
        assert_eq!(out, r#"{"t":"他说\"你好\"\n第二行"}"#);
        // 渲染结果必须是合法 JSON
        assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok());
    }

    #[test]
    fn render_template_keeps_unknown_placeholders() {
        let v = vars(&[]);
        let out = render_template("{{nope}}", &v);
        assert_eq!(out, "{{nope}}");
    }

    #[test]
    fn parse_headers_basic() {
        let hs = parse_headers("Authorization: Bearer abc123\nX-API-Key: foo\n");
        assert_eq!(
            hs,
            vec![
                ("Authorization".to_string(), "Bearer abc123".to_string()),
                ("X-API-Key".to_string(), "foo".to_string())
            ]
        );
    }

    #[test]
    fn parse_headers_skips_blank_and_invalid() {
        let hs = parse_headers("\n  \nno-colon-line\n:empty-key\n OK: yes ");
        assert_eq!(hs, vec![("OK".to_string(), "yes".to_string())]);
    }

    fn skill(id: &str, enabled: bool) -> Skill {
        Skill {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            prompt: format!("{} 的规则", id),
            builtin: false,
            editable: true,
            enabled,
        }
    }

    fn hooks_with(prompt: &str, skill_ids: &[&str]) -> HooksConfig {
        HooksConfig {
            enabled: true,
            url: String::new(),
            headers: String::new(),
            body_template: String::new(),
            prompt: prompt.to_string(),
            skill_ids: skill_ids.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn refine_prompt_none_when_unconfigured() {
        let skills = SkillsConfig {
            enabled: true,
            skills: vec![skill("a", true)],
        };
        assert!(assemble_refine_prompt(&hooks_with("", &[]), &skills).is_none());
    }

    #[test]
    fn refine_prompt_combines_prompt_and_skills() {
        let skills = SkillsConfig {
            enabled: true,
            skills: vec![skill("a", true), skill("b", false), skill("c", true)],
        };
        let p = assemble_refine_prompt(&hooks_with("提炼标题", &["a", "b", "c"]), &skills).unwrap();
        assert!(p.contains("提炼标题"));
        assert!(p.contains("a 的规则")); // 启用的关联 skill
        assert!(!p.contains("b 的规则")); // 禁用的关联 skill 被过滤
        assert!(p.contains("c 的规则"));
    }

    #[test]
    fn refine_prompt_with_only_prompt() {
        let skills = SkillsConfig {
            enabled: true,
            skills: vec![],
        };
        let p = assemble_refine_prompt(&hooks_with("只提炼", &[]), &skills).unwrap();
        assert!(p.contains("只提炼"));
    }
}
