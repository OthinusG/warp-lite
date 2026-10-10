//! Native WSL terminal delivery; Linux owns run identity and MCP, Windows owns UI guards.
use super::*;
use crate::agent_communication::wsl_settings::Target;
use warp_agent_bus::{
    companion::TaskCommand,
    readiness::{Activity, Draft},
    ssh_remote::{HostClient, SshProfile},
    transport::Wake,
    wsl_setup::{NativeAction, NativeRequest, NativeState},
};

type Client = Arc<tokio::sync::Mutex<HostClient>>;

#[derive(Clone, PartialEq, Eq)]
struct Context {
    session: SessionId,
    target: Target,
    shell_pid: u32,
    root: String,
}

#[derive(Default)]
pub(super) struct State {
    context: Option<Context>,
    client: Option<Client>,
    busy: bool,
    attempted: Option<std::time::Instant>,
    draft: Draft,
    epoch: u64,
    submitted: bool,
    cancelled: bool,
}

impl State {
    fn select(&mut self, context: Option<Context>) {
        if context == self.context {
            return;
        }
        self.context = context;
        self.client = None;
        self.busy = false;
        self.attempted = None;
        self.draft.clear();
        self.epoch = self.epoch.wrapping_add(1);
        self.submitted = false;
        self.cancelled = false;
    }
    pub(super) fn user_input(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        self.epoch = self.epoch.wrapping_add(1);
        match self.draft.input(bytes) {
            Some(Activity::Working) => {
                self.submitted = true;
                self.cancelled = false;
            }
            Some(Activity::Cancelled) => {
                self.submitted = false;
                self.cancelled = true;
            }
            _ => {
                self.submitted = false;
            }
        }
    }
}

async fn request(
    client: &Client,
    shell_pid: u32,
    action: NativeAction,
) -> anyhow::Result<Option<NativeState>> {
    tokio::time::timeout(Duration::from_secs(10), async {
        let value = client
            .lock()
            .await
            .project_tasks(
                &TaskCommand::GuestNative(NativeRequest { shell_pid, action }),
                0,
            )
            .await
            .map_err(|_| anyhow::anyhow!("WSL native delivery unavailable"))?;
        let value = value
            .get("value")
            .ok_or_else(|| anyhow::anyhow!("Invalid WSL native response"))?;
        Ok(serde_json::from_value(value.clone())?)
    })
    .await?
}

impl TerminalView {
    pub(super) fn record_wsl_peer_input(&mut self, bytes: &[u8], ctx: &AppContext) {
        let context = self.wsl_peer_context(ctx);
        self.wsl_peer.select(context);
        self.wsl_peer.user_input(bytes);
    }
    fn wsl_peer_context(&self, ctx: &AppContext) -> Option<Context> {
        if !self.is_long_running_and_user_controlled() {
            return None;
        }
        let session_id = self.active_block_session_id()?;
        let session = self.sessions_model().as_ref(ctx).get(session_id)?;
        let target = Target::from_session(&session)?;
        let shell_pid = session.wsl_shell_pid()?;
        let root = self
            .model
            .lock()
            .active_block_metadata()
            .current_working_directory()?
            .to_owned();
        Some(Context {
            session: session_id,
            target,
            shell_pid,
            root,
        })
    }

    fn wsl_peer_valid(&self, context: &Context, epoch: u64, ctx: &AppContext) -> bool {
        let settings = &crate::agent_communication::AgentCommunication::as_ref(ctx).wsl;
        settings.preferences.enabled
            && settings.targets.contains(&context.target)
            && self.wsl_peer.epoch == epoch
            && self.wsl_peer.draft.is_empty()
            && !self.wsl_peer.cancelled
            && self.wsl_peer_context(ctx).as_ref() == Some(context)
            && self.peer_input_is_empty(ctx)
    }

    pub(crate) fn poll_wsl_peer_work(
        &mut self,
        enabled: bool,
        targets: &[Target],
        ctx: &mut ViewContext<Self>,
    ) {
        let context = self.wsl_peer_context(ctx);
        self.wsl_peer.select(context.clone());
        let Some(context) = context else {
            return;
        };
        if !enabled || !targets.contains(&context.target) {
            self.wsl_peer.client = None;
            return;
        }
        if self.wsl_peer.busy
            || self
                .wsl_peer
                .attempted
                .is_some_and(|time| time.elapsed() < Duration::from_secs(1))
        {
            return;
        }
        let (draft, blocked) = self.peer_input_guard(ctx);
        let native_ready = CLIAgentSessionsModel::as_ref(ctx)
            .session(self.view_id)
            .filter(|session| session.listener.is_some())
            .map(|session| {
                matches!(
                    session.status,
                    crate::terminal::cli_agent_sessions::CLIAgentSessionStatus::Success
                )
            });
        let epoch = self.wsl_peer.epoch;
        let action = NativeAction::Observe {
            draft: draft || !self.wsl_peer.draft.is_empty(),
            blocked,
            input_epoch: epoch,
            submitted: self.wsl_peer.submitted,
            cancelled: self.wsl_peer.cancelled,
            native_ready,
        };
        self.wsl_peer.busy = true;
        self.wsl_peer.attempted = Some(std::time::Instant::now());
        let cached = self.wsl_peer.client.clone();
        let selected = context.clone();
        ctx.spawn(async move {
            let client = match cached {
                Some(client) => client,
                None => {
                    let profile = SshProfile {
                        target: "wsl".into(), config_file: None, remote_root: selected.root.clone(),
                        companion_path: warp_agent_bus::installation::wsl_companion_path(&selected.target.home)
                            .map_err(|_| anyhow::anyhow!("Invalid Linux home"))?,
                        remote_shell: warp_agent_bus::ssh_remote::RemoteShell::Posix,
                    };
                    Arc::new(tokio::sync::Mutex::new(HostClient::connect_wsl(&profile,
                        &selected.target.distribution, &selected.target.user).await
                        .map_err(|_| anyhow::anyhow!("WSL service unavailable"))?))
                }
            };
            let state = request(&client, selected.shell_pid, action).await?;
            let state = match state {
                Some(state) if state.wake.is_some() => request(&client, selected.shell_pid,
                    NativeAction::Claim { wake: state.wake.unwrap() }).await?,
                state => state,
            };
            Ok::<_, anyhow::Error>((client, state))
        }, move |view, result, ctx| {
            if view.wsl_peer.context.as_ref() != Some(&context) { return; }
            let (client, state) = match result {
                Ok(value) => value,
                Err(_) => { view.wsl_peer.client = None; view.wsl_peer.busy = false; return; }
            };
            view.wsl_peer.client = Some(client.clone());
            let Some(state) = state.filter(|state| state.valid) else { view.wsl_peer.busy = false; return; };
            let Some(wake) = state.wake else { view.wsl_peer.busy = false; return; };
            let agent = CLIAgentSessionsModel::as_ref(ctx).session(view.view_id).map(|session| session.agent)
                .filter(|agent| agent.command_prefix() == state.program || *agent == CLIAgent::Unknown);
            if uuid::Uuid::parse_str(&wake.message_id).is_err() || agent.is_none()
                || !view.wsl_peer_valid(&context, epoch, ctx) {
                view.finish_wsl_wake(client, context, wake, false, ctx);
                return;
            }
            let text = format!("Warpai peer work is waiting (message {}). Call warp_agent_inbox, process this message or task, and acknowledge it through MCP. Before ending your turn, call warp_agent_ready.", wake.message_id);
            let delay = view.stage_wsl_peer_text(agent.unwrap(), text, ctx);
            let connection = client.clone();
            let claimed = wake.clone();
            let shell_pid = context.shell_pid;
            ctx.spawn(async move {
                Timer::after(delay).await;
                request(&connection, shell_pid, NativeAction::Validate { wake: claimed }).await
            }, move |view, result, ctx| {
                let valid = result.ok().flatten().is_some_and(|state| state.valid && state.run == wake.run);
                let submitted = valid && view.wsl_peer_valid(&context, epoch, ctx);
                if submitted { view.write_to_pty(b"\r".to_vec(), ctx); }
                view.finish_wsl_wake(client, context, wake, submitted, ctx);
            });
        });
    }

    fn finish_wsl_wake(
        &mut self,
        client: Client,
        context: Context,
        wake: Wake,
        submitted: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        let shell_pid = context.shell_pid;
        ctx.spawn(async move { request(&client, shell_pid, NativeAction::Finish { wake, submitted }).await },
            move |view, result, _| {
                if view.wsl_peer.context.as_ref() == Some(&context) {
                    view.wsl_peer.busy = false;
                    if result.is_err() { view.wsl_peer.client = None; }
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manual_input_and_cancel_fence_native_delivery() {
        let mut state = State::default();
        state.user_input("几个字".as_bytes());
        assert!(!state.draft.is_empty());
        assert_eq!(state.epoch, 1);
        state.user_input(b"\x7f\x7f\x7f");
        assert!(state.draft.is_empty());
        state.user_input(b"\x03");
        assert!(state.cancelled);
        assert!(!state.draft.is_empty());
        state.user_input(b"continue\r");
        assert!(state.submitted && !state.cancelled && state.draft.is_empty());
        assert_eq!(state.epoch, 4);
    }
}
