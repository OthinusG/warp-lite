//! Usage mappings adapted from Orca 3e0b82835856dde57f43a47661cf736e3e3dd90d.
//! Copyright (c) 2026 Lovecast Inc. MIT license: docs/ORCA-LICENSE.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Provider {
    Codex,
    Claude,
    Gemini,
    Cursor,
    OpenCodeGo,
    OpenCodeZen,
    Kimi,
    MiniMax,
    MiniMaxCN,
    Zai,
    Zhipu,
    Grok,
    Antigravity,
}
impl Provider {
    pub(crate) const ALL: &'static [Self] = &[
        Self::Codex,
        Self::Claude,
        Self::Gemini,
        Self::Cursor,
        Self::OpenCodeGo,
        Self::OpenCodeZen,
        Self::Kimi,
        Self::MiniMax,
        Self::MiniMaxCN,
        Self::Zai,
        Self::Zhipu,
        Self::Grok,
        Self::Antigravity,
    ];
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude Code",
            Self::Gemini => "Gemini",
            Self::Cursor => "Cursor",
            Self::OpenCodeGo => "OpenCode Go",
            Self::OpenCodeZen => "OpenCode Zen",
            Self::Kimi => "Kimi Code",
            Self::MiniMax => "MiniMax",
            Self::MiniMaxCN => "MiniMax CN",
            Self::Zai => "Z.ai Coding Plan",
            Self::Zhipu => "Zhipu Coding Plan",
            Self::Grok => "Grok",
            Self::Antigravity => "Antigravity",
        }
    }
    pub(crate) fn program(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
            Self::Cursor => "cursor-agent",
            Self::Grok => "grok",
            Self::Antigravity => "agy",
            _ => "opencode",
        }
    }
    pub(crate) fn credential_hint(self) -> &'static str {
        match self {
            Self::Antigravity => "Uses the current agy login (requires 1.1.11 or newer)",
            Self::Cursor | Self::OpenCodeZen => {
                "Dashboard cookie (stored in OS credential storage)"
            }
            Self::Codex | Self::Claude | Self::Gemini | Self::Grok | Self::Kimi => {
                "Access token, or leave empty to use the current CLI login on this computer"
            }
            _ => "API key (stored in OS credential storage)",
        }
    }
    pub(crate) fn login(self) -> Option<&'static str> {
        match self {
            Self::Codex => Some("codex login"),
            Self::Claude => Some("claude auth login"),
            Self::Gemini => Some("gemini"),
            Self::Grok => Some("grok"),
            Self::Kimi => Some("kimi"),
            Self::Antigravity => Some("agy"),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Window {
    pub(crate) name: String,
    pub(crate) used: f64,
    pub(crate) reset: Option<String>,
}
#[derive(Clone, Default, Debug)]
pub(crate) struct Reading {
    pub(crate) windows: Vec<Window>,
    pub(crate) balance: Option<f64>,
}
fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.parse().ok())
        .filter(|v| v.is_finite())
}
fn ratio(value: &Value, used: &str, limit: &str, remaining: &str) -> Option<f64> {
    let total = number(&value[limit]).filter(|total| *total > 0.)?;
    let used = number(&value[used]).or_else(|| number(&value[remaining]).map(|n| total - n))?;
    Some(100. * used / total)
}
fn add(reading: &mut Reading, name: impl Into<String>, used: Option<f64>, reset: &Value) {
    if let Some(used) = used.filter(|n| n.is_finite()) {
        reading.windows.push(Window {
            name: name
                .into()
                .chars()
                .filter(|c| !c.is_control())
                .take(80)
                .collect(),
            used: used.clamp(0., 100.),
            reset: reset
                .as_str()
                .filter(|s| s.len() <= 80)
                .map(str::to_owned)
                .or_else(|| reset.as_i64().map(|n| n.to_string())),
        });
    }
}

pub(crate) fn parse(provider: Provider, data: &Value) -> Result<Reading, &'static str> {
    let mut out = Reading::default();
    match provider {
        Provider::Claude => {
            for (key, label) in [
                ("five_hour", "5h"),
                ("seven_day", "Weekly"),
                ("seven_day_sonnet", "Sonnet weekly"),
                ("seven_day_opus", "Opus weekly"),
            ] {
                let window = &data[key];
                add(
                    &mut out,
                    label,
                    number(&window["utilization"]).or_else(|| number(&window["used_percentage"])),
                    &window["resets_at"],
                );
            }
        }
        Provider::Codex => {
            for key in ["primary_window", "secondary_window"] {
                let w = &data["rate_limit"][key];
                let label = match w["limit_window_seconds"].as_u64() {
                    Some(18000) => "5h".to_owned(),
                    Some(604800) => "Weekly".to_owned(),
                    Some(seconds) => format!("{}h", seconds / 3600),
                    _ => "Quota".into(),
                };
                add(&mut out, label, number(&w["used_percent"]), &w["reset_at"]);
            }
        }
        Provider::OpenCodeGo => {
            for (key, label) in [
                ("rolling", "5h"),
                ("weekly", "Weekly"),
                ("monthly", "Monthly"),
            ] {
                let w = &data["usage"][key];
                add(&mut out, label, number(&w["percent"]), &w["resetsAt"]);
            }
        }
        Provider::OpenCodeZen => {
            if data["billingMode"] == "prepaid" && data["mode"] == "pay-as-you-go" {
                out.balance = data["balanceMicroCents"]
                    .as_str()
                    .and_then(|s| s.parse::<i64>().ok())
                    .filter(|n| n.unsigned_abs() <= 9_007_199_254_740_991)
                    .map(|n| n as f64 / 100_000_000.);
            }
        }
        Provider::Gemini => {
            if let Some(rows) = data["buckets"].as_array() {
                for w in rows.iter().take(64) {
                    if let Some(name) = w["modelId"].as_str() {
                        add(
                            &mut out,
                            name,
                            number(&w["remainingFraction"]).map(|n| (1. - n) * 100.),
                            &w["resetTime"],
                        );
                    }
                }
            }
        }
        Provider::Antigravity => {
            if let Some(groups) = data["command"]["data"]["groups"].as_array() {
                for group in groups.iter().take(32) {
                    if let Some(rows) = group["buckets"].as_array() {
                        for w in rows.iter().take(32).filter(|w| w["disabled"] != true) {
                            let label = format!(
                                "{} · {}",
                                group["name"].as_str().unwrap_or("Quota"),
                                w["name"].as_str().unwrap_or("Quota")
                            );
                            add(
                                &mut out,
                                label,
                                number(&w["remaining_fraction"]).map(|n| (1. - n) * 100.),
                                &w["reset_time"],
                            );
                        }
                    }
                }
            }
        }
        Provider::Kimi => {
            add(
                &mut out,
                "Weekly",
                ratio(&data["usage"], "used", "limit", "remaining"),
                &data["usage"]["resetTime"],
            );
            if let Some(rows) = data["limits"].as_array() {
                for w in rows.iter().take(32) {
                    let name = format!(
                        "{} {}",
                        w["window"]["duration"],
                        w["window"]["timeUnit"].as_str().unwrap_or("window")
                    );
                    add(
                        &mut out,
                        name,
                        ratio(&w["detail"], "used", "limit", "remaining"),
                        &w["detail"]["resetTime"],
                    );
                }
            }
        }
        Provider::MiniMax | Provider::MiniMaxCN => {
            if data["base_resp"]["status_code"]
                .as_i64()
                .is_some_and(|code| code != 0)
            {
                return Err("Provider rejected usage query");
            }
            if let Some(rows) = data["model_remains"].as_array() {
                for w in rows.iter().take(64) {
                    let name = w["model_name"].as_str().unwrap_or("Quota");
                    add(
                        &mut out,
                        format!("{name} · 5h"),
                        number(&w["current_interval_remaining_percent"]).map(|n| 100. - n),
                        &w["end_time"],
                    );
                    add(
                        &mut out,
                        format!("{name} · Weekly"),
                        number(&w["current_weekly_remaining_percent"]).map(|n| 100. - n),
                        &Value::Null,
                    );
                }
            }
        }
        Provider::Zai | Provider::Zhipu => {
            if let Some(rows) = data["data"]["limits"].as_array() {
                for w in rows.iter().take(32) {
                    let name = match (w["unit"].as_u64(), w["number"].as_u64()) {
                        (Some(3), Some(5)) => "5h",
                        (Some(6), Some(1)) => "Weekly",
                        _ => "Quota",
                    };
                    add(
                        &mut out,
                        name,
                        ratio(w, "currentValue", "usage", "remaining")
                            .or_else(|| number(&w["percentage"])),
                        &w["nextResetTime"],
                    );
                }
            }
        }
        Provider::Cursor => {
            let p = &data["individualUsage"]["plan"];
            if p["enabled"] != false {
                add(
                    &mut out,
                    "Monthly",
                    number(&p["totalPercentUsed"])
                        .or_else(|| ratio(p, "used", "limit", "remaining")),
                    &data["billingCycleEnd"],
                );
                for (key, name) in [
                    ("autoPercentUsed", "Cursor models"),
                    ("apiPercentUsed", "Other models"),
                ] {
                    add(&mut out, name, number(&p[key]), &data["billingCycleEnd"]);
                }
            }
            let p = &data["individualUsage"]["onDemand"];
            if p["enabled"] == true {
                add(
                    &mut out,
                    "On demand",
                    ratio(p, "used", "limit", "remaining")
                        .or_else(|| number(&p["totalPercentUsed"])),
                    &data["billingCycleEnd"],
                );
            }
        }
        Provider::Grok => {
            let w = if data["config"].is_object() {
                &data["config"]
            } else {
                data
            };
            add(
                &mut out,
                "Weekly",
                number(&w["creditUsagePercent"]),
                &w["currentPeriod"]["end"],
            );
            let used = number(&w["used"]["val"]);
            let limit = number(&w["monthlyLimit"]["val"]).filter(|n| *n > 0.);
            add(
                &mut out,
                "Monthly",
                used.zip(limit).map(|(u, l)| u / l * 100.),
                &w["billingPeriodEnd"],
            );
        }
    }
    if out.windows.is_empty() && out.balance.is_none() {
        Err("Usage unavailable for this account")
    } else {
        Ok(out)
    }
}

pub(crate) async fn fetch(
    provider: Provider,
    token: &str,
    context: &str,
) -> Result<Reading, &'static str> {
    if token.is_empty() {
        return Err("Sign in or connect an account in Settings");
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not start usage query")?;
    let mut secret = token
        .parse::<reqwest::header::HeaderValue>()
        .map_err(|_| "Invalid credential format")?;
    secret.set_sensitive(true);
    let project;
    let context = if provider == Provider::Gemini && context.is_empty() {
        let response = client
            .post("https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist")
            .bearer_auth(token)
            .json(&serde_json::json!({"metadata":{"ideType":"GEMINI_CLI","pluginType":"GEMINI"}}))
            .send()
            .await
            .map_err(|_| "Could not find Gemini project")?;
        let data = response_json(response).await?;
        project = data["cloudaicompanionProject"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 200)
            .ok_or("Gemini project unavailable")?
            .to_owned();
        &project
    } else {
        context
    };
    let url = match provider {
        Provider::Codex => "https://chatgpt.com/backend-api/wham/usage",
        Provider::Claude => "https://api.anthropic.com/api/oauth/usage",
        Provider::Gemini => "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota",
        Provider::Cursor => "https://cursor.com/api/usage-summary",
        Provider::OpenCodeGo => "https://opencode.ai/zen/go/v1/usage",
        Provider::OpenCodeZen => "https://opencode.ai/console/api/billing/status",
        Provider::Kimi => "https://api.kimi.com/coding/v1/usages",
        Provider::MiniMax => "https://platform.minimax.io/v1/api/openplatform/coding_plan/remains",
        Provider::MiniMaxCN => "https://www.minimaxi.com/v1/api/openplatform/coding_plan/remains",
        Provider::Zai => "https://api.z.ai/api/monitor/usage/quota/limit",
        Provider::Zhipu => "https://open.bigmodel.cn/api/monitor/usage/quota/limit",
        Provider::Grok => "https://cli-chat-proxy.grok.com/v1/billing?format=credits",
        Provider::Antigravity => return Err("Use the current agy login"),
    };
    let mut request = client.get(url).header("Accept", "application/json");
    match provider {
        Provider::Cursor => {
            request = request
                .header("Cookie", secret.clone())
                .header("Origin", "https://cursor.com")
                .header("Referer", "https://cursor.com/dashboard");
        }
        Provider::OpenCodeZen => {
            if context.is_empty() {
                return Err("A workspace ID is required for Zen balance");
            }
            request = request
                .header("Cookie", secret.clone())
                .header("x-org-id", context)
                .header(
                    "Referer",
                    format!("https://opencode.ai/console/{context}/billing"),
                );
        }
        Provider::Zai | Provider::Zhipu => {
            request = request.header("Authorization", secret);
        }
        Provider::Gemini => {
            request = client
                .post(url)
                .bearer_auth(token)
                .json(&serde_json::json!({"project":context}));
        }
        _ => {
            request = request.bearer_auth(token);
        }
    }
    if provider == Provider::Claude {
        request = request
            .header("anthropic-beta", "oauth-2025-04-20")
            .header("User-Agent", "claude-code/2.1.0");
    }
    if provider == Provider::Codex && !context.is_empty() {
        request = request.header("ChatGPT-Account-Id", context);
    }
    if provider == Provider::Grok {
        request = request.header("X-XAI-Token-Auth", "xai-grok-cli");
    }
    let response = request
        .send()
        .await
        .map_err(|_| "Usage query failed; retry in Settings")?;
    let data = response_json(response).await?;
    match parse(provider, &data) {
        Err(_) if provider == Provider::Grok => {
            let response = client
                .get("https://cli-chat-proxy.grok.com/v1/billing")
                .bearer_auth(token)
                .header("X-XAI-Token-Auth", "xai-grok-cli")
                .send()
                .await
                .map_err(|_| "Grok usage query unavailable")?;
            parse(provider, &response_json(response).await?)
        }
        result => result,
    }
}

async fn response_json(mut response: reqwest::Response) -> Result<Value, &'static str> {
    match response.status().as_u16() {
        401 => return Err("Sign-in expired; reconnect in Settings"),
        403 => return Err("Account has no access to this usage endpoint"),
        429 => return Err("Usage query rate limited; retry later"),
        200..=299 => (),
        _ => return Err("Provider usage service unavailable"),
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Incomplete usage response")?
    {
        if body.len() + chunk.len() > 1024 * 1024 {
            return Err("Usage response too large");
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| "Invalid usage response")
}

pub(crate) fn safe_agy_version(text: &str) -> bool {
    text.split(|c: char| !c.is_ascii_digit() && c != '.')
        .filter_map(|word| {
            let numbers: Vec<_> = word
                .split('.')
                .map(str::parse::<u32>)
                .collect::<Result<_, _>>()
                .ok()?;
            (numbers.len() == 3).then_some(numbers)
        })
        .next()
        .is_some_and(|v| v.as_slice() >= [1, 1, 11].as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn missing_usage_never_becomes_zero() {
        for provider in Provider::ALL {
            assert!(parse(*provider, &json!({})).is_err(), "{provider:?}");
        }
        assert!(parse(
            Provider::Cursor,
            &json!({"individualUsage":{"plan":{"enabled":false,"totalPercentUsed":0}}})
        )
        .is_err());
        assert!(parse(Provider::Kimi, &json!({"usage":{"used":10,"limit":0}})).is_err());
        assert!(parse(Provider::Codex, &json!({"credits":{"balance":50}})).is_err());
        assert!(parse(Provider::Grok, &json!({"prepaidBalance":{"val":"50"}})).is_err());
    }
    #[test]
    fn maps_percent_and_remaining_without_inventing_a_limit() {
        let r = parse(
            Provider::OpenCodeGo,
            &json!({"usage":{"rolling":{"percent":23},"weekly":{"percent":105}}}),
        )
        .unwrap();
        assert_eq!((r.windows[0].used, r.windows[1].used), (23., 100.));
        let r = parse(
            Provider::MiniMaxCN,
            &json!({"model_remains":[{"current_interval_remaining_percent":73}]}),
        )
        .unwrap();
        assert_eq!(r.windows[0].used, 27.);
        let r = parse(Provider::OpenCodeZen, &json!({"billingMode":"prepaid","mode":"pay-as-you-go","balanceMicroCents":"-125000000"})).unwrap();
        assert_eq!(r.balance, Some(-1.25));
        assert!(r.windows.is_empty());
        assert!(parse(
            Provider::OpenCodeZen,
            &json!({"billingMode":"credit","mode":"pay-as-you-go","balanceMicroCents":"100000000"})
        )
        .is_err());
    }
    #[test]
    fn supported_provider_payloads_use_their_actual_contracts() {
        let cases = [
            (
                Provider::Codex,
                json!({"rate_limit":{"primary_window":{"used_percent":40,"limit_window_seconds":18000}}}),
            ),
            (Provider::Claude, json!({"five_hour":{"utilization":40}})),
            (
                Provider::Gemini,
                json!({"buckets":[{"remainingFraction":0.6,"modelId":"gemini-pro"}]}),
            ),
            (
                Provider::Cursor,
                json!({"individualUsage":{"plan":{"totalPercentUsed":40}}}),
            ),
            (
                Provider::Kimi,
                json!({"usage":{"limit":"100","remaining":"60"}}),
            ),
            (
                Provider::MiniMax,
                json!({"base_resp":{"status_code":0},"model_remains":[{"current_interval_remaining_percent":60}]}),
            ),
            (
                Provider::Zai,
                json!({"data":{"limits":[{"usage":100,"currentValue":40,"unit":3,"number":5}]}}),
            ),
            (
                Provider::Zhipu,
                json!({"data":{"limits":[{"percentage":40}]}}),
            ),
            (Provider::Grok, json!({"config":{"creditUsagePercent":40}})),
        ];
        for (provider, data) in cases {
            let reading = parse(provider, &data).unwrap();
            assert!(
                (reading.windows[0].used - 40.).abs() < 0.001,
                "{provider:?}"
            );
        }
        assert!(parse(
            Provider::Grok,
            &json!({"config":{"monthlyLimit":{"val":"0"},"used":{"val":"50"}}})
        )
        .is_err());
    }
    #[test]
    fn agy_version_guard_and_disabled_buckets() {
        assert!(!safe_agy_version("agy 1.1.10"));
        assert!(!safe_agy_version("unknown"));
        assert!(!safe_agy_version("agy 1.1.10 (runtime 22.1.0)"));
        assert!(safe_agy_version("Antigravity CLI 1.2.11"));
        let r = parse(Provider::Antigravity, &json!({"command":{"data":{"groups":[{"name":"Pro","buckets":[{"remaining_fraction":0.4,"name":"5h"},{"remaining_fraction":0,"disabled":true}]}]}}})).unwrap();
        assert_eq!(r.windows.len(), 1);
        assert!((r.windows[0].used - 60.).abs() < 0.001);
    }
}
