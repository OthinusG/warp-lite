//! Reversible native MCP configuration. Never persist live terminal capabilities.
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

pub const SERVER: &str = "warp-lite-communication";
const START: &str = "# BEGIN WARP LITE COMMUNICATION";
const END: &str = "# END WARP LITE COMMUNICATION";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LegacyRemoteProfile {
    pub alias: String,
    pub coordinator: Uuid,
    pub device: Uuid,
    pub generation: u64,
    pub spaces: Vec<Uuid>,
    #[serde(default = "cleanup_pending")]
    pub cleanup_pending: bool,
}
fn cleanup_pending() -> bool { true }

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Preferences {
    pub enabled: bool,
    pub selected: BTreeMap<String, Installed>,
    // Legacy metadata is retained for export/cleanup, never used for SSH admission.
    #[serde(default, alias = "remote_profiles", skip_serializing_if = "Vec::is_empty")]
    pub(super) legacy_remote_profiles: Vec<LegacyRemoteProfile>,
}
impl Preferences {
    pub fn programs(&self) -> std::collections::HashSet<String> {
        if !self.enabled {
            return std::collections::HashSet::new();
        }
        self.selected
            .values()
            .filter(|entry| entry.active)
            .map(|entry| entry.program.clone())
            .collect()
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Installed {
    pub active: bool,
    pub program: String,
    pub executable: PathBuf,
    pub adapter: Adapter,
    pub bridge: PathBuf,
    #[serde(default)]
    pub search_paths: Vec<PathBuf>,
    #[serde(default)]
    pub launch_options: warp_agent_bus::launch::LaunchOptions,
}
#[derive(Clone, Serialize, Deserialize)]
pub enum Adapter {
    Codex(PathBuf),
    Toml {
        path: PathBuf,
        value: Value,
    },
    Vibe(PathBuf),
    YamlList {
        path: PathBuf,
        key: String,
        value: Value,
    },
    Cordis {
        path: PathBuf,
        value: Value,
    },
    Yaml {
        path: PathBuf,
        key: String,
        value: Value,
    },
    Json {
        path: PathBuf,
        key: String,
        value: Value,
    },
    Cli {
        add: Vec<String>,
        remove: Vec<String>,
        get: Vec<String>,
        get_all: bool,
    },
}
#[derive(Clone)]
pub struct Available {
    pub command: String,
    pub program: String,
    pub installed: Option<Installed>,
    pub status: String,
}

pub fn companion() -> Result<PathBuf> {
    let path = std::env::current_exe()?.with_file_name(if cfg!(windows) {
        "warp-agent.exe"
    } else {
        "warp-agent"
    });
    ensure!(path.is_file(), "Bundled communication bridge is missing");
    Ok(path)
}
fn run(executable: &Path, args: &[String], search_paths: &[PathBuf]) -> Result<(bool, String)> {
    use tokio::io::AsyncReadExt;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(8), async {
                let path = std::env::join_paths(
                    executable
                        .parent()
                        .into_iter()
                        .map(Path::to_owned)
                        .chain(search_paths.iter().cloned())
                        .chain(std::env::split_paths(
                            &std::env::var_os("PATH").unwrap_or_default(),
                        )),
                )?;
                let mut child = tokio::process::Command::new(executable)
                    .args(args)
                    .env("PATH", path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .kill_on_drop(true)
                    .spawn()?;
                let mut stdout = child.stdout.take().unwrap().take(1024 * 1024);
                let mut stderr = child.stderr.take().unwrap().take(1024 * 1024);
                let mut out = Vec::new();
                let mut err = Vec::new();
                let (_, _, status) = tokio::try_join!(
                    stdout.read_to_end(&mut out),
                    stderr.read_to_end(&mut err),
                    child.wait()
                )?;
                Ok((
                    status.success(),
                    format!(
                        "{}\n{}",
                        String::from_utf8_lossy(&out),
                        String::from_utf8_lossy(&err)
                    ),
                ))
            })
            .await
            .map_err(|_| anyhow::anyhow!("Agent configuration command timed out"))?
        })
}
fn output(executable: &Path, args: &[String], search_paths: &[PathBuf]) -> Result<Option<String>> {
    let (success, text) = run(executable, args, search_paths)?;
    Ok(success.then_some(text))
}
fn executable(command: &str, search_paths: &[PathBuf], home: &Path) -> Option<PathBuf> {
    let mut paths: Vec<_> =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
    paths.splice(0..0, search_paths.iter().cloned());
    // GUI launches may not inherit the login shell's PATH.
    {
        paths.extend([
            home.join(".local/bin"),
            home.join(".cargo/bin"),
            home.join(".npm-global/bin"),
        ]);
        if matches!(command, "qoder" | "qodercli" | "qoder-cli") {
            paths.push(home.join(".qoder/entry"));
        }
        if matches!(command, "qodercn" | "qoderclicn") {
            paths.extend([
                home.join(".qoder-cn/entry"),
                home.join(".qoder-cn/bin/qoderclicn"),
            ]);
        }
    }
    #[cfg(unix)]
    paths.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]);
    for path in paths {
        for suffix in if cfg!(windows) {
            vec![".exe", ".cmd", ".bat", ""]
        } else {
            vec![""]
        } {
            let candidate = path.join(format!("{command}{suffix}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
pub fn discover(
    commands: Vec<(String, String)>,
    bridge: &Path,
    search_paths: Vec<PathBuf>,
) -> Vec<Available> {
    let Some(home) = dirs::home_dir() else {
        return vec![];
    };
    let mut seen = std::collections::HashSet::new();
    commands
        .into_iter()
        .filter_map(|(program, command)| {
            let executable = executable(&command, &search_paths, &home)?;
            // Aliases load the same vendor configuration; configure it only once.
            if program != "custom" && !seen.insert(program.clone()) {
                return None;
            }
            let adapter = adapter(&home, &command, &executable, bridge, &search_paths);
            let help = output(&executable, &["--help".into()], &search_paths)
                .ok().flatten().unwrap_or_default();
            let mut launch_options = warp_agent_bus::launch::LaunchOptions::from_help(&program, &help);
            if program == "codex" {
                for command in ["resume", "fork"] {
                    if let Ok(Some(help)) = output(&executable, &[command.into(), "--help".into()], &search_paths) {
                        launch_options.0.extend(warp_agent_bus::launch::LaunchOptions::from_help(&program, &help).0);
                    }
                }
            }
            let status = if adapter.is_some() {
                "Available"
            } else {
                "Automatic native MCP setup is unavailable for this installed version"
            }
            .into();
            Some(Available {
                command,
                program: program.clone(),
                status,
                installed: adapter.map(|adapter| Installed {
                    active: true,
                    program,
                    executable,
                    adapter,
                    bridge: bridge.to_owned(),
                    search_paths: search_paths.clone(),
                    launch_options,
                }),
            })
        })
        .collect()
}
fn adapter(
    home: &Path,
    command: &str,
    executable: &Path,
    bridge: &Path,
    search_paths: &[PathBuf],
) -> Option<Adapter> {
    let standard = json!({"command": bridge, "args": ["mcp"]});
    if matches!(command, "hermes" | "hermes-agent") {
        return Some(Adapter::Yaml {
            path: home.join(".hermes/config.yaml"),
            key: "mcp_servers".into(),
            value: json!({"command":bridge, "args":["mcp"], "env":{"WARP_AGENT_ENDPOINT":"${WARP_AGENT_ENDPOINT}", "WARP_AGENT_CAPABILITY":"${WARP_AGENT_CAPABILITY}", "WARP_TERMINAL_SESSION_UUID":"${WARP_TERMINAL_SESSION_UUID}"}}),
        });
    }
    if matches!(command, "vibe" | "vibe-acp") {
        return Some(Adapter::Vibe(home.join(".vibe/config.toml")));
    }
    if command == "grok" {
        return Some(Adapter::Toml {
            path: home.join(".grok/config.toml"),
            value: standard.clone(),
        });
    }
    if matches!(command, "dsh" | "dsh-tui") {
        return Some(Adapter::Cordis {
            path: home.join(".dsh/cordis.patch.yml"),
            value: json!({"id":SERVER, "name":"@deepseek-ai/dsh-mcp-client", "config":{"serverName":SERVER,"transport":"stdio","command":bridge,"args":["mcp"]}}),
        });
    }
    if matches!(
        command,
        "trae" | "traecn" | "trae-cli" | "traecn-cli" | "traecli"
    ) {
        let directory = if cfg!(target_os = "macos") {
            home.join("Library/Application Support")
        } else if cfg!(windows) {
            home.join("AppData/Roaming")
        } else {
            home.join(".config")
        };
        return Some(Adapter::YamlList {
            path: directory.join("trae_cli/trae_cli.yaml"),
            key: "mcp_servers".into(),
            value: json!({"name":SERVER,"type":"stdio","command":bridge,"args":["mcp"]}),
        });
    }
    if command == "goose" {
        return Some(Adapter::Yaml {
            path: home.join(".config/goose/config.yaml"),
            key: "extensions".into(),
            value: json!({"name":SERVER, "cmd":bridge, "args":["mcp"], "enabled":true, "type":"stdio", "timeout":300}),
        });
    }
    let json_config = match command {
        "claude" => Some((
            ".claude.json",
            "mcpServers",
            json!({"type":"stdio", "command":bridge, "args":["mcp"]}),
        )),
        "auggie" => Some((".augment/settings.json", "mcpServers", standard.clone())),
        "omp" => Some((".omp/agent/mcp.json", "mcpServers", standard.clone())),
        "pi" => {
            // Older versions require third-party extensions and are intentionally excluded.
            let help = output(
                executable,
                &["mcp".into(), "add".into(), "--help".into()],
                search_paths,
            )
            .ok()??;
            if !help.to_lowercase().contains("mcp add") {
                return None;
            }
            Some((".pi/agent/mcp.json", "mcpServers", standard.clone()))
        }
        "gemini" => Some((
            ".gemini/settings.json",
            "mcpServers",
            json!({"command":bridge, "args":["mcp"], "env":{"WARP_AGENT_ENDPOINT":"${WARP_AGENT_ENDPOINT}","WARP_AGENT_CAPABILITY":"${WARP_AGENT_CAPABILITY}","WARP_TERMINAL_SESSION_UUID":"${WARP_TERMINAL_SESSION_UUID}"}}),
        )),
        "opencode" => Some((
            ".config/opencode/opencode.json",
            "mcp",
            json!({"type":"local", "command":[bridge, "mcp"], "enabled":true}),
        )),
        "amp" => Some((
            ".config/amp/settings.json",
            "amp.mcpServers",
            standard.clone(),
        )),
        "agent" | "cursor-agent" => Some((
            ".cursor/mcp.json",
            "mcpServers",
            json!({"command":bridge, "args":["mcp"], "env":{"WARP_AGENT_ENDPOINT":"${env:WARP_AGENT_ENDPOINT}", "WARP_AGENT_CAPABILITY":"${env:WARP_AGENT_CAPABILITY}", "WARP_TERMINAL_SESSION_UUID":"${env:WARP_TERMINAL_SESSION_UUID}"}}),
        )),
        "copilot" => Some((
            ".copilot/mcp-config.json",
            "mcpServers",
            json!({"type":"local", "command":bridge, "args":["mcp"], "tools":["*"]}),
        )),
        "droid" => Some((
            ".factory/mcp.json",
            "mcpServers",
            json!({"type":"stdio", "command":bridge, "args":["mcp"], "disabled":false}),
        )),
        _ => None,
    };
    if command == "codex" {
        return Some(Adapter::Codex(home.join(".codex/config.toml")));
    }
    if let Some((path, key, value)) = json_config {
        return Some(Adapter::Json {
            path: home.join(path),
            key: key.into(),
            value,
        });
    }
    cli_adapter(command, executable, bridge, search_paths)
}
fn cli_adapter(
    command: &str,
    executable: &Path,
    bridge: &Path,
    search_paths: &[PathBuf],
) -> Option<Adapter> {
    let output = |args: &[String]| output(executable, args, search_paths);
    // Qoder/QoderCN and Antigravity expose native setup; custom versions use the same verified contract.
    let help = output(&["mcp".into(), "add".into(), "--help".into()]).ok()??;
    if !help.to_lowercase().contains("mcp")
        || !help.to_lowercase().contains("add")
        || !help.contains("name")
        || !(help.contains("command") || help.contains("COMMAND"))
    {
        return None;
    }
    let mut add = vec!["mcp".into(), "add".into()];
    let mut remove = vec!["mcp".into(), "remove".into()];
    let mut get = vec!["mcp".into(), "get".into()];
    if help.contains("--scope") && help.contains("user") {
        add.extend(["--scope".into(), "user".into()]);
        let remove_help = output(&["mcp".into(), "remove".into(), "--help".into()]).ok()??;
        if remove_help.contains("--scope") {
            remove.extend(["--scope".into(), "user".into()]);
        }
    }
    if help.contains("--type") {
        add.extend(["--type".into(), "stdio".into()]);
    } else if help.contains("--transport") {
        add.extend(["--transport".into(), "stdio".into()]);
    }
    add.push(SERVER.into());
    if command != "agy" {
        add.push("--".into());
    }
    add.extend([bridge.to_string_lossy().into_owned(), "mcp".into()]);
    remove.push(SERVER.into());
    get.push(SERVER.into());
    // Inspect the installed contract; unavailable clients are not guessed into readiness.
    let remove_help = output(&["mcp".into(), "remove".into(), "--help".into()]).ok()??;
    if !remove_help.to_lowercase().contains("name") {
        return None;
    }
    let get_help = output(&["mcp".into(), "get".into(), "--help".into()])
        .ok()
        .flatten();
    let get_all = get_help
        .as_ref()
        .is_none_or(|help| !help.to_lowercase().contains("name"));
    if get_all {
        output(&["mcp".into(), "list".into(), "--help".into()]).ok()??;
        get = vec!["mcp".into(), "list".into()];
    } else if get_help
        .as_ref()
        .is_some_and(|help| help.contains("--scope") && help.contains("user"))
    {
        get.extend(["--scope".into(), "user".into()]);
    }
    Some(Adapter::Cli {
        add,
        remove,
        get,
        get_all,
    })
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid configuration path"))?;
    std::fs::create_dir_all(parent)?;
    ensure!(
        !path
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink()),
        "Refusing to replace a configuration symlink"
    );
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    if let Ok(metadata) = std::fs::metadata(path) {
        file.as_file().set_permissions(metadata.permissions())?;
    }
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)
        .map_err(|_| anyhow::anyhow!("Could not replace configuration file"))?;
    Ok(())
}
pub fn save_preferences(path: &Path, preferences: &Preferences) -> Result<()> {
    atomic_write(path, &serde_json::to_vec(preferences)?)
}
fn read_text(path: &Path) -> Result<String> {
    ensure!(
        !path
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink()),
        "Configuration symlink was preserved"
    );
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.into()),
    }
}
fn vibe_servers(path: &Path) -> Result<Vec<Value>> {
    let parsed: toml::Value = toml::from_str(&read_text(path)?)
        .map_err(|_| anyhow::anyhow!("Invalid Vibe TOML; configuration was preserved"))?;
    match parsed.get("mcp_servers") {
        Some(servers) => serde_json::from_value(serde_json::to_value(servers)?)
            .map_err(|_| anyhow::anyhow!("Vibe MCP servers must be a list")),
        None => Ok(vec![]),
    }
}
/// Vibe's native environment layer supplies the stdio child env at runtime, never on disk.
pub fn vibe_environment(
    installed: &Installed,
    env: &mut std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>,
    directory: Option<&Path>,
) -> Result<()> {
    let Adapter::Vibe(path) = &installed.adapter else {
        return Ok(());
    };
    let mut servers = vibe_servers(path)?;
    if let Some(directory) = directory {
        let project = directory.join(".vibe/config.toml");
        if project.is_file() {
            let parsed: toml::Value = toml::from_str(&read_text(&project)?)?;
            if parsed.get("mcp_servers").is_some() {
                servers = vibe_servers(&project)?;
            }
        }
    }
    if let Some(existing) = env
        .get(std::ffi::OsStr::new("VIBE_MCP_SERVERS"))
        .cloned()
        .or_else(|| std::env::var_os("VIBE_MCP_SERVERS"))
    {
        servers = serde_json::from_str(
            existing
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid Vibe MCP environment"))?,
        )?;
    }
    ensure!(
        !servers.iter().any(|server| server["name"] == SERVER),
        "Communication server belongs to the user; preserved"
    );
    let mut dynamic = serde_json::Map::new();
    for name in [
        "WARP_AGENT_ENDPOINT",
        "WARP_AGENT_CAPABILITY",
        "WARP_TERMINAL_SESSION_UUID",
    ] {
        let value = env
            .get(std::ffi::OsStr::new(name))
            .and_then(|v| v.to_str())
            .ok_or_else(|| anyhow::anyhow!("Missing Warpai terminal binding"))?;
        dynamic.insert(name.into(), Value::String(value.into()));
    }
    servers.push(json!({"name":SERVER,"transport":"stdio","command":[installed.bridge],"args":["mcp"],"env":dynamic}));
    env.insert(
        "VIBE_MCP_SERVERS".into(),
        serde_json::to_string(&servers)?.into(),
    );
    Ok(())
}
fn codex_block(bridge: &Path) -> String {
    let command = toml::Value::String(bridge.to_string_lossy().into_owned());
    format!("{START}\n[mcp_servers.{SERVER}]\ncommand = {command}\nargs = [\"mcp\"]\nenv_vars = [\"WARP_AGENT_ENDPOINT\", \"WARP_AGENT_CAPABILITY\", \"WARP_TERMINAL_SESSION_UUID\"]\n{END}\n")
}
pub fn configure(installed: &Installed, enable: bool, owned: bool) -> Result<()> {
    match &installed.adapter {
        Adapter::Vibe(path) => {
            if enable {
                let servers = vibe_servers(path)?;
                ensure!(
                    !servers.iter().any(|server| server["name"] == SERVER),
                    "Communication server belongs to the user; preserved"
                );
            }
            Ok(())
        }
        Adapter::Toml { path, value } => {
            let text = read_text(path)?;
            let parsed: toml::Value = toml::from_str(&text)
                .map_err(|_| anyhow::anyhow!("Invalid agent TOML; configuration was preserved"))?;
            let entry = toml::Value::try_from(json!({"mcp_servers": {SERVER:value}}))?;
            let block = format!("{START}\n{}{END}\n", toml::to_string(&entry)?);
            let present = parsed
                .get("mcp_servers")
                .and_then(|v| v.get(SERVER))
                .is_some();
            if present {
                ensure!(
                    owned && text.contains(&block),
                    "Communication configuration was changed or belongs to the user; preserved"
                );
            }
            if enable && present || !enable && !present {
                return Ok(());
            }
            let next = if enable {
                format!("{text}\n{block}")
            } else {
                text.replacen(&block, "", 1)
            };
            let _: toml::Value = toml::from_str(&next)?;
            atomic_write(path, next.as_bytes())
        }
        Adapter::YamlList { path, value, .. } | Adapter::Cordis { path, value } => {
            let text = read_text(path)?;
            if !enable && text.trim().is_empty() {
                return Ok(());
            }
            let cordis = matches!(&installed.adapter, Adapter::Cordis { .. });
            let mut root: Value = serde_yaml::from_str(if text.trim().is_empty() {
                if cordis {
                    "[]"
                } else {
                    "{}"
                }
            } else {
                &text
            })
            .map_err(|_| anyhow::anyhow!("Invalid agent YAML; configuration was preserved"))?;
            if cordis {
                let rows = root
                    .as_array_mut()
                    .ok_or_else(|| anyhow::anyhow!("Cordis patch must be a list"))?;
                let expected = json!({"insert":[value]});
                let matches: Vec<_> = rows
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| {
                        row.get("insert")
                            .and_then(Value::as_array)
                            .is_some_and(|entries| {
                                entries.iter().any(|entry| {
                                    entry["id"] == SERVER || entry["config"]["serverName"] == SERVER
                                })
                            })
                    })
                    .map(|(index, _)| index)
                    .collect();
                ensure!(
                    matches.len() <= 1,
                    "Duplicate communication entries; preserved"
                );
                if let Some(&index) = matches.first() {
                    ensure!(
                        owned && rows[index] == expected,
                        "Communication configuration was changed or belongs to the user; preserved"
                    );
                    if !enable {
                        rows.remove(index);
                    }
                } else if enable {
                    rows.push(expected);
                }
            } else {
                let Adapter::YamlList { key, .. } = &installed.adapter else {
                    unreachable!()
                };
                let object = root
                    .as_object_mut()
                    .ok_or_else(|| anyhow::anyhow!("Agent YAML must be an object"))?;
                if !enable && !object.contains_key(key) {
                    return Ok(());
                }
                let rows = object
                    .entry(key.clone())
                    .or_insert_with(|| json!([]))
                    .as_array_mut()
                    .ok_or_else(|| anyhow::anyhow!("MCP configuration must be a list"))?;
                let matches: Vec<_> = rows
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| row["name"] == SERVER)
                    .map(|(index, _)| index)
                    .collect();
                ensure!(
                    matches.len() <= 1,
                    "Duplicate communication entries; preserved"
                );
                if let Some(&index) = matches.first() {
                    ensure!(
                        owned && rows[index] == *value,
                        "Communication configuration was changed or belongs to the user; preserved"
                    );
                    if !enable {
                        rows.remove(index);
                    }
                } else if enable {
                    rows.push(value.clone());
                }
            }
            atomic_write(path, serde_yaml::to_string(&root)?.as_bytes())
        }
        Adapter::Codex(path) => {
            let text = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
                Err(e) => return Err(e.into()),
            };
            let parsed: toml::Value = toml::from_str(&text)
                .map_err(|_| anyhow::anyhow!("Invalid Codex TOML; configuration was preserved"))?;
            let block = codex_block(&installed.bridge);
            let present = parsed
                .get("mcp_servers")
                .and_then(|v| v.get(SERVER))
                .is_some();
            if present {
                ensure!(
                    owned && text.contains(&block),
                    "Communication configuration was changed or belongs to the user; preserved"
                );
            }
            if enable && present || !enable && !present {
                return Ok(());
            }
            let next = if enable {
                format!("{text}\n{block}")
            } else {
                text.replacen(&block, "", 1)
            };
            toml::from_str::<toml::Value>(&next)
                .map_err(|_| anyhow::anyhow!("Invalid generated Codex configuration"))?;
            atomic_write(path, next.as_bytes())
        }
        Adapter::Json { path, key, value } | Adapter::Yaml { path, key, value } => {
            let text = match std::fs::read(path) {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => b"{}".to_vec(),
                Err(e) => return Err(e.into()),
            };
            let yaml = matches!(installed.adapter, Adapter::Yaml { .. });
            let mut root: Value = if yaml {
                serde_yaml::from_slice(&text).map_err(|_| {
                    anyhow::anyhow!("Invalid agent YAML; configuration was preserved")
                })?
            } else {
                serde_json::from_slice(&text).map_err(|_| {
                    anyhow::anyhow!("Invalid agent JSON; configuration was preserved")
                })?
            };
            let object = root
                .as_object_mut()
                .ok_or_else(|| anyhow::anyhow!("Agent configuration must be an object"))?;
            if !enable && !object.contains_key(key) {
                return Ok(());
            }
            let servers = object
                .entry(key.clone())
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| anyhow::anyhow!("MCP configuration must be an object"))?;
            if !enable && !servers.contains_key(SERVER) {
                return Ok(());
            }
            if let Some(existing) = servers.get(SERVER) {
                ensure!(
                    owned && existing == value,
                    "Communication configuration was changed or belongs to the user; preserved"
                );
            }
            if enable {
                servers.insert(SERVER.into(), value.clone());
            } else {
                servers.remove(SERVER);
            }
            let bytes = if yaml {
                serde_yaml::to_string(&root)?.into_bytes()
            } else {
                serde_json::to_vec_pretty(&root)?
            };
            atomic_write(path, &bytes)
        }
        Adapter::Cli {
            add,
            remove,
            get,
            get_all,
        } => {
            let (success, text) = run(&installed.executable, get, &installed.search_paths)?;
            let current = if *get_all {
                ensure!(success, "Could not inspect native MCP configuration");
                listed_entry(&text)
            } else if text.trim() == format!("Server \"{SERVER}\" not found in user settings.") {
                // QoderCN reports this successful lookup of an absent user-scope entry with status zero.
                None
            } else if success {
                Some(text)
            } else {
                let reason = text.to_lowercase();
                ensure!(
                    [
                        "not found",
                        "no mcp server",
                        "no server",
                        "does not exist",
                        "not configured"
                    ]
                    .iter()
                    .any(|missing| reason.contains(missing)),
                    "Could not inspect native MCP configuration"
                );
                None
            };
            if let Some(current) = &current {
                ensure!(
                    owned
                        && current.contains(&*installed.bridge.to_string_lossy())
                        && current.contains("mcp"),
                    "Communication server already exists or was modified; preserved"
                );
            }
            if enable && current.is_some() || !enable && current.is_none() {
                return Ok(());
            }
            ensure!(
                output(
                    &installed.executable,
                    if enable { add } else { remove },
                    &installed.search_paths
                )?
                .is_some(),
                "Agent rejected communication configuration"
            );
            Ok(())
        }
    }
}
fn listed_entry(text: &str) -> Option<String> {
    let mut lines = text.lines().skip_while(|line| {
        !line
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
            .any(|word| word == SERVER)
    });
    let first = lines.next()?;
    let mut entry = first.to_owned();
    // Native list output can put command/args on indented lines, rather than the name row.
    for line in lines {
        if line.trim().is_empty() || !line.starts_with(char::is_whitespace) {
            break;
        }
        entry.push('\n');
        entry.push_str(line);
    }
    Some(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_remote_profiles_are_read_only_metadata_and_preserve_local_selection() {
        let profile = serde_json::json!({
            "alias": "old-host", "coordinator": Uuid::new_v4(), "device": Uuid::new_v4(),
            "generation": 1, "spaces": [Uuid::new_v4()],
        });
        let preferences: Preferences = serde_json::from_value(serde_json::json!({
            "enabled": true, "selected": {}, "remote_profiles": [profile.clone()],
        })).unwrap();
        assert!(preferences.enabled);
        assert_eq!(preferences.legacy_remote_profiles.len(), 1);
        assert!(preferences.legacy_remote_profiles[0].cleanup_pending);
        let encoded = serde_json::to_value(&preferences).unwrap();
        assert!(encoded.get("remote_profiles").is_none());
        assert_eq!(encoded["legacy_remote_profiles"][0]["alias"], "old-host");
        assert_eq!(encoded["selected"], serde_json::json!({}));
        let roundtrip: Preferences = serde_json::from_value(encoded).unwrap();
        assert!(roundtrip.legacy_remote_profiles[0].cleanup_pending);
        let old: Preferences = serde_json::from_value(serde_json::json!({
            "enabled": false, "selected": {},
        })).unwrap();
        assert!(old.legacy_remote_profiles.is_empty());
        for field in ["credential", "invitation", "capability", "private_key"] {
            let mut injected = profile.clone();
            injected[field] = serde_json::json!("synthetic");
            assert!(serde_json::from_value::<LegacyRemoteProfile>(injected).is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn qodercn_discovery_and_native_scope_contract_use_the_shell_path() {
        use std::os::unix::fs::PermissionsExt;
        let home = tempfile::tempdir().unwrap();
        let entry = home.path().join(".qoder-cn/entry");
        let interpreters = home.path().join("interpreters");
        std::fs::create_dir_all(&entry).unwrap();
        std::fs::create_dir_all(&interpreters).unwrap();
        let cli = entry.join("qodercn");
        std::fs::write(&cli, "#!/usr/bin/env warp-test-interpreter\n").unwrap();
        let interpreter = interpreters.join("warp-test-interpreter");
        std::fs::write(&interpreter, "#!/bin/sh\necho 'Usage: mcp add <name> <commandOrUrl> [args...] --scope user --transport stdio; mcp remove <name> --scope; mcp get <name> --scope'\n").unwrap();
        for path in [&cli, &interpreter] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let paths = vec![interpreters, entry.clone()];
        assert_eq!(
            executable("qodercn", &paths, home.path()),
            Some(cli.clone())
        );
        assert!(
            cli_adapter("qodercn", &cli, Path::new("/bridge"), &[]).is_none(),
            "An absent interpreter must report unavailable"
        );
        let configured =
            adapter(home.path(), "qodercn", &cli, Path::new("/bridge"), &paths).unwrap();
        let Adapter::Cli {
            add,
            get,
            remove,
            get_all,
        } = configured
        else {
            panic!("QoderCN must use its native MCP command")
        };
        assert_eq!(
            add,
            [
                "mcp",
                "add",
                "--scope",
                "user",
                "--transport",
                "stdio",
                SERVER,
                "--",
                "/bridge",
                "mcp"
            ]
        );
        assert_eq!(get, ["mcp", "get", SERVER, "--scope", "user"]);
        assert_eq!(remove, ["mcp", "remove", "--scope", "user", SERVER]);
        assert!(!get_all);
        assert_eq!(
            executable("qodercn", &paths[..1], home.path()),
            Some(cli),
            "Vendor entry fallback must work without a login-shell PATH"
        );
        let qoder = entry.join("qoder");
        std::fs::write(&qoder, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&qoder, std::fs::Permissions::from_mode(0o700)).unwrap();
        let available = discover(
            vec![
                ("qoder".into(), "qoder".into()),
                ("qodercn".into(), "qodercn".into()),
            ],
            Path::new("/bridge"),
            paths,
        );
        assert_eq!(available.len(), 2);
        assert!(available[0].installed.is_none());
        let selected = available[1].installed.clone().unwrap();
        assert_eq!(selected.program, "qodercn");
        let preferences = Preferences {
            enabled: true,
            selected: BTreeMap::from([("qodercn".into(), selected)]),
            legacy_remote_profiles: vec![],
        };
        assert!(preferences.programs().contains("qodercn"));
        assert!(!preferences.programs().contains("qoder"));
    }
    #[cfg(unix)]
    #[test]
    fn every_managed_alias_has_native_setup_and_vibe_keeps_bindings_in_memory() {
        use std::os::unix::fs::PermissionsExt;
        let home = tempfile::tempdir().unwrap();
        let cli = home.path().join("native-agent");
        std::fs::write(&cli, "#!/bin/sh\necho 'Usage: mcp add <name> <command>; mcp remove <name>; mcp get <name>'\n").unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        let aliases: Vec<_> = include_str!("../terminal/cli_agent.rs")
            .lines()
            .filter(|line| line.contains("=> &[\""))
            .flat_map(|line| line.split('"').skip(1).step_by(2))
            .collect();
        assert!(aliases.contains(&"dsh-tui") && aliases.contains(&"traecli"));
        for command in aliases {
            assert!(
                adapter(home.path(), command, &cli, Path::new("/bridge"), &[]).is_some(),
                "Missing native setup for {command}"
            );
        }
        let Adapter::Json { value, .. } =
            adapter(home.path(), "cursor-agent", &cli, Path::new("/bridge"), &[]).unwrap()
        else {
            panic!("Cursor must use native JSON configuration")
        };
        for name in ["WARP_AGENT_ENDPOINT", "WARP_AGENT_CAPABILITY", "WARP_TERMINAL_SESSION_UUID"] {
            assert_eq!(value["env"][name], format!("${{env:{name}}}"));
        }
        let path = home.path().join("vibe.toml");
        let original = "# user settings\n[[mcp_servers]]\nname = 'user-server'\ntransport = 'stdio'\ncommand = 'user-command'\n";
        std::fs::write(&path, original).unwrap();
        let installed = Installed {
            active: true,
            program: "vibe".into(),
            executable: cli,
            adapter: Adapter::Vibe(path.clone()),
            bridge: PathBuf::from("/bridge"),
            search_paths: vec![],
            launch_options: Default::default(),
        };
        configure(&installed, true, false).unwrap();
        let mut env = std::collections::HashMap::from([
            ("WARP_AGENT_ENDPOINT".into(), "test-endpoint".into()),
            ("WARP_AGENT_CAPABILITY".into(), "test-binding".into()),
            ("WARP_TERMINAL_SESSION_UUID".into(), "test-terminal".into()),
        ]);
        vibe_environment(&installed, &mut env, None).unwrap();
        let servers: Value = serde_json::from_str(
            env[std::ffi::OsStr::new("VIBE_MCP_SERVERS")]
                .to_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(servers[0]["name"], "user-server");
        assert_eq!(servers[1]["env"]["WARP_AGENT_CAPABILITY"], "test-binding");
        assert!(vibe_environment(&installed, &mut env, None).is_err());
        configure(&installed, false, true).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), original);
    }
    #[test]
    fn managed_configuration_is_reversible_and_preserves_user_settings() {
        let directory = tempfile::tempdir().unwrap();
        for adapter in [
            Adapter::Codex(directory.path().join("config.toml")),
            Adapter::Toml {
                path: directory.path().join("grok.toml"),
                value: json!({"command":"/bridge", "args":["mcp"]}),
            },
            Adapter::YamlList {
                path: directory.path().join("trae.yaml"),
                key: "mcp_servers".into(),
                value: json!({"name":SERVER,"command":"/bridge","args":["mcp"],"type":"stdio"}),
            },
            Adapter::Cordis {
                path: directory.path().join("cordis.patch.yml"),
                value: json!({"id":SERVER,"name":"@deepseek-ai/dsh-mcp-client","config":{"serverName":SERVER,"transport":"stdio","command":"/bridge","args":["mcp"]}}),
            },
            Adapter::Yaml {
                path: directory.path().join("hermes.yaml"),
                key: "mcp_servers".into(),
                value: json!({"command":"/bridge", "args":["mcp"]}),
            },
            Adapter::Json {
                path: directory.path().join("config.json"),
                key: "mcpServers".into(),
                value: json!({"command":"/bridge", "args":["mcp"]}),
            },
        ] {
            let path = match &adapter {
                Adapter::Codex(p) => p,
                Adapter::Json { path, .. }
                | Adapter::Yaml { path, .. }
                | Adapter::Toml { path, .. }
                | Adapter::YamlList { path, .. }
                | Adapter::Cordis { path, .. } => path,
                _ => unreachable!(),
            };
            let original = match adapter {
                Adapter::Codex(_) | Adapter::Toml { .. } => "# user comment\nmodel = \"existing\"\n",
                Adapter::Cordis { .. } => "- insert:\n    - id: user-server\n      name: existing\n",
                _ => "{\"theme\":\"existing\",\"mcpServers\":{\"user-server\":{\"command\":\"user\"}}}",
            };
            std::fs::write(path, original).unwrap();
            let installed = Installed {
                active: true,
                program: "test".into(),
                executable: PathBuf::from("test"),
                adapter: adapter.clone(),
                bridge: PathBuf::from("/bridge"),
                search_paths: vec![],
                launch_options: Default::default(),
            };
            configure(&installed, true, false).unwrap();
            configure(&installed, true, true).unwrap();
            assert!(configure(&installed, true, false).is_err());
            let original = std::fs::read_to_string(path).unwrap();
            let changed = original.replace("/bridge", "/user-bridge");
            std::fs::write(path, &changed).unwrap();
            assert!(configure(&installed, false, true).is_err());
            assert_eq!(std::fs::read_to_string(path).unwrap(), changed);
            std::fs::write(path, original).unwrap();
            let mut preferences = Preferences::default();
            preferences
                .selected
                .insert("test".into(), installed.clone());
            assert!(preferences.programs().is_empty());
            preferences.enabled = true;
            assert!(preferences.programs().contains("test"));
            preferences.selected.get_mut("test").unwrap().active = false;
            let restored: Preferences =
                serde_json::from_slice(&serde_json::to_vec(&preferences).unwrap()).unwrap();
            assert!(
                restored.programs().is_empty(),
                "Pending cleanup must never restore authorization"
            );
            configure(&installed, false, true).unwrap();
            configure(&installed, false, true).unwrap();
            let remaining = std::fs::read_to_string(path).unwrap();
            assert!(remaining.contains("existing"));
            assert!(!remaining.contains(SERVER));
            if matches!(adapter, Adapter::Codex(_) | Adapter::Toml { .. }) {
                assert!(remaining.contains("# user comment"));
                assert!(!remaining.contains("env_vars"));
            } else {
                assert!(remaining.contains("user-server"));
            }
        }
    }
    #[cfg(unix)]
    #[test]
    fn native_cli_setup_checks_ownership_and_is_reversible() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let cli = directory.path().join("native-agent");
        std::fs::write(
            &cli,
            r#"#!/bin/sh
entry="$(dirname "$0")/entry"
if [ "$3" = "--help" ]; then
    echo "Usage: mcp $2 <name> <command> [args...]"
    exit 0
fi
case "$2" in
    get) if [ -f "$entry" ]; then cat "$entry"; else echo 'server not found' >&2; exit 1; fi ;;
    add) printf '%s mcp' "$5" > "$entry" ;;
    remove) rm -f "$entry" ;;
    *) exit 2 ;;
esac
"#,
        )
        .unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            discover(
                vec![("custom".into(), "native-agent".into())],
                Path::new("/bridge"),
                vec![directory.path().to_owned()]
            )
            .len(),
            1
        );
        let adapter = adapter(
            directory.path(),
            "native-agent",
            &cli,
            Path::new("/bridge"),
            &[],
        )
        .unwrap();
        let installed = Installed {
            active: true,
            program: "custom".into(),
            executable: cli,
            adapter,
            bridge: PathBuf::from("/bridge"),
            search_paths: vec![],
            launch_options: Default::default(),
        };
        configure(&installed, true, false).unwrap();
        configure(&installed, true, true).unwrap();
        assert!(configure(&installed, true, false).is_err());
        std::fs::write(directory.path().join("entry"), "/user-command mcp").unwrap();
        assert!(configure(&installed, false, true).is_err());
        std::fs::write(directory.path().join("entry"), "/bridge mcp").unwrap();
        configure(&installed, false, true).unwrap();
        assert!(!directory.path().join("entry").exists());
        let source = std::fs::read_to_string(&installed.executable).unwrap();
        std::fs::write(
            &installed.executable,
            source.replace(
                "echo 'server not found' >&2; exit 1",
                "echo 'Server \"warp-lite-communication\" not found in user settings.'; exit 0",
            ),
        ).unwrap();
        configure(&installed, false, false).unwrap();
        configure(&installed, true, false).unwrap();
        assert!(configure(&installed, true, false).is_err());
        configure(&installed, false, true).unwrap();
        assert!(!directory.path().join("entry").exists());
        let foreign = "Server \"warp-lite-communication\" not found in user settings.\n/user-command mcp";
        std::fs::write(directory.path().join("entry"), foreign).unwrap();
        assert!(configure(&installed, true, false).is_err());
        assert_eq!(std::fs::read_to_string(directory.path().join("entry")).unwrap(), foreign);
    }
}
