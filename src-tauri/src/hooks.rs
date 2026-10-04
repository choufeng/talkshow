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
    /// 整理(或润色)后的正文;LLM 输出 JSON 时为该 JSON 字符串
    pub text: String,
    pub app: String,
    /// LLM 结构化输出的变量池(如 title/dueDate/priority),键名即模板 {{变量}}
    pub vars: HashMap<String, String>,
}

/// refine_content 结果:纯文本模式 vars 为空,结构化模式 vars 来自 LLM JSON
pub struct RefineResult {
    pub text: String,
    pub vars: HashMap<String, String>,
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

/// 递归清理渲染后的 JSON:
/// 1) 值含 "{{" 的字段 = LLM 未输出该变量 → 整个键值对删除
/// 2) 字符串值恰为数字/布尔 → 还原原生类型(模板统一带引号写 "priority":"{{priority}}")
pub fn clean_json(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(map) => {
            let doomed: Vec<String> = map
                .iter()
                .filter(|(_, val)| val.as_str().is_some_and(|s| s.contains("{{")))
                .map(|(k, _)| k.clone())
                .collect();
            for k in doomed {
                map.remove(&k);
            }
            for (_, val) in map.iter_mut() {
                coerce_scalar(val);
                clean_json(val);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                coerce_scalar(item);
                clean_json(item);
            }
        }
        _ => {}
    }
}

fn coerce_scalar(v: &mut serde_json::Value) {
    if let Some(s) = v.as_str() {
        if let Ok(n) = s.parse::<serde_json::Number>() {
            *v = serde_json::Value::Number(n);
        } else if s == "true" {
            *v = serde_json::Value::Bool(true);
        } else if s == "false" {
            *v = serde_json::Value::Bool(false);
        }
    }
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
) -> Result<RefineResult, String> {
    let Some(instruction) = assemble_refine_prompt(hooks, skills_config) else {
        return Ok(RefineResult {
            text: text.to_string(),
            vars: Default::default(),
        });
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

    let full_prompt = format!("{}{}\n\n输入文本：\n{}", instruction, time_context(), text);
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
            let vars = parse_json_vars(&refined);
            Ok(RefineResult {
                text: refined,
                vars,
            })
        }
        Ok(Err(e)) => Err(format!("LLM 调用失败: {}", e)),
        Err(_) => Err(format!("LLM 调用超时({}s)", timeout_secs)),
    }
}

/// 当前时间上下文:供整理 LLM 换算"明天下午三点"等相对时间
fn time_context() -> String {
    let now = chrono::Local::now();
    let weekday = ["日", "一", "二", "三", "四", "五", "六"]
        [now.format("%u").to_string().parse::<usize>().unwrap_or(7) - 1];
    format!(
        "\n\n【当前时间】{} 星期{}，时区偏移 {}。涉及相对时间（今天/明天/后天/下午三点等）时，换算为具体日期时间。",
        now.format("%Y-%m-%d %H:%M"),
        weekday,
        now.format("%z")
    )
}

/// LLM 输出若为 JSON object → 提取为变量池(值为 null/嵌套以外的标量;嵌套转 JSON 字符串);否则空池
fn parse_json_vars(output: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(output)
    else {
        return vars;
    };
    for (k, v) in map {
        let val = match v {
            serde_json::Value::String(val) => val,
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            // 数组/对象转 JSON 字符串,模板里不带引号引用: "items":{{items}}
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                serde_json::to_string(&v).unwrap_or_default()
            }
            serde_json::Value::Null => continue, // null = 缺失,渲染后由 clean 删占位符字段
        };
        vars.insert(k, val);
    }
    vars
}

/// 渲染模板为最终 body(含 clean pass)。发送与确认预览共用。
pub fn render_body(hooks: &HooksConfig, payload: &HookPayload) -> Result<String, String> {
    let mut vars: HashMap<&str, String> = HashMap::new();
    vars.insert("raw", payload.raw.clone());
    vars.insert("text", payload.text.clone());
    vars.insert("ts", chrono::Local::now().to_rfc3339());
    vars.insert("app", payload.app.clone());
    // LLM 变量池覆盖注入(title/dueDate/...);同名时 LLM 优先
    for (k, v) in &payload.vars {
        vars.insert(k.as_str(), v.clone());
    }
    let body = render_template(&hooks.body_template, &vars);

    // 清理 pass:LLM 未输出的字段残留 {{var}} → 删除该键;数字/布尔字符串还原原生类型
    let mut json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("模板渲染结果不是合法 JSON: {} | body={}", e, body))?;
    clean_json(&mut json);
    Ok(serde_json::to_string(&json).unwrap_or_else(|_| body.clone()))
}

/// POST 已渲染 body 到 Hook URL。2xx 视为成功。
pub async fn post_body(logger: &Logger, hooks: &HooksConfig, body: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let mut req = client
        .post(hooks.url.trim())
        .timeout(std::time::Duration::from_secs(SEND_TIMEOUT_SECS))
        .header("Content-Type", "application/json")
        .body(body.to_string());
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

/// 渲染模板并 POST JSON 到 Hook URL。2xx 视为成功。
pub async fn send_to_hook(
    logger: &Logger,
    hooks: &HooksConfig,
    payload: &HookPayload,
) -> Result<(), String> {
    if hooks.url.trim().is_empty() {
        return Err("Hook URL 未配置".to_string());
    }
    let body = render_body(hooks, payload)?;
    post_body(logger, hooks, &body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Skill;

    fn vars<'a>(pairs: &'a [(&'a str, &'a str)]) -> HashMap<&'a str, String> {
        pairs.iter().map(|(k, v)| (*k, v.to_string())).collect()
    }

    #[test]
    fn render_body_combines_vars_and_clean() {
        let hooks = HooksConfig {
            enabled: true,
            url: String::new(),
            headers: String::new(),
            body_template:
                "{\"title\":\"{{title}}\",\"content\":\"{{text}}\",\"due\":\"{{dueDate}}\"}"
                    .to_string(),
            prompt: String::new(),
            skill_ids: Vec::new(),
        };
        let mut vars = HashMap::new();
        vars.insert("title".to_string(), "买牛奶".to_string());
        // dueDate 缺失 → 占位符字段应被清理
        let payload = HookPayload {
            raw: "记录 买牛奶".to_string(),
            text: "买牛奶".to_string(),
            app: "Safari".to_string(),
            vars,
        };
        let body = render_body(&hooks, &payload).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let obj = v.as_object().unwrap();
        assert_eq!(obj.get("title").unwrap(), "买牛奶");
        assert_eq!(obj.get("content").unwrap(), "买牛奶");
        assert!(!obj.contains_key("due"));
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
    fn parse_json_vars_from_structured_output() {
        let vars = parse_json_vars(
            r#"{"title":"开会","dueDate":"2026-09-25T15:00:00+0800","priority":3,"isAllDay":false,"items":null}"#,
        );
        assert_eq!(vars.get("title").unwrap(), "开会");
        assert_eq!(vars.get("dueDate").unwrap(), "2026-09-25T15:00:00+0800");
        assert_eq!(vars.get("priority").unwrap(), "3");
        assert_eq!(vars.get("isAllDay").unwrap(), "false");
        assert!(!vars.contains_key("items")); // null = 缺失
    }

    #[test]
    fn parse_json_vars_plain_text_returns_empty() {
        assert!(parse_json_vars("开会讨论Q3预算").is_empty());
        assert!(parse_json_vars(r#"["not","object"]"#).is_empty());
    }

    #[test]
    fn clean_json_removes_unresolved_placeholders() {
        let mut v: serde_json::Value =
            serde_json::from_str(r#"{"title":"开会","dueDate":"{{dueDate}}","keep":"正常值"}"#)
                .unwrap();
        clean_json(&mut v);
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("dueDate")); // 占位符字段删除
        assert_eq!(obj.get("title").unwrap(), "开会");
        assert_eq!(obj.get("keep").unwrap(), "正常值");
    }

    #[test]
    fn clean_json_coerces_scalar_strings() {
        let mut v: serde_json::Value = serde_json::from_str(
            r#"{"priority":"3","isAllDay":"false","content":"room 3","n":"1.5"}"#,
        )
        .unwrap();
        clean_json(&mut v);
        let obj = v.as_object().unwrap();
        assert_eq!(obj.get("priority").unwrap(), &serde_json::json!(3));
        assert_eq!(obj.get("isAllDay").unwrap(), &serde_json::json!(false));
        assert_eq!(obj.get("n").unwrap(), &serde_json::json!(1.5));
        assert_eq!(obj.get("content").unwrap(), "room 3"); // 非纯数字不动
    }

    #[test]
    fn clean_json_handles_nested() {
        let mut v: serde_json::Value = serde_json::from_str(
            r#"{"title":"t","items":[{"name":"{{x}}"},{"name":"step"}],"tags":["{{t}}"]}"#,
        )
        .unwrap();
        clean_json(&mut v);
        let obj = v.as_object().unwrap();
        let items = obj.get("items").unwrap().as_array().unwrap();
        assert!(!items[0].as_object().unwrap().contains_key("name"));
        assert_eq!(items[1].as_object().unwrap().get("name").unwrap(), "step");
        // ponytail: 数组元素内的 {{ 占位符不删(仅字段级删除);动态数组用整体变量 "items":{{items}}
        assert_eq!(obj.get("tags").unwrap().as_array().unwrap().len(), 1);
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
