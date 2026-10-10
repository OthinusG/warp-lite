//! Native guest MCP attaches to a Linux Agent lifetime, rather than a panel or managed PTY.
use super::*;
use std::{
    collections::{HashMap, HashSet},
    io::Read,
    os::unix::fs::MetadataExt,
    sync::Mutex,
};

struct Process {
    parent: u32,
    group: u32,
    tty: i64,
    foreground: i64,
    started: u64,
}

pub(super) fn distribution_namespace(distribution: &str) -> std::io::Result<String> {
    use sha2::Digest;
    if distribution.is_empty()
        || distribution.len() > 256
        || distribution.chars().any(char::is_control)
    {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    // Imported guests can share a home mount; their service state must still stay separate.
    Ok(format!(
        "{:x}",
        sha2::Sha256::digest(distribution.as_bytes())
    ))
}

fn process(pid: u32) -> std::io::Result<Process> {
    if pid <= 1 {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    let directory = PathBuf::from(format!("/proc/{pid}"));
    // Kernel process metadata proves account ownership without reading command arguments.
    if directory.metadata()?.uid() != unsafe { libc::geteuid() } {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    let stat = std::fs::read_to_string(directory.join("stat"))?;
    parse_stat(&stat)
}

fn parse_stat(stat: &str) -> std::io::Result<Process> {
    let invalid = || std::io::Error::from(std::io::ErrorKind::InvalidData);
    if stat.len() > 16384 {
        return Err(invalid());
    }
    let fields: Vec<_> = stat
        .rsplit_once(')')
        .ok_or_else(invalid)?
        .1
        .split_whitespace()
        .collect();
    if fields.len() < 20 || matches!(fields[0], "Z" | "X" | "x") {
        return Err(invalid());
    }
    Ok(Process {
        parent: fields[1].parse().map_err(|_| invalid())?,
        group: fields[2].parse().map_err(|_| invalid())?,
        tty: fields[4].parse().map_err(|_| invalid())?,
        foreground: fields[5].parse().map_err(|_| invalid())?,
        started: fields[19].parse().map_err(|_| invalid())?,
    })
}

fn foreground_owner(peer: u32) -> std::io::Result<u32> {
    let parent = process(peer)?;
    if parent.tty == 0 || parent.foreground != i64::from(parent.group) {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    // npm and shell launchers share the interactive command's group, unlike a detached daemon.
    let owner = ancestor(peer, parent.group)?;
    if owner.tty != parent.tty || owner.foreground != i64::from(parent.group) {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    Ok(parent.group)
}

fn ancestor(mut peer: u32, agent: u32) -> std::io::Result<Process> {
    for _ in 0..32 {
        let current = process(peer)?;
        if peer == agent {
            return Ok(current);
        }
        if current.parent == peer {
            break;
        }
        peer = current.parent;
    }
    Err(std::io::ErrorKind::PermissionDenied.into())
}

struct Guest {
    started: u64,
    shell: u32,
    shell_started: u64,
    input_epoch: Option<u64>,
    native_ready: Option<bool>,
    root: PathBuf,
    program: String,
    run: String,
    binding: tasks::RunBinding,
}

pub(super) struct Guests {
    runs: Mutex<HashMap<u32, Guest>>,
    preferences_path: PathBuf,
}

impl Guests {
    pub fn new(preferences_path: PathBuf) -> Self {
        Self {
            runs: Mutex::new(HashMap::new()),
            preferences_path,
        }
    }

    fn programs(&self) -> Option<HashSet<String>> {
        let mut bytes = Vec::new();
        std::fs::File::open(&self.preferences_path)
            .ok()?
            .take(512 * 1024 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() > 512 * 1024 {
            return None;
        }
        serde_json::from_slice::<crate::setup::Preferences>(&bytes)
            .ok()
            .map(|preferences| preferences.programs())
    }

    pub fn reap_and_active(&self) -> bool {
        let Ok(mut guests) = self.runs.lock() else {
            return true;
        };
        let programs = self.programs().unwrap_or_default();
        guests.retain(|pid, guest| {
            programs.contains(&guest.program)
                && process(*pid).is_ok_and(|process| process.started == guest.started)
        });
        !guests.is_empty()
    }

    pub fn native(
        &self,
        root: &Path,
        request: crate::wsl_setup::NativeRequest,
    ) -> Result<Option<crate::wsl_setup::NativeState>, ManagedErrorCode> {
        use crate::wsl_setup::{NativeAction, NativeState};
        self.reap_and_active();
        let root = PathBuf::from(
            crate::project_root(root).map_err(|_| ManagedErrorCode::ManagedInvalidInput)?,
        );
        let shell = process(request.shell_pid).map_err(path_error)?;
        let mut runs = self
            .runs
            .lock()
            .map_err(|_| ManagedErrorCode::ManagedUnavailable)?;
        let Some((pid, guest)) = runs.iter_mut().find(|(_, guest)| {
            guest.root == root
                && guest.shell == request.shell_pid
                && guest.shell_started == shell.started
        }) else {
            return Ok(None);
        };
        let owner = process(*pid).map_err(path_error)?;
        if owner.started != guest.started || owner.foreground != i64::from(*pid) || owner.tty == 0 {
            return Ok(None);
        }
        let broker = &guest.binding.broker;
        let terminal = &guest.binding.terminal;
        let mut state = NativeState {
            run: guest.run.clone(),
            program: guest.program.clone(),
            wake: None,
            valid: false,
        };
        match request.action {
            NativeAction::Observe {
                draft,
                blocked,
                input_epoch,
                submitted,
                cancelled,
                native_ready,
            } => {
                if guest.input_epoch.is_some_and(|epoch| epoch != input_epoch) {
                    broker.native_input(terminal, submitted, cancelled);
                }
                guest.input_epoch = Some(input_epoch);
                broker.input_guard(terminal, draft, blocked);
                if native_ready.is_some() && native_ready != guest.native_ready {
                    broker.readiness(terminal, native_ready == Some(true));
                    guest.native_ready = native_ready;
                }
                state.wake = broker
                    .wakeups()
                    .into_iter()
                    .find(|wake| wake.terminal == *terminal && wake.run == guest.run);
            }
            NativeAction::Claim { wake }
            | NativeAction::Validate { wake }
            | NativeAction::Finish { wake, .. }
                if wake.terminal != *terminal || wake.run != guest.run =>
            {
                return Err(ManagedErrorCode::ManagedStaleAttachment);
            }
            NativeAction::Claim { wake } => {
                state.valid = broker.claim_wake(&wake);
                state.wake = Some(wake);
            }
            NativeAction::Validate { wake } => {
                state.valid = broker.wake_valid(&wake);
                state.wake = Some(wake);
            }
            NativeAction::Finish { wake, submitted } => {
                broker.finish_wake(&wake, submitted);
            }
        }
        Ok(Some(state))
    }

    pub fn bind(
        &self,
        projects: &tasks::Projects,
        fence: &ManagedFence,
        root: &Path,
        peer: Option<u32>,
        pid: u32,
        program: &str,
    ) -> Result<GuestMcpBound, ManagedErrorCode> {
        if program.is_empty()
            || program.len() > 64
            || program == crate::OPERATOR_PROGRAM
            || !program
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
        {
            return Err(ManagedErrorCode::ManagedInvalidInput);
        }
        if !self
            .programs()
            .is_some_and(|programs| programs.contains(program))
        {
            return Err(ManagedErrorCode::ManagedPermissionDenied);
        }
        let owner = ancestor(peer.ok_or(ManagedErrorCode::ManagedPermissionDenied)?, pid)
            .map_err(path_error)?;
        if owner.group != pid || owner.tty == 0 || owner.foreground != i64::from(pid) {
            return Err(ManagedErrorCode::ManagedPermissionDenied);
        }
        let shell = process(owner.parent).map_err(path_error)?;
        let root = PathBuf::from(
            crate::project_root(root).map_err(|_| ManagedErrorCode::ManagedInvalidInput)?,
        );
        let mut guests = self
            .runs
            .lock()
            .map_err(|_| ManagedErrorCode::ManagedUnavailable)?;
        if guests
            .get(&pid)
            .is_some_and(|guest| guest.started != owner.started)
        {
            guests.remove(&pid);
        }
        if let Some(guest) = guests.get(&pid) {
            if guest.root != root || guest.program != program {
                return Err(ManagedErrorCode::ManagedConflict);
            }
        } else {
            let terminal = Uuid::new_v4().to_string();
            let run = Uuid::new_v4().to_string();
            let binding = projects.bind_run(fence, &root, &terminal, &run, program)?;
            guests.insert(
                pid,
                Guest {
                    started: owner.started,
                    shell: owner.parent,
                    shell_started: shell.started,
                    input_epoch: None,
                    native_ready: None,
                    root,
                    program: program.into(),
                    run,
                    binding,
                },
            );
        }
        let guest = guests
            .get(&pid)
            .ok_or(ManagedErrorCode::ManagedUnavailable)?;
        let value = |name: &str| {
            guest
                .binding
                .environment
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
                .ok_or(ManagedErrorCode::ManagedUnavailable)
        };
        Ok(GuestMcpBound {
            fence: Some(fence.clone()),
            endpoint: value(crate::transport::ENDPOINT)?,
            terminal: value(crate::transport::TERMINAL)?,
            capability: value(crate::transport::CAPABILITY)?,
            run: guest.run.clone(),
        })
    }
}

pub(super) async fn serve_mcp(program: String) -> anyhow::Result<()> {
    use rmcp::ServiceExt;
    let directory = std::env::current_dir()?;
    let directory = directory
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Guest directory must be UTF-8"))?
        .to_owned();
    let agent = foreground_owner(u32::try_from(unsafe { libc::getppid() })?)?;
    let mut client = crate::ssh_remote::HostClient::connect_local(&directory)
        .await
        .map_err(|_| anyhow::anyhow!("WSL service unavailable"))?;
    let binding = client
        .guest_mcp_bind(agent, program)
        .await
        .map_err(|_| anyhow::anyhow!("Native Agent binding unavailable"))?;
    let bridge = crate::mcp::Bridge::from_guest_binding(binding, directory);
    // The service holds the process-owned run even when this MCP connection ends.
    drop(client);
    bridge
        .serve(crate::mcp::legacy_transport(rmcp::transport::stdio()).await?)
        .await?
        .waiting()
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distributions_remain_distinct_even_with_a_shared_home() {
        let first = distribution_namespace("Custom Linux 多语言").unwrap();
        assert_eq!(first.len(), 64);
        assert_ne!(first, distribution_namespace("Other Linux").unwrap());
        assert!(distribution_namespace("").is_err());
        assert!(distribution_namespace("bad\nname").is_err());
    }
    #[test]
    fn linux_process_identity_handles_parentheses_and_rejects_zombies() {
        let stat = |state: &str| {
            format!("42 (agent (worker)) {state} 7 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1234 0")
        };
        let owner = parse_stat(&stat("S")).unwrap();
        assert_eq!(owner.parent, 7);
        assert_eq!(owner.started, 1234);
        assert!(parse_stat(&stat("Z")).is_err());
        assert!(parse_stat("42 malformed").is_err());
        assert!(ancestor(std::process::id(), std::process::id()).is_ok());
        assert!(ancestor(std::process::id(), u32::MAX).is_err());
    }
    #[test]
    fn native_binding_survives_attachment_and_ends_with_process() {
        check_native_binding(false);
    }
    #[test]
    fn native_git_binding_registers_and_ends_with_process() {
        check_native_binding(true);
    }
    fn check_native_binding(git_project: bool) {
        let directory = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        if git_project { crate::worktrees::tests::git(project.path(), &["init", "-q"]); }
        let projects = tasks::Projects::new(directory.path());
        let installed = crate::setup::Installed {
            active: true,
            program: "fixture".into(),
            executable: "/bin/fixture".into(),
            adapter: crate::setup::Adapter::Codex("/unused/fixture.toml".into()),
            bridge: "/unused/bridge".into(),
            search_paths: vec![],
            launch_options: Default::default(),
        };
        let preferences = crate::setup::Preferences {
            enabled: true,
            selected: [("fixture".into(), installed)].into_iter().collect(),
            ..Default::default()
        };
        crate::setup::save_preferences(&directory.path().join("mcp-settings.json"), &preferences)
            .unwrap();
        let mut child = crate::native_pty::NativePty::spawn(
            Path::new("/bin/sh"),
            &["-c".into(), "echo $$; /bin/sleep 30 & echo $!; wait".into()],
            project.path(),
            crate::native_pty::Size {
                columns: 80,
                rows: 24,
            },
        )
        .unwrap();
        let mut output = std::io::BufReader::new(&child.reader);
        let mut pid = String::new();
        std::io::BufRead::read_line(&mut output, &mut pid).unwrap();
        let pid: u32 = pid.trim().parse().unwrap();
        let mut peer = String::new();
        std::io::BufRead::read_line(&mut output, &mut peer).unwrap();
        let peer: u32 = peer.trim().parse().unwrap();
        assert_eq!(foreground_owner(pid).unwrap(), pid);
        assert_eq!(foreground_owner(peer).unwrap(), pid);
        use std::os::unix::process::CommandExt;
        let mut detached_command = std::process::Command::new("/bin/sleep");
        detached_command.arg("30");
        unsafe {
            detached_command.pre_exec(|| {
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut detached = detached_command.spawn().unwrap();
        assert!(foreground_owner(detached.id()).is_err());
        detached.kill().unwrap();
        detached.wait().unwrap();
        let fence = ManagedFence {
            service_id: Uuid::new_v4().to_string(),
            service_boot_id: Uuid::new_v4().to_string(),
            connection_id: Uuid::new_v4().to_string(),
            project_id: Uuid::new_v4().to_string(),
        };
        let first = projects
            .guests
            .bind(
                &projects,
                &fence,
                project.path(),
                Some(peer),
                pid,
                "fixture",
            )
            .unwrap();
        let registered = crate::transport::call(&first.endpoint, &crate::transport::Request {
            protocol_major: crate::transport::PROTOCOL_MAJOR,
            terminal: first.terminal.clone(), capability: first.capability.clone(),
            run: None, defer_initial_ready: true, native_activity: None,
            directory: Some(project.path().to_str().unwrap().into()),
            operation: crate::Operation::AgentRegister { name: String::new() },
        }).unwrap();
        assert_eq!(registered["run"].as_str(), Some(first.run.as_str()));
        let mut later = fence.clone();
        later.connection_id = Uuid::new_v4().to_string();
        let second = projects
            .guests
            .bind(
                &projects,
                &later,
                project.path(),
                Some(peer),
                pid,
                "fixture",
            )
            .unwrap();
        assert_eq!(first.run, second.run);
        assert_eq!(first.terminal, second.terminal);
        let observe = |shell_pid| crate::wsl_setup::NativeRequest {
            shell_pid,
            action: crate::wsl_setup::NativeAction::Observe {
                draft: true,
                blocked: true,
                input_epoch: 1,
                submitted: false,
                cancelled: false,
                native_ready: None,
            },
        };
        let native = projects
            .guests
            .native(project.path(), observe(std::process::id()))
            .unwrap()
            .unwrap();
        assert_eq!(native.run, first.run);
        assert!(native.wake.is_none());
        assert!(projects
            .guests
            .native(project.path(), observe(peer))
            .unwrap()
            .is_none());
        assert!(projects
            .guests
            .native(directory.path(), observe(std::process::id()))
            .unwrap()
            .is_none());
        assert!(projects.guests.reap_and_active());
        assert!(projects
            .guests
            .bind(
                &projects,
                &fence,
                project.path(),
                Some(std::process::id()),
                pid,
                "fixture"
            )
            .is_err());
        child.stop().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while child.exit_code().unwrap().is_none() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!projects.guests.reap_and_active());
    }
}
