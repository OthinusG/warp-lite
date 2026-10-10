//! Resolve the confirmed SSH terminal for existing project tools.
use warp_agent_bus::{ssh_files::SshConnection, ssh_remote::SshProfile};
use warpui::{AppContext, SingletonEntity, WindowId};

pub(crate) fn session_connection(
    session: &crate::terminal::model::session::Session,
) -> Option<SshConnection> {
    if session.ssh_arguments().is_none() && !session.is_legacy_ssh_session() {
        if let Some(distribution) = session.wsl_distro_name() {
            return Some(SshConnection::Wsl {
                distribution: distribution.into(), user: session.user().into(),
            });
        }
    }
    let arguments = session.ssh_arguments()?;
    Some(match session.ssh_control_socket() {
        Some(socket) => SshConnection::Multiplexed {
            socket: socket.to_owned(),
            wsl: session.wsl_distro_name().map(str::to_owned),
        },
        None if session.wsl_distro_name().is_some() => return None,
        None => SshConnection::Native {
            arguments: arguments.to_vec(),
            session: format!("{:?}", session.id()),
        },
    })
}

pub(crate) fn selected_ssh(
    app: &AppContext,
    window: WindowId,
) -> Option<(SshProfile, SshConnection)> {
    let active = crate::workspace::ActiveSession::as_ref(app);
    if active.remote_pending(window) {
        return None;
    }
    let session = active.session(window)?;
    let connection = session_connection(&session)?;
    let host_info = session.host_info();
    let (companion_path, remote_shell) = warp_agent_bus::installation::companion_path(
        session.home_dir()?,
        if matches!(connection, SshConnection::Wsl { .. }) { "Linux" }
        else { host_info.os_category.as_deref()? },
    )
    .ok()?;
    let profile = SshProfile {
        target: if matches!(connection, SshConnection::Wsl { .. }) { "wsl".into() } else { session.hostname().into() },
        config_file: None,
        remote_root: active.current_directory(window)?.into(),
        companion_path,
        remote_shell,
    };
    profile.validate().ok()?;
    Some((profile, connection))
}

pub(crate) fn selection_key(profile: &SshProfile, connection: &SshConnection) -> String {
    format!(
        "{}:{}:{}",
        connection.scope_key(),
        profile.target,
        profile.remote_root
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::model::session::{command_executor::NoOpCommandExecutor, Session, SessionInfo};

    #[test]
    fn wsl_selection_keeps_guest_account_and_nested_ssh_distinct() {
        let mut info = SessionInfo::new_for_test().with_user("guest".into());
        info.wsl_name = Some("Ubuntu Test".into());
        let session = Session::new(info.clone(), std::sync::Arc::new(NoOpCommandExecutor::new()));
        assert_eq!(session_connection(&session), Some(SshConnection::Wsl {
            distribution: "Ubuntu Test".into(), user: "guest".into(),
        }));
        let nested = Session::new(info.with_ssh_socket_path("/tmp/ssh-owned".into()),
            std::sync::Arc::new(NoOpCommandExecutor::new()));
        assert!(!matches!(session_connection(&nested), Some(SshConnection::Wsl { .. })),
            "A nested SSH route cannot fall back to its WSL parent");
    }
}
