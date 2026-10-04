//! Per-launch MCP transport binding, independent of a vendor's shared process environment.
use crate::mcp::Bridge;
use anyhow::{anyhow, ensure, Result};
use async_tungstenite::{
    tokio::{accept_hdr_async_with_config, client_async_with_config},
    tungstenite::{
        handshake::server::{Request, Response},
        protocol::WebSocketConfig,
        Message,
    },
};
use futures_util::{SinkExt, StreamExt};
use rmcp::ServiceExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashSet},
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncWriteExt, process::Command};
use uuid::Uuid;

pub const LAUNCHES: &str = "WARP_AGENT_LAUNCHES";
pub const LAUNCH_CATALOG: &str = "native-launches.json";
const PROXY_TOKEN: &str = "WARP_AGENT_SESSION_TOKEN";
const SERVER: &str = "warp-lite-communication";

#[derive(Serialize, Deserialize)]
pub struct NativeLaunch {
    pub executable: PathBuf,
    pub program: String,
    pub options: crate::launch::LaunchOptions,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ForwardContext {
    directory: Option<String>,
}

pub(crate) fn without_terminal_binding(command: &mut Command) {
    for name in [
        crate::transport::ENDPOINT,
        crate::transport::CAPABILITY,
        crate::transport::TERMINAL,
        LAUNCHES,
        "WARP_AGENT_BIN",
        "WARP_AGENT_LAUNCH_PATH",
        PROXY_TOKEN,
    ] {
        command.env_remove(name);
    }
    if let (Some(path), Some(launchers)) = (
        std::env::var_os("PATH"),
        std::env::var_os("WARP_AGENT_LAUNCH_PATH"),
    ) {
        if let Ok(path) = std::env::join_paths(
            std::env::split_paths(&path).filter(|path| path.as_os_str() != launchers),
        ) {
            command.env("PATH", path);
        }
    }
}

fn codex_no_proxy(primary: Option<OsString>, fallback: Option<OsString>) -> OsString {
    let mut bypass = primary.or(fallback).unwrap_or_default();
    if !bypass.is_empty() {
        bypass.push(",");
    }
    bypass.push("127.0.0.1");
    bypass
}

/// No command string or shell evaluation: aliases dispatch to the resolved native executable.
pub fn native_executable(invocation: &str) -> Option<NativeLaunch> {
    let catalog = std::env::var_os("WARP_AGENT_LAUNCH_PATH")
        .map(|directory| PathBuf::from(directory).join(LAUNCH_CATALOG));
    resolve_native_executable(
        invocation,
        catalog.as_deref(),
        std::env::var(LAUNCHES).ok().as_deref(),
    )
}

fn resolve_native_executable(
    invocation: &str,
    catalog: Option<&Path>,
    snapshot: Option<&str>,
) -> Option<NativeLaunch> {
    let name = Path::new(invocation).file_name()?.to_str()?;
    let normalized = if cfg!(windows) {
        name.to_ascii_lowercase()
    } else {
        name.to_owned()
    };
    let name = if cfg!(windows) {
        normalized.strip_suffix(".exe").unwrap_or(&normalized)
    } else {
        name
    };
    if matches!(name, "warpai-agent" | "warpai-agent.exe") {
        return None;
    }
    let current: Option<BTreeMap<String, NativeLaunch>> = catalog
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let mut launches = current.unwrap_or_default();
    let old: BTreeMap<String, NativeLaunch> = snapshot
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default();
    for (key, mut launch) in old {
        // A stale pane snapshot preserves execution, but cannot authorize communication.
        if catalog.is_some() {
            launch.program = "custom".into();
        }
        launches.entry(key).or_insert(launch);
    }
    let key = launches
        .keys()
        .find(|key| {
            if cfg!(windows) {
                key.eq_ignore_ascii_case(name)
            } else {
                key.as_str() == name
            }
        })?
        .clone();
    launches.remove(&key)
}

pub fn install_launchers(
    directory: &Path,
    companion: &Path,
    launches: &BTreeMap<String, NativeLaunch>,
) -> Result<()> {
    std::fs::create_dir_all(directory)?;
    for name in launches.keys() {
        ensure!(
            !name.is_empty()
                && !matches!(name.as_str(), "." | ".." | "warpai-agent" | "warpai-agent.exe")
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')),
            "Invalid native launcher name"
        );
        let path = directory.join(if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.clone()
        });
        if path.exists() {
            continue;
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(companion, path)?;
        #[cfg(windows)]
        std::fs::hard_link(companion, &path)
            .or_else(|_| std::fs::copy(companion, path).map(|_| ()))?;
    }
    Ok(())
}

/// The daemon receives only this private IPC address, never terminal capabilities.
struct Relay {
    endpoint: String,
    #[cfg(unix)]
    directory: PathBuf,
    task: tokio::task::JoinHandle<()>,
}
impl Relay {
    async fn start(bridge: Bridge) -> Result<Self> {
        #[cfg(unix)]
        let directory = {
            use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
            let directory = PathBuf::from("/tmp").join(format!("warp-mcp-{}", Uuid::new_v4()));
            std::fs::DirBuilder::new().mode(0o700).create(&directory)?;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
            directory
        };
        #[cfg(unix)]
        let endpoint = directory.join("mcp.sock").to_string_lossy().into_owned();
        #[cfg(windows)]
        let endpoint = format!(r"\\.\pipe\warp-mcp-{}", Uuid::new_v4());
        #[cfg(unix)]
        let listener = tokio::net::UnixListener::bind(&endpoint)?;
        #[cfg(windows)]
        let listener = tokio::net::windows::named_pipe::ServerOptions::new()
            .first_pipe_instance(true)
            .create(&endpoint)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&endpoint, std::fs::Permissions::from_mode(0o600))?;
        }
        #[cfg(windows)]
        let next_endpoint = endpoint.clone();
        let task = tokio::spawn(async move {
            #[cfg(windows)]
            let mut listener = listener;
            let mut clients = tokio::task::JoinSet::new();
            loop {
                let stream = {
                    #[cfg(unix)]
                    {
                        match listener.accept().await {
                            Ok((stream, _)) => stream,
                            Err(_) => break,
                        }
                    }
                    #[cfg(windows)]
                    {
                        if listener.connect().await.is_err() {
                            break;
                        }
                        let Ok(next) = tokio::net::windows::named_pipe::ServerOptions::new()
                            .create(&next_endpoint)
                        else {
                            break;
                        };
                        std::mem::replace(&mut listener, next)
                    }
                };
                // Expired launch sockets cannot create a new terminal identity.
                // JoinSet keeps all clients owned by this launch; aborting it revokes their streams.
                if clients.len() >= 16 {
                    while clients.try_join_next().is_some() {}
                }
                if clients.len() >= 16 {
                    continue;
                }
                let bridge = bridge.clone();
                clients.spawn(async move {
                    let mut stream = stream;
                    let Ok(context) = crate::transport::receive::<ForwardContext>(
                        &mut stream,
                        std::time::Instant::now() + Duration::from_secs(5),
                    )
                    .await
                    else {
                        return;
                    };
                    let bridge = match context.directory {
                        Some(directory) => bridge.with_native_directory(directory),
                        None => bridge,
                    };
                    let Ok(transport) = crate::mcp::legacy_transport(tokio::io::split(stream)).await else {
                        return;
                    };
                    if let Ok(service) = bridge.serve(transport).await {
                        let _ = service.waiting().await;
                    }
                });
            }
        });
        Ok(Self {
            endpoint,
            #[cfg(unix)]
            directory,
            task,
        })
    }
    fn config(&self, companion: &Path) -> Value {
        json!({"command": companion, "args": ["forward", self.endpoint]})
    }
}
impl Drop for Relay {
    fn drop(&mut self) {
        self.task.abort();
        #[cfg(unix)]
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

pub async fn forward(endpoint: &str) -> Result<()> {
    let mut stream =
        tokio::time::timeout(Duration::from_secs(5), crate::transport::connect(endpoint))
            .await
            .map_err(|_| anyhow!("Native MCP relay unavailable"))??;
    let directory = std::env::current_dir()?
        .to_str()
        .ok_or_else(|| anyhow!("Native workspace must be UTF-8"))?
        .to_owned();
    crate::transport::send(
        &mut stream,
        &ForwardContext {
            directory: Some(directory),
        },
        std::time::Instant::now() + Duration::from_secs(5),
    )
    .await?;
    let (mut reader, mut writer) = tokio::io::split(stream);
    tokio::try_join!(
        async {
            tokio::io::copy(&mut tokio::io::stdin(), &mut writer).await?;
            writer.shutdown().await
        },
        async {
            tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await?;
            Ok::<_, std::io::Error>(())
        }
    )?;
    Ok(())
}

/// Only these requests create/rebind the TUI's own thread. Other clients' notifications are ignored.
#[derive(Default)]
struct ThreadBinding {
    pending: HashSet<String>,
    thread: Option<String>,
    awaiting_turn: bool,
    turn: Option<String>,
    pending_turn: HashSet<String>,
}
impl ThreadBinding {
    fn outgoing(&mut self, value: &mut Value, config: &Value) -> Option<bool> {
        if value["method"] == "initialize" {
            // The TUI can opt out of status notifications it doesn't render; the relay needs them.
            if let Some(methods) = value["params"]["capabilities"]["optOutNotificationMethods"].as_array_mut() {
                methods.retain(|method| !matches!(method.as_str(), Some("thread/status/changed" | "turn/started" | "turn/completed")));
            }
        }
        if matches!(
            value["method"].as_str(),
            Some("thread/start" | "thread/resume" | "thread/fork")
        ) {
            let Some(id) = value.get("id") else {
                return None;
            };
            self.pending.insert(id.to_string());
            self.thread = None;
            self.turn = None;
            let Some(params) = value.get_mut("params").and_then(Value::as_object_mut) else {
                return None;
            };
            let overrides = params.entry("config").or_insert_with(|| json!({}));
            if overrides.is_null() {
                *overrides = json!({});
            }
            if let Some(overrides) = overrides.as_object_mut() {
                overrides.insert(format!("mcp_servers.{SERVER}"), config.clone());
            }
            return Some(false);
        }
        let starting_turn = value["method"] == "turn/start"
            && self
                .thread
                .as_deref()
                .is_some_and(|id| value["params"]["threadId"] == id);
        if starting_turn {
            self.awaiting_turn = false;
            self.turn = Some(String::new());
            if let Some(id) = value.get("id") { self.pending_turn.insert(id.to_string()); }
        }
        starting_turn.then_some(false)
    }
    #[cfg(test)]
    fn incoming(&mut self, value: &Value) -> Option<bool> {
        self.incoming_status(value).map(|status| status.is_idle())
    }
    fn incoming_status(&mut self, value: &Value) -> Option<crate::readiness::Activity> {
        use crate::readiness::Activity;
        let status = |status: &Value, awaiting_turn: bool| match status["type"].as_str() {
            Some("idle") if !awaiting_turn => Activity::Idle,
            Some("active") if status["activeFlags"].as_array().is_some_and(|flags| flags.iter().any(|flag| flag == "waitingOnApproval")) => Activity::WaitingApproval,
            Some("active") if status["activeFlags"].as_array().is_some_and(|flags| flags.iter().any(|flag| flag == "waitingOnUserInput")) => Activity::WaitingInput,
            Some("systemError") => Activity::Error,
            Some("notLoaded") => Activity::Starting,
            _ => Activity::Working,
        };
        if value.get("id").is_some_and(|id| self.pending_turn.remove(&id.to_string())) {
            if let Some(turn) = value["result"]["turn"]["id"].as_str() {
                self.turn = Some(turn.into());
            }
        }
        if value
            .get("id")
            .is_some_and(|id| self.pending.remove(&id.to_string()))
        {
            let thread = &value["result"]["thread"];
            if let Some(id) = thread["id"].as_str() {
                self.thread = Some(id.to_owned());
                if thread["status"]["type"] == "active" {
                    self.awaiting_turn = false;
                }
                return Some(status(&thread["status"], self.awaiting_turn));
            }
        }
        if value["method"] == "thread/status/changed"
            && self
                .thread
                .as_deref()
                .is_some_and(|id| value["params"]["threadId"] == id)
        {
            if value["params"]["status"]["type"] == "active" {
                self.awaiting_turn = false;
            }
            return Some(status(&value["params"]["status"], self.awaiting_turn));
        }
        if self.thread.as_deref().is_some_and(|id| value["params"]["threadId"] == id) {
            match value["method"].as_str() {
                Some("turn/started") => {
                    self.awaiting_turn = false;
                    self.turn = value["params"]["turn"]["id"].as_str().map(str::to_owned);
                    return Some(Activity::Working);
                }
                Some("turn/completed") if !self.awaiting_turn && self.turn.as_deref().is_some_and(|id| value["params"]["turn"]["id"] == id) => {
                    self.turn = None;
                    return Some(Activity::Idle);
                }
                Some(method) if method.ends_with("/requestApproval") => return Some(Activity::WaitingApproval),
                Some("item/tool/requestUserInput" | "mcpServer/elicitation/request") => return Some(Activity::WaitingInput),
                _ => {}
            }
        }
        None
    }
}

/// Reuse the native daemon's authenticated control connection without restarting it.
async fn codex_proxy(
    listener: tokio::net::TcpListener,
    token: String,
    executable: PathBuf,
    config: Value,
    bridge: Bridge,
    initial_work: bool,
) -> Result<()> {
    // Match Codex's native remote transport limit so existing large transcripts still work.
    let ws_config = WebSocketConfig {
        max_message_size: Some(128 << 20),
        max_frame_size: Some(128 << 20),
        ..Default::default()
    };
    let downstream = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let (stream, _) = listener.accept().await?;
            let authorization = format!("Bearer {token}");
            let callback = move |request: &Request, response: Response| {
                if request
                    .headers()
                    .get("authorization")
                    .and_then(|value| value.to_str().ok())
                    == Some(authorization.as_str())
                {
                    Ok(response)
                } else {
                    Err(async_tungstenite::tungstenite::http::Response::builder()
                        .status(401)
                        .body(None)
                        .unwrap())
                }
            };
            if let Ok(Ok(socket)) = tokio::time::timeout(
                Duration::from_secs(2),
                accept_hdr_async_with_config(stream, callback, Some(ws_config)),
            )
            .await
            {
                break Ok::<_, std::io::Error>(socket);
            }
        }
    })
    .await
    .map_err(|_| anyhow!("Native session connection timed out"))??;
    let mut proxy = Command::new(executable)
        .args(["app-server", "proxy"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let mut output = proxy
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Native daemon unavailable"))?;
    let mut input = proxy
        .stdin
        .take()
        .ok_or_else(|| anyhow!("Native daemon unavailable"))?;
    let (transport, socket) = tokio::io::duplex(65536);
    let mut tasks = tokio::task::JoinSet::new();
    tasks.spawn(async move {
        let (mut reader, mut writer) = tokio::io::split(transport);
        let _ = tokio::try_join!(
            tokio::io::copy(&mut output, &mut writer),
            tokio::io::copy(&mut reader, &mut input)
        );
        let _ = proxy.kill().await;
    });
    let result = async {
        let (mut upstream, _) = tokio::time::timeout(Duration::from_secs(5), client_async_with_config("ws://localhost/", socket, Some(ws_config)))
            .await.map_err(|_| anyhow!("Native daemon handshake timed out"))??;
        let mut downstream = downstream;
        let mut binding = ThreadBinding { awaiting_turn: initial_work, ..Default::default() };
        let context = bridge.clone();
        let (activity, mut updates) = tokio::sync::mpsc::channel::<(crate::readiness::Activity, Option<tokio::sync::oneshot::Sender<()>>)>(32);
        tasks.spawn(async move {
            while let Some((status, done)) = updates.recv().await {
                let bridge = bridge.clone();
                let _ = tokio::task::spawn_blocking(move || bridge.native_status(status)).await;
                if let Some(done) = done { let _ = done.send(()); }
            }
        });
        let result = async {
            loop {
                tokio::select! {
                    message = downstream.next() => {
                        let Some(message) = message else { break; };
                        let mut message = message?;
                        if matches!(message, Message::Close(_)) { break; }
                        if matches!(message, Message::Ping(_) | Message::Pong(_)) { downstream.flush().await?; continue; }
                        if let Message::Text(text) = &message {
                            let mut value: Value = serde_json::from_str(text)?;
                            if let Some(ready) = binding.outgoing(&mut value, &config) {
                                // Establish busy before forwarding: discovery can precede the thread reply.
                                // Use the same FIFO as incoming events so a queued older idle cannot win.
                                let (done, applied) = tokio::sync::oneshot::channel();
                                let status = if ready { crate::readiness::Activity::Idle } else { crate::readiness::Activity::Working };
                                let _ = activity.send((status, Some(done))).await;
                                let _ = applied.await;
                                if let Some(directory) = value["params"]["cwd"].as_str() { let _ = context.native_directory(directory); }
                            }
                            message = Message::Text(value.to_string());
                        }
                        upstream.send(message).await?;
                    }
                    message = upstream.next() => {
                        let Some(message) = message else { break; };
                        let message = message?;
                        if matches!(message, Message::Close(_)) { break; }
                        if matches!(message, Message::Ping(_) | Message::Pong(_)) { upstream.flush().await?; continue; }
                        if let Message::Text(text) = &message {
                            let value: Value = serde_json::from_str(text)?;
                            if let Some(status) = binding.incoming_status(&value) { let _ = activity.send((status, None)).await; }
                        }
                        downstream.send(message).await?;
                    }
                }
            }
            Ok::<_, anyhow::Error>(())
        }.await;
        result
    }.await;
    result
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CodexMode {
    Native,
    Embedded,
    Shared,
}

/// Reuse the installed option contract so global arguments cannot hide a batch subcommand.
pub fn codex_accepts_peer_prompt(args: &[String], options: &crate::launch::LaunchOptions) -> bool {
    codex_launch_mode(
        &args.iter().map(OsString::from).collect::<Vec<_>>(),
        options,
    )
    .0 != CodexMode::Native
}

fn codex_launch_mode(
    args: &[OsString],
    options: &crate::launch::LaunchOptions,
) -> (CodexMode, bool) {
    use crate::launch::Arity;
    let mut args = args.iter().peekable();
    let mut first_positional = true;
    let mut mode = CodexMode::Shared;
    let mut positional = Vec::new();
    let mut last = false;
    let mut initial_work = false;
    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy();
        if arg == "--" {
            initial_work |= args.peek().is_some();
            break;
        }
        if !arg.starts_with('-') {
            if first_positional
                && matches!(
                    arg.as_ref(),
                    "exec"
                        | "e"
                        | "review"
                        | "login"
                        | "logout"
                        | "mcp"
                        | "plugin"
                        | "app-server"
                        | "remote-control"
                        | "app"
                        | "completion"
                        | "update"
                        | "doctor"
                        | "sandbox"
                        | "debug"
                        | "apply"
                        | "a"
                        | "queue"
                        | "archive"
                        | "delete"
                        | "migrate-rollouts"
                        | "unarchive"
                        | "cloud"
                        | "cloud-tasks"
                        | "exec-server"
                        | "features"
                        | "help"
                        | "tcp-tunnel"
                        | "execpolicy"
                        | "responses-api-proxy"
                        | "stdio-to-uds"
                )
            {
                return (CodexMode::Native, false);
            }
            positional.push(arg.into_owned());
            first_positional = false;
            continue;
        }
        let (mut name, mut inline) = arg
            .split_once('=')
            .map_or((arg.as_ref(), None), |(name, value)| (name, Some(value)));
        if !options.0.contains_key(name)
            && name.is_ascii()
            && !name.starts_with("--")
            && name.len() > 2
        {
            if options.0.get(&name[..2]).is_some_and(|arity| {
                matches!(
                    arity,
                    Arity::Value | Arity::Values | Arity::BlockedValue | Arity::BlockedValues
                )
            }) {
                inline = Some(&arg[2..]);
                name = &arg[..2];
            }
        }
        // Preserve native backend selection while binding the embedded client's MCP separately.
        if matches!(
            name,
            "--remote" | "--remote-auth-token-env" | "--help" | "-h" | "--version" | "-V"
        ) {
            return (CodexMode::Native, false);
        }
        if matches!(
            name,
            "--no-daemon"
                | "--oss"
                | "--profile"
                | "-p"
                | "--config"
                | "-c"
                | "--search"
                | "--strict-config"
                | "--dangerously-bypass-hook-trust"
        ) {
            mode = CodexMode::Embedded;
        }
        last |= name == "--last";
        initial_work |= matches!(name, "--image" | "-i");
        let Some(arity) = options.0.get(name) else {
            return (CodexMode::Native, false);
        };
        let value = match arity {
            Arity::Value | Arity::Values | Arity::BlockedValue | Arity::BlockedValues => {
                let value = inline.map(str::to_owned).or_else(|| {
                    args.next()
                        .map(|value| value.to_string_lossy().into_owned())
                });
                let Some(value) = value else {
                    return (CodexMode::Native, false);
                };
                if matches!(arity, Arity::Values | Arity::BlockedValues) {
                    while args
                        .peek()
                        .is_some_and(|value| !value.to_string_lossy().starts_with('-'))
                    {
                        args.next();
                    }
                }
                Some(value)
            }
            Arity::OptionalValue | Arity::BlockedOptionalValue => {
                if inline.is_none()
                    && args
                        .peek()
                        .is_some_and(|value| !value.to_string_lossy().starts_with('-'))
                {
                    args.next();
                }
                inline.map(str::to_owned)
            }
            _ => None,
        };
        if matches!(name, "--enable" | "--disable")
            && !value.as_deref().is_some_and(|value| {
                matches!(
                    value,
                    "daemon_auto_start"
                        | "worktrees"
                        | "transcript_v2"
                        | "realtime_conversation"
                        | "standalone_web_search"
                        | "remote_models"
                        | "request_rule"
                        | "responses_websockets_v2"
                        | "workspace_owner_usage_nudge"
                        | "tool_search_always_defer_mcp_tools"
                        | "remote_compaction_v2"
                        | "multi_agent_mode"
                )
            })
        {
            if value.as_deref().is_some_and(|value| {
                matches!(
                    value,
                    "api_key_model_discovery"
                        | "code_mode_host"
                        | "auth_elicitation"
                        | "mcp_oauth_refresh_coordination"
                )
            }) {
                return (CodexMode::Native, false);
            }
            mode = CodexMode::Embedded;
        }
        if name == "--disable" && value.as_deref() == Some("daemon_auto_start") {
            return (CodexMode::Native, false);
        }
    }
    // Native resume/fork shifts SESSION_ID into PROMPT when --last is set.
    initial_work |= match positional.first().map(String::as_str) {
        None => false,
        Some("agents") => positional.len() > 1,
        Some("resume" | "fork") => positional.len() > if last { 1 } else { 2 },
        Some(_) => true,
    };
    (mode, initial_work)
}

pub async fn launch(binding: &NativeLaunch, mut args: Vec<OsString>) -> Result<i32> {
    let executable = &binding.executable;
    let name = binding.program.as_str();
    let mut options = binding.options.clone();
    // Restored panes can precede settings discovery when upgrading metadata without option contracts.
    if name == "codex" && options.0.is_empty() {
        for subcommand in [None, Some("resume"), Some("fork")] {
            let mut help = Command::new(executable);
            if let Some(subcommand) = subcommand {
                help.arg(subcommand);
            }
            if let Ok(Ok(output)) = tokio::time::timeout(
                Duration::from_secs(3),
                help.arg("--help")
                    .stdin(Stdio::null())
                    .stderr(Stdio::null())
                    .kill_on_drop(true)
                    .output(),
            )
            .await
            {
                if output.status.success() {
                    options.0.extend(
                        crate::launch::LaunchOptions::from_help(
                            "codex",
                            &String::from_utf8_lossy(&output.stdout),
                        )
                        .0,
                    );
                }
            }
        }
    }
    let companion = std::env::var_os("WARP_AGENT_BIN").map(PathBuf::from);
    let bridge = Bridge::from_env().ok();
    let (mut codex_mode, initial_work) = if name == "codex" {
        codex_launch_mode(&args, &options)
    } else {
        (CodexMode::Native, false)
    };
    if codex_mode == CodexMode::Shared && std::env::var_os("CODEX_EXEC_SERVER_URL").is_some() {
        codex_mode = CodexMode::Embedded;
    }
    let shared_codex = name == "codex"
        && companion.is_some()
        && bridge.is_some()
        && codex_mode == CodexMode::Shared;
    let relay = if (name == "codex" && codex_mode != CodexMode::Native
        || matches!(
            name,
            "claude" | "qoder" | "qodercli" | "qoder-cli" | "qodercn" | "qoderclicn"
        ))
        && bridge.is_some()
        && companion.is_some()
    {
        Some(Relay::start(bridge.as_ref().unwrap().clone()).await?)
    } else {
        None
    };
    let mut tasks = tokio::task::JoinSet::new();
    let mut command = Command::new(executable);
    if shared_codex {
        let mut daemon = Command::new(executable);
        without_terminal_binding(&mut daemon);
        let status = tokio::time::timeout(
            Duration::from_secs(15),
            daemon
                .args(["app-server", "daemon", "start"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .status(),
        )
        .await
        .map_err(|_| anyhow!("Native daemon startup timed out"))??;
        ensure!(
            status.success(),
            "Native daemon could not start; existing processes were preserved"
        );
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
        let endpoint = format!("ws://{}", listener.local_addr()?);
        let token = Uuid::new_v4().to_string();
        command.env(PROXY_TOKEN, &token);
        // Proxy-aware Codex builds must reach this launch's loopback listener directly.
        for (name, fallback) in [("NO_PROXY", "no_proxy"), ("no_proxy", "NO_PROXY")] {
            command.env(
                name,
                codex_no_proxy(std::env::var_os(name), std::env::var_os(fallback)),
            );
        }
        let mut config = relay.as_ref().unwrap().config(companion.as_ref().unwrap());
        config["env_vars"] = json!([]);
        tasks.spawn(codex_proxy(
            listener,
            token,
            executable.to_owned(),
            config,
            bridge.unwrap(),
            initial_work,
        ));
        // Global options precede both positional prompts and resume/fork subcommands.
        let mut remote = vec![
            "--remote".into(),
            endpoint.into(),
            "--remote-auth-token-env".into(),
            PROXY_TOKEN.into(),
        ];
        remote.append(&mut args);
        args = remote;
    } else if let Some(relay) = &relay {
        let config = relay.config(companion.as_ref().unwrap());
        let mut native = if name == "codex" {
            ["command", "args"]
                .into_iter()
                .flat_map(|key| {
                    [
                        OsString::from("-c"),
                        format!("mcp_servers.{SERVER}.{key}={}", config[key]).into(),
                    ]
                })
                .chain([
                    "-c".into(),
                    format!("mcp_servers.{SERVER}.env_vars=[]").into(),
                ])
                .collect::<Vec<_>>()
        } else {
            vec![
                "--mcp-config".into(),
                json!({"mcpServers": {SERVER: config}}).to_string().into(),
            ]
        };
        native.append(&mut args);
        args = native;
    }
    let mut child = command.args(args).kill_on_drop(true).spawn()?;
    let result = if !tasks.is_empty() {
        tokio::select! {
            result = child.wait() => result?,
            result = tasks.join_next() => {
                match tokio::time::timeout(Duration::from_secs(3), child.wait()).await {
                    Ok(status) => status?,
                    Err(_) => {
                        let _ = child.kill().await;
                        result.ok_or_else(|| anyhow!("Native session proxy unavailable"))?
                            .map_err(|_| anyhow!("Native session proxy unavailable"))??;
                        return Err(anyhow!("Native session connection closed"));
                    }
                }
            }
        }
    } else {
        child.wait().await?
    };
    drop(relay);
    Ok(result.code().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_turn_completion_recovers_and_interactive_waits_remain_distinct() {
        use crate::readiness::Activity;
        let mut binding = ThreadBinding::default();
        let mut initialize = json!({"method":"initialize","params":{"capabilities":{"optOutNotificationMethods":["thread/status/changed","turn/started","turn/completed","item/started"]}}});
        assert_eq!(binding.outgoing(&mut initialize, &json!({})), None);
        assert_eq!(initialize["params"]["capabilities"]["optOutNotificationMethods"], json!(["item/started"]));
        binding.outgoing(&mut json!({"id":1,"method":"thread/start","params":{}}), &json!({}));
        assert_eq!(binding.incoming_status(&json!({"id":1,"result":{"thread":{"id":"mine","status":{"type":"idle"}}}})), Some(Activity::Idle));
        for turn in ["one", "two"] {
            binding.outgoing(&mut json!({"id":2,"method":"turn/start","params":{"threadId":"mine"}}), &json!({}));
            assert_eq!(binding.incoming_status(&json!({"method":"turn/completed","params":{"threadId":"mine","turn":{"id":"stale"}}})), None);
            assert_eq!(binding.incoming_status(&json!({"method":"turn/started","params":{"threadId":"mine","turn":{"id":turn}}})), Some(Activity::Working));
            assert_eq!(binding.incoming_status(&json!({"method":"thread/status/changed","params":{"threadId":"mine","status":{"type":"active","activeFlags":["waitingOnApproval"]}}})), Some(Activity::WaitingApproval));
            assert_eq!(binding.incoming_status(&json!({"method":"item/tool/requestUserInput","params":{"threadId":"mine"}})), Some(Activity::WaitingInput));
            assert_eq!(binding.incoming_status(&json!({"method":"turn/completed","params":{"threadId":"other","turn":{"id":turn}}})), None);
            assert_eq!(binding.incoming_status(&json!({"method":"turn/completed","params":{"threadId":"mine","turn":{"id":turn,"status":"completed"}}})), Some(Activity::Idle));
        }
        assert_eq!(binding.incoming_status(&json!({"method":"thread/status/changed","params":{"threadId":"mine","status":{"type":"systemError"}}})), Some(Activity::Error));
    }
    #[test]
    fn codex_loopback_bypasses_proxy_without_losing_user_exclusions() {
        for (primary, fallback, expected) in [
            (None, None, "127.0.0.1"),
            (Some(""), Some("internal.example"), "127.0.0.1"),
            (None, Some("internal.example"), "internal.example,127.0.0.1"),
            (Some("internal.example"), None, "internal.example,127.0.0.1"),
            (
                Some("upper.example"),
                Some("lower.example"),
                "upper.example,127.0.0.1",
            ),
            (Some("*"), None, "*,127.0.0.1"),
        ] {
            assert_eq!(
                codex_no_proxy(primary.map(OsString::from), fallback.map(OsString::from)),
                OsString::from(expected)
            );
        }
    }

    #[test]
    fn shared_launcher_catalog_supersedes_old_pane_snapshots() {
        let directory = tempfile::tempdir().unwrap();
        let catalog = directory.path().join(LAUNCH_CATALOG);
        let launch = |program: &str| NativeLaunch {
            executable: PathBuf::from(format!("/native/{program}")),
            program: program.into(),
            options: Default::default(),
        };
        let snapshot =
            serde_json::to_string(&BTreeMap::from([("codex", launch("codex"))])).unwrap();
        let mut current =
            BTreeMap::from([("codex", launch("codex")), ("qodercn", launch("qodercn"))]);
        std::fs::write(&catalog, serde_json::to_vec(&current).unwrap()).unwrap();
        assert_eq!(
            resolve_native_executable("qodercn", Some(&catalog), Some(&snapshot))
                .unwrap()
                .program,
            "qodercn"
        );
        current.get_mut("codex").unwrap().program = "custom".into();
        std::fs::write(&catalog, serde_json::to_vec(&current).unwrap()).unwrap();
        let disabled = resolve_native_executable("codex", Some(&catalog), Some(&snapshot)).unwrap();
        assert_eq!(disabled.program, "custom");
        assert_eq!(disabled.executable, Path::new("/native/codex"));
        assert!(resolve_native_executable("warpai-agent", Some(&catalog), Some(&snapshot)).is_none());
        #[cfg(windows)]
        assert_eq!(
            resolve_native_executable("QODERCN.EXE", Some(&catalog), Some(&snapshot))
                .unwrap()
                .program,
            "qodercn"
        );
        std::fs::write(&catalog, b"invalid").unwrap();
        assert_eq!(
            resolve_native_executable("codex", Some(&catalog), Some(&snapshot))
                .unwrap()
                .program,
            "custom"
        );
        assert_eq!(
            resolve_native_executable("codex", None, Some(&snapshot))
                .unwrap()
                .program,
            "codex"
        );
    }

    #[test]
    fn codex_launch_preserves_argument_values_and_native_modes() {
        let options = crate::launch::LaunchOptions::from_help("codex", "  --no-daemon  Embedded\n  -m, --model <MODEL>  Model\n  -p, --profile <PROFILE>  Profile\n  -c, --config <KEY>  Config\n  --enable <FEATURE>  Enable\n  --disable <FEATURE>  Disable\n  --last  Last\n  --all  All\n  --add-dir <DIR>  Directory\n  -i, --image <FILE>...  Images");
        for (args, initial_work) in [
            ("--yolo -m exec", false),
            ("task", true),
            ("-- task", true),
            ("resume --last", false),
            ("resume session-id", false),
            ("resume --last task", true),
            ("resume session-id task", true),
            ("fork --all", false),
            ("fork session-id task", true),
            ("--image mcp.png", true),
        ] {
            let args = args
                .split_whitespace()
                .map(OsString::from)
                .collect::<Vec<_>>();
            assert_eq!(
                codex_launch_mode(&args, &options).1,
                initial_work,
                "{args:?}"
            );
        }
        for args in [
            "",
            "--yolo",
            "--yolo -m exec",
            "--model=mcp --add-dir help",
            "-mapp-server",
            "resume --last",
            "--yolo fork --all",
            "--enable worktrees",
            "--image mcp.png --yolo task",
            "-- exec",
        ] {
            let args = args
                .split_whitespace()
                .map(OsString::from)
                .collect::<Vec<_>>();
            assert_eq!(
                codex_launch_mode(&args, &options).0,
                CodexMode::Shared,
                "{args:?}"
            );
        }
        for args in ["--no-daemon --yolo", "-pprofile", "-cmodel=x"] {
            let args = args
                .split_whitespace()
                .map(OsString::from)
                .collect::<Vec<_>>();
            assert_eq!(
                codex_launch_mode(&args, &options).0,
                CodexMode::Embedded,
                "{args:?}"
            );
        }
        for args in [
            "exec task",
            "--yolo mcp list",
            "--model model app-server",
            "--enable code_mode_host",
            "--disable code_mode_host",
            "--remote ws://localhost",
            "--unknown",
            "--help",
        ] {
            let words = args
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert!(!codex_accepts_peer_prompt(&words, &options), "{args}");
            let args = args
                .split_whitespace()
                .map(OsString::from)
                .collect::<Vec<_>>();
            assert_ne!(
                codex_launch_mode(&args, &options).0,
                CodexMode::Shared,
                "{args:?}"
            );
        }
    }

    #[tokio::test]
    async fn shared_transport_keeps_every_agent_terminal_isolated() {
        use crate::{
            transport::{self, Request, RunningBroker},
            Operation,
        };
        use rmcp::model::CallToolRequestParam;
        let server = RunningBroker::start(Path::new(":memory:")).unwrap();
        let issuer_capability = server.broker.prepare("issuer").unwrap();
        server
            .broker
            .activate("issuer", "codex", "/project", false)
            .unwrap();
        let mut issuer = Request {
            protocol_major: crate::transport::PROTOCOL_MAJOR,
            terminal: "issuer".into(),
            capability: issuer_capability,
            run: None,
            defer_initial_ready: false,
            native_activity: None,
            directory: None,
            operation: Operation::AgentRegister {
                name: "issuer".into(),
            },
        };
        let endpoint = server.broker.endpoint.clone();
        issuer = tokio::task::spawn_blocking(move || {
            issuer.run = transport::call(&endpoint, &issuer).unwrap()["run"]
                .as_str()
                .map(str::to_owned);
            issuer
        })
        .await
        .unwrap();
        let mut programs: Vec<_> = include_str!("../../../app/src/terminal/cli_agent.rs")
            .lines()
            .filter_map(|line| line.trim().strip_prefix("CLIAgent::"))
            .filter(|line| line.contains("=> &[\""))
            .filter_map(|line| line.split('"').nth(1))
            .collect();
        programs.push("custom");
        assert!(programs.len() >= 19);
        for (index, program) in programs.into_iter().enumerate() {
            let mut panes = Vec::new();
            for copy in 0..2 {
                let terminal = format!("pane-{index}-{copy}");
                let capability = server.broker.prepare(&terminal).unwrap();
                server
                    .broker
                    .activate(&terminal, program, "/project", true)
                    .unwrap();
                let bridge = Bridge::test_binding(
                    server.broker.endpoint.clone(),
                    terminal.clone(),
                    capability.clone(),
                );
                bridge.native_activity(true).unwrap();
                assert!(server
                    .broker
                    .peers("issuer")
                    .iter()
                    .all(|peer| peer.terminal != terminal));
                let relay = Relay::start(bridge.clone()).await.unwrap();
                let config = relay.config(Path::new("/companion")).to_string();
                assert!(!config.contains(&capability));
                assert!(!config.contains(&terminal));
                // Both clients run in this same process; neither identity comes from its environment.
                let mut stream = transport::connect(&relay.endpoint).await.unwrap();
                transport::send(
                    &mut stream,
                    &ForwardContext { directory: None },
                    std::time::Instant::now() + Duration::from_secs(5),
                )
                .await
                .unwrap();
                let client = ().serve(stream).await.unwrap();
                assert_eq!(client.list_tools(None).await.unwrap().tools.len(), 30);
                let peer = server
                    .broker
                    .peers("issuer")
                    .into_iter()
                    .find(|peer| peer.terminal == terminal)
                    .unwrap();
                let mut send = issuer.clone();
                send.operation = Operation::AgentSend {
                    to: peer.name,
                    body: terminal.clone(),
                    subject: None,
                    thread_id: None,
                    reply_to: None,
                    task_id: None,
                    request_id: Uuid::new_v4().to_string(),
                };
                let endpoint = server.broker.endpoint.clone();
                tokio::task::spawn_blocking(move || transport::call(&endpoint, &send).unwrap())
                    .await
                    .unwrap();
                let inbox = client
                    .call_tool(CallToolRequestParam {
                        name: "warp_agent_inbox".into(),
                        arguments: None,
                    })
                    .await
                    .unwrap();
                assert_ne!(inbox.is_error, Some(true));
                let inbox = serde_json::to_string(&inbox.content).unwrap();
                assert!(inbox.contains(&terminal));
                assert!(!inbox.contains(&format!("pane-{index}-{}", 1 - copy)));
                let ready = bridge.clone();
                tokio::task::spawn_blocking(move || ready.native_activity(true).unwrap())
                    .await
                    .unwrap();
                if copy == 1 {
                    server.broker.user_input(&terminal, false);
                }
                panes.push((relay, bridge, client, terminal));
            }
            tokio::time::sleep(Duration::from_millis(850)).await;
            let wakes = server.broker.wakeups();
            assert_eq!(wakes.len(), 1);
            assert_eq!(wakes[0].terminal, panes[0].3);
            let busy = panes[0].1.clone();
            tokio::task::spawn_blocking(move || busy.native_activity(false).unwrap())
                .await
                .unwrap();
            let premature = panes[0]
                .2
                .call_tool(CallToolRequestParam {
                    name: "warp_agent_ready".into(),
                    arguments: None,
                })
                .await
                .unwrap();
            assert!(serde_json::to_string(&premature.content)
                .unwrap()
                .contains("\\\"ready\\\":false"));
            assert_eq!(panes[0].2.list_tools(None).await.unwrap().tools.len(), 30);
            assert!(
                server.broker.wakeups().is_empty(),
                "Rediscovery cannot replay an old idle notification"
            );
            for (_relay, bridge, client, terminal) in panes {
                server.broker.end(&terminal);
                server
                    .broker
                    .activate(&terminal, program, "/project", false)
                    .unwrap();
                assert!(
                    client.list_tools(None).await.is_err(),
                    "A replacement cannot inherit an old relay identity"
                );
                tokio::task::spawn_blocking(move || assert!(bridge.native_activity(true).is_err()))
                    .await
                    .unwrap();
                client.cancel().await.unwrap();
                server.broker.end(&terminal);
            }
        }
    }
    #[test]
    fn initial_task_does_not_become_ready_between_thread_and_turn_start() {
        let mut binding = ThreadBinding {
            awaiting_turn: true,
            ..Default::default()
        };
        let config = json!({});
        assert_eq!(
            binding.outgoing(
                &mut json!({"id":1,"method":"thread/start","params":{}}),
                &config
            ),
            Some(false)
        );
        assert_eq!(
            binding.incoming(
                &json!({"id":1,"result":{"thread":{"id":"mine","status":{"type":"idle"}}}})
            ),
            Some(false)
        );
        assert_eq!(binding.incoming(&json!({"method":"thread/status/changed","params":{"threadId":"other","status":{"type":"active"}}})), None);
        assert_eq!(binding.incoming(&json!({"method":"thread/status/changed","params":{"threadId":"mine","status":{"type":"idle"}}})), Some(false));
        assert_eq!(
            binding.outgoing(
                &mut json!({"id":2,"method":"turn/start","params":{"threadId":"mine"}}),
                &config
            ),
            Some(false)
        );
        assert_eq!(binding.incoming(&json!({"method":"thread/status/changed","params":{"threadId":"mine","status":{"type":"idle"}}})), Some(true));
    }

    #[test]
    fn session_overrides_are_thread_local_and_status_is_owned() {
        let mut binding = ThreadBinding::default();
        let config =
            json!({"command":"/bridge", "args":["forward","/private/mcp.sock"], "env_vars":[]});
        let mut request = json!({"id":7,"method":"thread/start","params":{"cwd":"/project","sandbox":"danger-full-access","config":{"model_reasoning_effort":"high","mcp_servers.other":{"command":"other"}}}});
        assert_eq!(binding.outgoing(&mut request, &config), Some(false));
        assert_eq!(
            request["params"]["config"]["mcp_servers.warp-lite-communication"],
            config
        );
        assert_eq!(request["params"]["sandbox"], "danger-full-access");
        assert_eq!(
            request["params"]["config"]["mcp_servers.other"]["command"],
            "other"
        );
        assert_eq!(
            binding.incoming(
                &json!({"id":8,"result":{"thread":{"id":"other","status":{"type":"idle"}}}})
            ),
            None
        );
        assert_eq!(
            binding.incoming(
                &json!({"id":7,"result":{"thread":{"id":"mine","status":{"type":"idle"}}}})
            ),
            Some(true)
        );
        assert_eq!(binding.incoming(&json!({"method":"thread/status/changed","params":{"threadId":"other","status":{"type":"idle"}}})), None);
        assert_eq!(binding.incoming(&json!({"method":"thread/status/changed","params":{"threadId":"mine","status":{"type":"active","activeFlags":["waitingOnApproval"]}}})), Some(false));
        assert_eq!(
            binding.outgoing(
                &mut json!({"id":10,"method":"turn/start","params":{"threadId":"other"}}),
                &config
            ),
            None
        );
        assert_eq!(
            binding.outgoing(
                &mut json!({"id":11,"method":"turn/start","params":{"threadId":"mine"}}),
                &config
            ),
            Some(false)
        );
        let mut resume =
            json!({"id":9,"method":"thread/resume","params":{"threadId":"new","config":null}});
        assert_eq!(binding.outgoing(&mut resume, &config), Some(false));
        assert_eq!(binding.incoming(&json!({"method":"thread/status/changed","params":{"threadId":"mine","status":{"type":"idle"}}})), None);
        assert_eq!(
            binding.incoming(
                &json!({"id":9,"result":{"thread":{"id":"new","status":{"type":"idle"}}}})
            ),
            Some(true)
        );
        assert_eq!(binding.incoming(&json!({"method":"thread/status/changed","params":{"threadId":"mine","status":{"type":"idle"}}})), None);
    }
}
