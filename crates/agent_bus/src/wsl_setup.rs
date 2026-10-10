//! Guest-owned MCP installation through existing native vendor adapters.
use crate::setup::{self, Available, Preferences};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRequest {
    pub shell_pid: u32,
    pub action: NativeAction,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeAction {
    Observe {
        draft: bool,
        blocked: bool,
        input_epoch: u64,
        submitted: bool,
        cancelled: bool,
        native_ready: Option<bool>,
    },
    Claim {
        wake: crate::transport::Wake,
    },
    Validate {
        wake: crate::transport::Wake,
    },
    Finish {
        wake: crate::transport::Wake,
        submitted: bool,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeState {
    pub run: String,
    pub program: String,
    pub wake: Option<crate::transport::Wake>,
    pub valid: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "action",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Action {
    Enable(bool),
    Select(String),
    Rescan,
    RemoveAll,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub action: Action,
    pub commands: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub account: String,
    pub preferences: Preferences,
    pub available: Vec<Available>,
    pub status: String,
}

#[cfg(all(feature = "wsl_companion", target_os = "linux"))]
pub fn uninstall() -> Result<()> {
    use sha2::Digest;
    let executable = std::env::current_exe()?;
    let expected = std::path::PathBuf::from(crate::installation::wsl_companion_path(
        dirs::home_dir()
            .and_then(|home| home.to_str().map(str::to_owned))
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Linux home unavailable"))?,
    )?);
    ensure!(executable == expected, "Unexpected WSL installation");
    for (count, entry) in std::fs::read_dir("/proc")?.enumerate() {
        ensure!(count < 16384, "Process limit exceeded");
        let entry = entry?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == std::process::id() {
            continue;
        }
        if let Ok(path) = std::fs::read_link(entry.path().join("exe")) {
            ensure!(
                path.to_str()
                    .map(|path| path.trim_end_matches(" (deleted)"))
                    != expected.to_str(),
                "WSL Companion is active"
            );
        }
    }
    let directory = executable
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Missing installation"))?;
    for (count, entry) in std::fs::read_dir(directory)?.enumerate() {
        ensure!(count < 256, "Installation entry limit exceeded");
        let entry = entry?;
        let name = entry.file_name();
        let Some(hash) = name
            .to_str()
            .and_then(|name| name.strip_prefix("companion-runtime-"))
        else {
            continue;
        };
        if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            continue;
        }
        ensure!(entry.file_type()?.is_dir(), "Runtime must be a directory");
        let sums_path = entry.path().join("SHA256SUMS");
        ensure!(
            !sums_path.symlink_metadata()?.file_type().is_symlink(),
            "Runtime metadata cannot be a link"
        );
        let mut sums = Vec::new();
        std::fs::File::open(sums_path)?
            .take(512 * 1024 + 1)
            .read_to_end(&mut sums)?;
        ensure!(
            sums.len() <= 512 * 1024 && format!("{:x}", sha2::Sha256::digest(&sums)) == hash,
            "Unrecognized runtime directory"
        );
        std::fs::remove_dir_all(entry.path())?;
    }
    let manifest = directory.join("companion-manifest.json");
    match manifest.symlink_metadata() {
        Ok(metadata) => {
            ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "Invalid manifest"
            );
            std::fs::remove_file(manifest)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
    }
    std::fs::remove_file(executable)?;
    Ok(())
}

#[cfg(all(feature = "wsl_companion", target_os = "linux"))]
pub fn run() -> Result<()> {
    let mut bytes = Vec::new();
    std::io::stdin().take(65537).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 65536, "Setup request too large");
    let request: Request = serde_json::from_slice(&bytes)?;
    ensure!(
        request.commands.len() <= 128
            && request.commands.iter().all(|(program, command)| {
                [program, command].into_iter().all(|value| {
                    !value.is_empty()
                        && value.len() <= 64
                        && value
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
                })
            }),
        "Invalid Agent command"
    );
    let path = crate::companion::guest_preferences_path()?;
    let _lock = lock_preferences(&path)?;
    let mut preferences: Preferences = match std::fs::File::open(&path) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(512 * 1024 + 1).read_to_end(&mut bytes)?;
            ensure!(bytes.len() <= 512 * 1024, "Guest preferences too large");
            serde_json::from_slice(&bytes)?
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Default::default(),
        Err(error) => return Err(error.into()),
    };
    let bridge = std::env::current_exe()?;
    let mut available = setup::discover(
        request.commands,
        &bridge,
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect(),
    );
    let failed = apply_action(&request.action, &mut preferences, &mut available, &path)?;
    for row in &mut available {
        if row.program == "codex"
            && !preferences
                .selected
                .get(&row.command)
                .is_some_and(|entry| entry.active)
        {
            row.status = "Available for guest MCP setup".into();
        }
    }
    let account = std::process::Command::new("id").arg("-un").output()?;
    ensure!(
        account.status.success() && account.stdout.len() <= 256,
        "Linux account unavailable"
    );
    let response = Response {
        account: String::from_utf8(account.stdout)?.trim().into(),
        preferences,
        available,
        status: if matches!(request.action, Action::Rescan) {
            "Agent scan complete for this WSL account."
        } else if failed {
            "Some MCP settings could not be updated; other integrations were preserved."
        } else {
            "WSL MCP settings saved."
        }
        .into(),
    };
    let bytes = serde_json::to_vec(&response)?;
    ensure!(bytes.len() <= 512 * 1024, "Setup response too large");
    std::io::stdout().write_all(&bytes)?;
    Ok(())
}

#[cfg(all(feature = "wsl_companion", target_os = "linux"))]
fn lock_preferences(path: &std::path::Path) -> Result<std::fs::File> {
    use std::os::{fd::AsRawFd, unix::fs::OpenOptionsExt};
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid guest preferences path"))?;
    std::fs::create_dir_all(parent)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(parent.join("mcp-settings.lock"))?;
    // Atomic saves replace the preferences inode, so lock a separate stable file.
    ensure!(
        unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
        "Another WSL setup operation is active; rescan after it finishes"
    );
    Ok(file)
}

#[cfg(all(feature = "wsl_companion", target_os = "linux"))]
fn apply_action(
    action: &Action,
    preferences: &mut Preferences,
    available: &mut [Available],
    path: &std::path::Path,
) -> Result<bool> {
    if matches!(action, Action::Rescan) {
        return Ok(false);
    }
    let command = match action {
        Action::Select(command) => Some(command.as_str()),
        _ => None,
    };
    let select_enable = command.is_some_and(|name| {
        !preferences
            .selected
            .get(name)
            .is_some_and(|entry| entry.active)
    });
    if matches!(action, Action::Select(_)) {
        preferences.enabled = true;
    }
    if let Action::Enable(enabled) = action {
        preferences.enabled = *enabled;
    }
    if matches!(action, Action::RemoveAll) {
        preferences.enabled = false;
        for row in available.iter() {
            if let Some(installed) = &row.installed {
                preferences
                    .selected
                    .entry(row.command.clone())
                    .or_insert_with(|| installed.clone());
            }
        }
        // Revoke participation even when removing a vendor configuration fails.
        for installed in preferences.selected.values_mut() {
            installed.active = false;
        }
        setup::save_preferences(path, preferences)?;
    }
    let mut failed = false;
    for (name, installed) in &mut preferences.selected {
        if !preferences.enabled || command == Some(name.as_str()) {
            match setup::configure_guest(installed, false, true) {
                Ok(()) => installed.active = false,
                Err(_) => {
                    installed.active = false;
                    failed = true;
                }
            }
        }
    }
    if preferences.enabled {
        for row in available.iter_mut() {
            let Some(installed) = &row.installed else {
                continue;
            };
            let selected = preferences.selected.get(&row.command);
            let enable = if command == Some(row.command.as_str()) {
                select_enable
            } else {
                selected.is_some_and(|entry| entry.active)
            };
            if !enable {
                continue;
            }
            let mut entry = installed.clone();
            match setup::configure_guest(&entry, true, selected.is_some()) {
                Ok(()) => {
                    entry.active = true;
                    row.status = "WSL MCP enabled".into();
                }
                Err(_) => {
                    entry.active = false;
                    row.status = "WSL MCP setup failed; existing configuration preserved".into();
                    failed = true;
                }
            }
            preferences.selected.insert(row.command.clone(), entry);
        }
    }
    setup::save_preferences(path, preferences)?;
    Ok(failed)
}

#[cfg(all(test, feature = "wsl_companion", target_os = "linux"))]
mod tests {
    use super::*;
    use crate::setup::{Adapter, Installed, SERVER};

    #[test]
    fn account_lock_rejects_concurrent_setup_and_symlinks() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mcp-settings.json");
        let lock = lock_preferences(&path).unwrap();
        assert!(lock_preferences(&path).is_err());
        let other = directory.path().join("other/mcp-settings.json");
        assert!(lock_preferences(&other).is_ok());
        drop(lock);
        assert!(lock_preferences(&path).is_ok());
        std::fs::remove_file(directory.path().join("mcp-settings.lock")).unwrap();
        std::os::unix::fs::symlink(&other, directory.path().join("mcp-settings.lock")).unwrap();
        assert!(lock_preferences(&path).is_err());
    }

    #[test]
    fn maintenance_preserves_other_accounts_and_rescan_is_read_only() {
        let directory = tempfile::tempdir().unwrap();
        let mut accounts = Vec::new();
        for account in [
            "selected",
            "other-user",
            "other-distro",
            "windows-local",
            "ssh",
        ] {
            let home = directory.path().join(account);
            std::fs::create_dir(&home).unwrap();
            let config = home.join("config.toml");
            std::fs::write(
                &config,
                "[mcp_servers.user_tool]\ncommand = \"user-tool\"\n",
            )
            .unwrap();
            let installed = Installed {
                active: true,
                program: "codex".into(),
                executable: "/bin/codex".into(),
                adapter: Adapter::Codex(config.clone()),
                bridge: home.join("warpai-wsl-companion"),
                search_paths: vec![],
                launch_options: Default::default(),
            };
            setup::configure_guest(&installed, true, false).unwrap();
            accounts.push((config.clone(), installed, std::fs::read(&config).unwrap()));
        }
        let path = directory.path().join("selected/mcp-settings.json");
        // Discovery also cleans up owned entries absent from the saved selection.
        let mut preferences = Preferences {
            enabled: true,
            ..Default::default()
        };
        setup::save_preferences(&path, &preferences).unwrap();
        let saved = std::fs::read(&path).unwrap();
        let mut available = vec![Available {
            command: "codex".into(),
            program: "codex".into(),
            installed: Some(accounts[0].1.clone()),
            status: "Available".into(),
        }];
        assert!(!apply_action(&Action::Rescan, &mut preferences, &mut available, &path).unwrap());
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        assert_eq!(std::fs::read(&accounts[0].0).unwrap(), accounts[0].2);
        assert!(preferences.enabled);
        assert!(
            !apply_action(&Action::RemoveAll, &mut preferences, &mut available, &path).unwrap()
        );
        assert!(!preferences.enabled);
        assert!(!preferences.selected["codex"].active);
        let config: toml::Value =
            toml::from_str(&std::fs::read_to_string(&accounts[0].0).unwrap()).unwrap();
        assert!(config["mcp_servers"].get(SERVER).is_none());
        assert_eq!(
            config["mcp_servers"]["user_tool"]["command"].as_str(),
            Some("user-tool")
        );
        for (config, _, original) in &accounts[1..] {
            assert_eq!(std::fs::read(config).unwrap(), *original);
        }
        assert!(!apply_action(
            &Action::Select("codex".into()),
            &mut preferences,
            &mut available,
            &path
        )
        .unwrap());
        assert!(preferences.enabled && preferences.selected["codex"].active);
    }
}
