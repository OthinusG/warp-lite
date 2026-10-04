//! Per-launch MCP transport binding, independent of a vendor's shared process environment.
use crate::mcp::Bridge;
use anyhow::{anyhow, ensure, Result};
use rmcp::ServiceExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{io::AsyncWriteExt, process::Command};
use uuid::Uuid;

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
        "WARP_AGENT_LAUNCHES",
        "WARP_AGENT_BIN",
        "WARP_AGENT_LAUNCH_PATH",
        "WARP_AGENT_SESSION_TOKEN",
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

/// Native MCP clients receive only this private IPC address, never terminal capabilities.
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
                    let Ok(transport) =
                        crate::mcp::legacy_transport(tokio::io::split(stream)).await
                    else {
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

/// Explicit remote launches execute the installed client with its documented MCP options.
pub async fn launch(binding: &NativeLaunch, mut args: Vec<OsString>) -> Result<i32> {
    let executable = &binding.executable;
    let name = binding.program.as_str();
    let companion = std::env::var_os("WARP_AGENT_BIN").map(PathBuf::from);
    let bridge = Bridge::from_env().ok();
    let codex_interactive =
        name == "codex" && codex_launch_mode(&args, &binding.options).0 != CodexMode::Native;
    if codex_interactive && bridge.is_some() {
        ensure!(
            binding.options.0.contains_key("--no-daemon"),
            "Installed Codex lacks verified session isolation; launch the native CLI directly"
        );
    }
    let relay = if (codex_interactive || matches!(name, "claude" | "qoder" | "qodercn"))
        && companion.is_some()
        && bridge.is_some()
    {
        Some(Relay::start(bridge.unwrap()).await?)
    } else {
        None
    };
    if let Some(relay) = &relay {
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
        if name == "codex" && binding.options.0.contains_key("--no-daemon") {
            native.insert(0, "--no-daemon".into());
        }
        native.append(&mut args);
        args = native;
    }
    // Inherit the launcher's actual project directory; never force a repository or daemon frontend.
    let result = Command::new(executable)
        .args(args)
        .kill_on_drop(true)
        .status()
        .await?;
    drop(relay);
    Ok(result.code().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
