//! Resolve the confirmed SSH terminal for existing project tools.
use warp_agent_bus::{ssh_files::SshConnection, ssh_remote::SshProfile};
use warpui::{AppContext, SingletonEntity, WindowId};

pub(crate) fn session_connection(
    session: &crate::terminal::model::session::Session,
) -> Option<SshConnection> {
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
    if let Some(source) = active.file_source(window) {
        let mut profile = source.files.profile.clone();
        profile.remote_root = source.files.canonical_root.clone();
        return Some((profile, source.files.connection.clone()));
    }
    if active.remote_pending(window) {
        return None;
    }
    let session = active.session(window)?;
    let connection = session_connection(&session)?;
    let (companion_path, remote_shell) = warp_agent_bus::installation::companion_path(
        session.home_dir()?,
        session.host_info().os_category.as_deref()?,
    )
    .ok()?;
    let profile = SshProfile {
        target: session.hostname().into(),
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
