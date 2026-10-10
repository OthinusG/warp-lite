//! Opt-in, desktop-local provider usage; credentials never enter persisted metadata.
pub(crate) mod providers;
use async_compat::CompatExt as _;
use providers::{Provider, Reading};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::Read,
    path::PathBuf,
    time::{Duration, Instant},
};
use warpui::{Entity, ModelContext, SingletonEntity};
use warpui_extras::secure_storage::AppContextExt as _;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Account {
    pub(crate) id: String,
    pub(crate) provider: Provider,
    pub(crate) label: String,
    pub(crate) context: String,
    pub(crate) visible: bool,
    pub(crate) cli_login: bool,
}
#[derive(Default)]
pub(crate) struct AgentUsage {
    pub(crate) accounts: Vec<Account>,
    pub(crate) readings: HashMap<String, Result<Reading, &'static str>>,
    pub(crate) checking: bool,
    pub(crate) status: String,
    pub(crate) updated: Option<Instant>,
    generation: u64,
    configuration_error: bool,
    #[cfg(debug_assertions)]
    capture_preview: bool,
}
fn metadata_path() -> PathBuf {
    warp_core::paths::data_dir().join("agent-usage-accounts.json")
}
fn key(id: &str) -> String {
    format!("warpai-agent-usage-{id}")
}
fn valid_account(account: &Account) -> bool {
    uuid::Uuid::parse_str(&account.id).is_ok()
        && !account.label.trim().is_empty()
        && account.label.len() <= 80
        && account.context.len() <= 200
        && !account.label.chars().any(char::is_control)
        && !account.context.chars().any(char::is_control)
}
impl AgentUsage {
    pub(crate) fn new(ctx: &mut ModelContext<Self>) -> Self {
        let mut model = Self::default();
        match bounded_json(metadata_path()) {
            Ok(value) => match serde_json::from_value::<Vec<Account>>(value) {
                Ok(accounts)
                    if accounts.len() <= 64
                        && accounts.iter().all(valid_account)
                        && accounts
                            .iter()
                            .map(|a| &a.id)
                            .collect::<std::collections::HashSet<_>>()
                            .len()
                            == accounts.len() =>
                {
                    model.accounts = accounts
                }
                _ => {
                    model.status =
                        "Could not load usage accounts; existing configuration was preserved".into()
                }
            },
            Err(_) if metadata_path().exists() => {
                model.status =
                    "Could not load usage accounts; existing configuration was preserved".into()
            }
            Err(_) => (),
        }
        model.configuration_error = !model.status.is_empty();
        Self::schedule(ctx);
        model
    }
    fn schedule(ctx: &mut ModelContext<Self>) {
        ctx.spawn(
            async { warpui::r#async::Timer::after(Duration::from_secs(60)).await },
            |model, _, ctx| {
                if model
                    .updated
                    .is_none_or(|time| time.elapsed() >= Duration::from_secs(300))
                {
                    model.refresh(ctx);
                }
                Self::schedule(ctx);
            },
        );
    }
    fn persist(accounts: &[Account]) -> Result<(), &'static str> {
        let bytes = serde_json::to_vec(accounts).map_err(|_| "Could not save usage accounts")?;
        crate::agent_communication::setup::atomic_write(&metadata_path(), &bytes)
            .map_err(|_| "Could not save usage accounts")
    }
    pub(crate) fn add(
        &mut self,
        provider: Provider,
        label: String,
        context: String,
        token: String,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        let cli_login = token.is_empty();
        let account = Account {
            id: uuid::Uuid::new_v4().to_string(),
            provider,
            label,
            context,
            visible: true,
            cli_login,
        };
        if !self.configuration_error
            && self.accounts.len() < 64
            && valid_account(&account)
            && token.len() <= 16 * 1024
            && !token.chars().any(char::is_control)
            && (!cli_login
                || matches!(
                    provider,
                    Provider::Codex
                        | Provider::Claude
                        | Provider::Gemini
                        | Provider::Kimi
                        | Provider::Grok
                        | Provider::Antigravity
                ))
        {
            if !cli_login
                && ctx
                    .secure_storage()
                    .write_value(&key(&account.id), &token)
                    .is_err()
            {
                self.status = "Could not store credential in OS credential storage".into();
                ctx.notify();
                return false;
            }
            let mut next = self.accounts.clone();
            next.push(account.clone());
            if let Err(error) = Self::persist(&next) {
                if !cli_login {
                    let _ = ctx.secure_storage().remove_value(&key(&account.id));
                }
                self.status = error.into();
                ctx.notify();
                return false;
            }
            self.accounts = next;
            self.status.clear();
            self.generation += 1;
            self.refresh(ctx);
            ctx.notify();
            true
        } else {
            self.status = "Enter a label and the required credential; account limit is 64".into();
            ctx.notify();
            false
        }
    }
    pub(crate) fn toggle(&mut self, id: &str, ctx: &mut ModelContext<Self>) {
        if self.configuration_error {
            return;
        }
        let mut next = self.accounts.clone();
        if let Some(account) = next.iter_mut().find(|a| a.id == id) {
            account.visible = !account.visible;
        }
        match Self::persist(&next) {
            Ok(()) => {
                self.accounts = next;
                self.generation += 1;
                self.updated = None;
            }
            Err(error) => self.status = error.into(),
        }
        ctx.notify();
    }
    pub(crate) fn remove(&mut self, id: &str, ctx: &mut ModelContext<Self>) {
        if self.configuration_error {
            return;
        }
        let has_credential = self.accounts.iter().any(|a| a.id == id && !a.cli_login);
        let mut next = self.accounts.clone();
        next.retain(|a| a.id != id);
        match Self::persist(&next) {
            Ok(()) => {
                self.accounts = next;
                self.readings.remove(id);
                self.generation += 1;
                if has_credential && ctx.secure_storage().remove_value(&key(id)).is_err() {
                    self.status =
                        "Account removed; OS credential cleanup could not be confirmed".into();
                }
            }
            Err(error) => self.status = error.into(),
        }
        ctx.notify();
    }
    pub(crate) fn refresh(&mut self, ctx: &mut ModelContext<Self>) {
        #[cfg(debug_assertions)]
        if self.capture_preview {
            return;
        }
        if self.checking {
            return;
        }
        let requests: Vec<_> = self
            .accounts
            .iter()
            .filter(|a| a.visible)
            .map(|account| {
                let token = if account.cli_login {
                    None
                } else {
                    ctx.secure_storage().read_value(&key(&account.id)).ok()
                };
                (account.clone(), token)
            })
            .collect();
        if requests.is_empty() {
            return;
        }
        self.checking = true;
        let generation = self.generation;
        ctx.notify();
        ctx.spawn(
            async move {
                futures::future::join_all(requests.into_iter().map(|(account, token)| async move {
                    let reading = async {
                        tokio::time::timeout(Duration::from_secs(40), query(&account, token))
                            .await
                            .unwrap_or(Err("Usage query timed out"))
                    }
                    .compat()
                    .await;
                    (account.id, reading)
                }))
                .await
            },
            move |model, rows, ctx| {
                model.checking = false;
                if generation == model.generation {
                    model.readings = rows.into_iter().collect();
                    model.updated = Some(Instant::now());
                } else {
                    model.refresh(ctx);
                }
                ctx.notify();
            },
        );
    }
    #[cfg(debug_assertions)]
    pub(crate) fn capture_fixture(&mut self, state: &str, ctx: &mut ModelContext<Self>) {
        self.capture_preview = true;
        self.generation += 1;
        self.accounts.clear();
        self.readings.clear();
        self.checking = state == "loading";
        self.status.clear();
        self.updated = Some(Instant::now());
        if state != "empty" {
            for index in 0..if state == "overflow" { 8 } else { 3 } {
                let provider =
                    [Provider::Codex, Provider::Claude, Provider::OpenCodeZen][index % 3];
                let id = uuid::Uuid::from_u128(index as u128 + 1).to_string();
                self.accounts.push(Account {
                    id: id.clone(),
                    provider,
                    label: if state == "overflow" {
                        "A long account label for native clipping verification".into()
                    } else {
                        format!("Sample {}", index + 1)
                    },
                    context: String::new(),
                    visible: true,
                    cli_login: false,
                });
                if state != "loading" {
                    self.readings.insert(
                        id,
                        if state == "error" {
                            Err("Sign-in expired; reconnect in Settings")
                        } else if state == "balance" || provider == Provider::OpenCodeZen {
                            Ok(Reading {
                                windows: Vec::new(),
                                balance: Some(27.86),
                            })
                        } else {
                            Ok(Reading {
                                windows: vec![providers::Window {
                                    name: "5h".into(),
                                    used: 37. + index as f64 * 22.,
                                    reset: None,
                                }],
                                balance: None,
                            })
                        },
                    );
                }
            }
        }
        ctx.notify();
    }
}
impl Entity for AgentUsage {
    type Event = ();
}
impl SingletonEntity for AgentUsage {}

pub(crate) fn agent_icon(program: &str) -> crate::ui_components::icons::Icon {
    let name = std::path::Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program);
    enum_iterator::all::<crate::terminal::cli_agent::CLIAgent>()
        .find(|agent| {
            agent
                .command_prefixes()
                .iter()
                .any(|prefix| name == *prefix || name.strip_suffix(".exe") == Some(*prefix))
        })
        .and_then(|agent| agent.icon())
        .unwrap_or(crate::ui_components::icons::Icon::Terminal)
}
fn bounded_json(path: PathBuf) -> Result<serde_json::Value, &'static str> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| "Sign in with the CLI first")?
        .take(256 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read CLI login")?;
    if bytes.len() > 256 * 1024 {
        return Err("CLI login data too large");
    }
    serde_json::from_slice(&bytes).map_err(|_| "CLI login data unavailable")
}
async fn output(program: &str, args: &[&str], seconds: u64) -> Result<Vec<u8>, &'static str> {
    use tokio::io::AsyncReadExt as _;
    let operation = async {
        let mut child = tokio::process::Command::new(program)
            .args(args)
            .kill_on_drop(true)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|_| "CLI is unavailable")?;
        let stdout = child.stdout.take().ok_or("CLI output unavailable")?;
        let mut bytes = Vec::new();
        stdout
            .take(512 * 1024 + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| "Could not read CLI output")?;
        if bytes.len() > 512 * 1024 {
            return Err("CLI response too large");
        }
        if !child
            .wait()
            .await
            .map_err(|_| "CLI query failed")?
            .success()
        {
            return Err("CLI query failed; sign in again");
        }
        Ok(bytes)
    };
    tokio::time::timeout(Duration::from_secs(seconds), operation)
        .await
        .map_err(|_| "CLI usage query timed out")?
}
async fn query(account: &Account, token: Option<String>) -> Result<Reading, &'static str> {
    if account.provider == Provider::Antigravity {
        let version = output("agy", &["--version"], 5).await?;
        if !providers::safe_agy_version(&String::from_utf8_lossy(&version)) {
            return Err("Requires agy 1.1.11 or newer for a free usage query");
        }
        let bytes = output(
            "agy",
            &[
                "-p",
                "/usage",
                "--output-format",
                "json",
                "--print-timeout",
                "20s",
            ],
            30,
        )
        .await?;
        let data = serde_json::from_slice(&bytes).map_err(|_| "Invalid agy usage response")?;
        return providers::parse(Provider::Antigravity, &data);
    }
    let mut context = account.context.clone();
    let token = if account.cli_login {
        let home = dirs::home_dir().ok_or("CLI home directory unavailable")?;
        let data = match account.provider {
            Provider::Codex => bounded_json(
                std::env::var_os("CODEX_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home.join(".codex"))
                    .join("auth.json"),
            )?,
            Provider::Claude => {
                #[cfg(target_os = "macos")]
                {
                    serde_json::from_slice(
                        &output(
                            "security",
                            &[
                                "find-generic-password",
                                "-s",
                                "Claude Code-credentials",
                                "-w",
                            ],
                            5,
                        )
                        .await?,
                    )
                    .map_err(|_| "Claude login unavailable")?
                }
                #[cfg(not(target_os = "macos"))]
                {
                    bounded_json(home.join(".claude/.credentials.json"))?
                }
            }
            Provider::Gemini => bounded_json(home.join(".gemini/oauth_creds.json"))?,
            Provider::Kimi => bounded_json(
                std::env::var_os("KIMI_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home.join(".kimi"))
                    .join("credentials/kimi-code.json"),
            )?,
            Provider::Grok => bounded_json(
                std::env::var_os("GROK_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home.join(".grok"))
                    .join("auth.json"),
            )?,
            _ => return Err("Connect a credential in Settings"),
        };
        let token = match account.provider {
            Provider::Codex => {
                if context.is_empty() {
                    context = data["tokens"]["account_id"]
                        .as_str()
                        .unwrap_or("")
                        .to_owned();
                }
                data["tokens"]["access_token"].as_str()
            }
            Provider::Claude => data["claudeAiOauth"]["accessToken"].as_str(),
            Provider::Grok => data
                .as_object()
                .and_then(|entries| {
                    entries
                        .iter()
                        .find(|(k, v)| k.starts_with("https://auth.x.ai") && v["key"].is_string())
                        .map(|(_, v)| v)
                })
                .and_then(|v| v["key"].as_str()),
            _ => data["access_token"].as_str(),
        };
        token
            .ok_or("CLI login unavailable; sign in again")?
            .to_owned()
    } else {
        token.ok_or("Credential unavailable; reconnect in Settings")?
    };
    providers::fetch(account.provider, &token, &context).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn account_metadata_contains_no_credentials_and_rejects_untrusted_labels() {
        let mut account = Account {
            id: uuid::Uuid::new_v4().to_string(),
            provider: Provider::Codex,
            label: "Current CLI account".into(),
            context: String::new(),
            visible: true,
            cli_login: true,
        };
        assert!(valid_account(&account));
        let json = serde_json::to_value(&account).unwrap();
        assert!(json.get("token").is_none());
        assert!(json.get("credential").is_none());
        account.label = "bad\nlabel".into();
        assert!(!valid_account(&account));
        account.label = "Valid".into();
        account.id = "../../other".into();
        assert!(!valid_account(&account));
    }
}
