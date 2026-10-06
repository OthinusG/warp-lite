//! Existing SSH selection shared by file tools. Content uses the system SFTP client.
use super::ssh_remote::{ConnectionError, HostClient, SshProfile};
use remote_protocol::proto::*;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{io::AsyncWriteExt, process::Command, sync::Mutex};

#[derive(Clone, Debug)]
pub enum SshConnection {
    Multiplexed {
        socket: PathBuf,
        wsl: Option<String>,
    },
    Native {
        arguments: Vec<String>,
        session: String,
    },
}
impl SshConnection {
    pub fn scope_key(&self) -> String {
        match self {
            Self::Multiplexed { socket, wsl } => format!("master:{socket:?}:{wsl:?}"),
            Self::Native { session, .. } => format!("native:{session}"),
        }
    }
    pub async fn connect(&self, profile: &SshProfile) -> Result<HostClient, ConnectionError> {
        match self {
            Self::Multiplexed { socket, wsl } => {
                HostClient::connect_session(profile, socket, wsl.as_deref()).await
            }
            Self::Native { arguments, .. } => {
                HostClient::connect_arguments(profile, arguments).await
            }
        }
    }
    fn sftp_command(&self) -> Result<Command, ConnectionError> {
        let mut command = match self {
            Self::Multiplexed {
                wsl: Some(distribution),
                ..
            } => {
                let mut command = Command::new("wsl");
                command.args(["--distribution", distribution, "--exec", "sftp"]);
                command
            }
            _ => Command::new("sftp"),
        };
        crate::session::without_terminal_binding(&mut command);
        command.env_remove("VIBE_MCP_SERVERS");
        command.args([
            "-b",
            "-",
            "-oBatchMode=yes",
            "-oStrictHostKeyChecking=yes",
            "-oForwardAgent=no",
            "-oForwardX11=no",
            "-oPermitLocalCommand=no",
            "-oConnectTimeout=10",
            "-oClearAllForwardings=yes",
        ]);
        match self {
            Self::Multiplexed { socket, .. } => {
                let path = socket.to_str().ok_or(ConnectionError::InvalidProfile)?;
                if path.chars().any(char::is_control) {
                    return Err(ConnectionError::InvalidProfile);
                }
                command.args(["-oControlMaster=no", "-oProxyCommand=false"]);
                command.arg(format!("-oControlPath={path}"));
                command.arg("warpai-session");
            }
            Self::Native { arguments, .. } => {
                command.args(sftp_arguments(arguments)?);
            }
        }
        command.kill_on_drop(true);
        Ok(command)
    }
    async fn local_sftp_path(&self, path: &Path) -> Result<String, ConnectionError> {
        let path = path.to_str().ok_or(ConnectionError::InvalidInput)?;
        if let Self::Multiplexed {
            wsl: Some(distribution),
            ..
        } = self
        {
            use tokio::io::AsyncReadExt;
            let mut child = Command::new("wsl")
                .args([
                    "--distribution",
                    distribution,
                    "--exec",
                    "wslpath",
                    "-u",
                    path,
                ])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .map_err(|_| ConnectionError::SshUnavailable)?;
            let bytes = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                let output = child.stdout.take().ok_or(ConnectionError::ConnectionLost)?;
                let mut bytes = Vec::new();
                output
                    .take(4097)
                    .read_to_end(&mut bytes)
                    .await
                    .map_err(|_| ConnectionError::ConnectionLost)?;
                if bytes.len() > 4096 {
                    return Err(ConnectionError::InvalidInput);
                }
                if !child
                    .wait()
                    .await
                    .map_err(|_| ConnectionError::ConnectionLost)?
                    .success()
                {
                    return Err(ConnectionError::InvalidInput);
                }
                Ok(bytes)
            })
            .await
            .map_err(|_| ConnectionError::ConnectionLost)??;
            String::from_utf8(bytes)
                .map(|v| v.trim_end_matches(['\r', '\n']).to_owned())
                .map_err(|_| ConnectionError::InvalidInput)
        } else {
            Ok(path.replace('\\', "/"))
        }
    }
}

/// Translate admitted SSH flags; notably ssh -p/-b/-l have different SFTP meanings.
fn sftp_arguments(arguments: &[String]) -> Result<Vec<String>, ConnectionError> {
    if arguments.len() > 128 || arguments.iter().any(|arg| arg.len() > 4096) {
        return Err(ConnectionError::InvalidProfile);
    }
    let mut result = Vec::new();
    let (destination, options) = arguments
        .split_last()
        .ok_or(ConnectionError::InvalidProfile)?;
    if destination.starts_with('-')
        || destination.is_empty()
        || destination.chars().any(char::is_control)
    {
        return Err(ConnectionError::InvalidProfile);
    }
    let mut index = 0;
    while index < options.len() {
        let option = &options[index];
        if matches!(option.as_str(), "-4" | "-6" | "-C") {
            result.push(option.clone());
            index += 1;
            continue;
        }
        if option == "--" {
            index += 1;
            continue;
        }
        let flag = option.get(..2).ok_or(ConnectionError::InvalidProfile)?;
        let value = if option.len() == 2 {
            index += 1;
            options
                .get(index)
                .ok_or(ConnectionError::InvalidProfile)?
                .clone()
        } else {
            option[2..].to_owned()
        };
        if value.is_empty() || value.chars().any(char::is_control) {
            return Err(ConnectionError::InvalidProfile);
        }
        match flag {
            "-F" | "-i" | "-J" | "-o" => {
                result.push(flag.into());
                result.push(value);
            }
            "-p" => {
                result.push("-P".into());
                result.push(value);
            }
            "-l" | "-b" | "-B" | "-c" | "-m" | "-I" => {
                let name = match flag {
                    "-l" => "User",
                    "-b" => "BindAddress",
                    "-B" => "BindInterface",
                    "-c" => "Ciphers",
                    "-m" => "MACs",
                    _ => "PKCS11Provider",
                };
                result.push(format!("-o{name}={value}"));
            }
            _ => return Err(ConnectionError::InvalidProfile),
        }
        index += 1;
    }
    result.push(destination.clone());
    Ok(result)
}

fn batch_path(path: &str) -> Result<String, ConnectionError> {
    if path.is_empty() || path.chars().any(char::is_control) {
        return Err(ConnectionError::InvalidInput);
    }
    let mut result = String::from("\"");
    for character in path.chars() {
        // OpenSSH's batch parser escapes glob characters inside quotes itself.
        if "\\\"".contains(character) {
            result.push('\\');
        }
        result.push(character);
    }
    result.push('"');
    Ok(result)
}

fn relative_path(root: &str, path: &str, windows: bool) -> Result<String, ConnectionError> {
    let normalize = |s: &str| {
        if windows {
            s.replace('\\', "/")
        } else {
            s.to_owned()
        }
    };
    let root = normalize(root);
    let path = normalize(path);
    let relative = if path == root || (windows && path.eq_ignore_ascii_case(&root)) {
        ""
    } else {
        let prefix = format!("{}/", root.trim_end_matches('/'));
        if windows
            && path
                .get(..prefix.len())
                .is_some_and(|value| value.eq_ignore_ascii_case(&prefix))
        {
            &path[prefix.len()..]
        } else {
            path.strip_prefix(&prefix)
                .ok_or(ConnectionError::InvalidInput)?
        }
    };
    if path.len() > 4096
        || relative.chars().any(char::is_control)
        || relative
            .split('/')
            .any(|part| matches!(part, "." | "..") || (part.is_empty() && !relative.is_empty()))
    {
        return Err(ConnectionError::InvalidInput);
    }
    Ok(relative.into())
}

fn cache_path(directory: &Path, relative: &str) -> Result<PathBuf, ConnectionError> {
    if relative.is_empty() {
        return Err(ConnectionError::InvalidInput);
    }
    // Remote names need not be representable on the desktop (Windows drives,
    // reserved names and case aliases). A digest prevents local traversal.
    let name = format!("{:x}", Sha256::digest(relative.as_bytes()));
    let extension = Path::new(relative)
        .extension()
        .and_then(|v| v.to_str())
        .filter(|v| v.len() <= 16 && v.bytes().all(|b| b.is_ascii_alphanumeric()));
    Ok(directory.join(match extension {
        Some(extension) => format!("{name}.{extension}"),
        None => name,
    }))
}

async fn transfer_batch(mut command: Command, batch: &str) -> Result<(), ConnectionError> {
    command.kill_on_drop(true);
    let mut child = command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| ConnectionError::SshUnavailable)?;
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut input = child.stdin.take().ok_or(ConnectionError::ConnectionLost)?;
        input
            .write_all(batch.as_bytes())
            .await
            .map_err(|_| ConnectionError::ConnectionLost)?;
        drop(input);
        let status = child
            .wait()
            .await
            .map_err(|_| ConnectionError::ConnectionLost)?;
        if status.success() {
            Ok(())
        } else {
            Err(ConnectionError::ConnectionLost)
        }
    })
    .await
    .unwrap_or(Err(ConnectionError::ConnectionLost))
}

pub struct RemoteFiles {
    pub profile: SshProfile,
    pub connection: SshConnection,
    pub canonical_root: String,
    pub identity: String,
    client: Mutex<HostClient>,
    connected: AtomicBool,
    cache: tempfile::TempDir,
}

/// A canceled transfer closes its control attachment so owned staging is reclaimed.
struct TransferCancellation<'a>(Option<&'a RemoteFiles>);
impl Drop for TransferCancellation<'_> {
    fn drop(&mut self) {
        if let Some(files) = self.0 {
            files.disconnect();
        }
    }
}
impl std::fmt::Debug for RemoteFiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteFiles")
            .field("root", &self.canonical_root)
            .finish_non_exhaustive()
    }
}
impl RemoteFiles {
    /// Connect only to the explicitly provisioned native acceptance server.
    #[cfg(debug_assertions)]
    pub async fn connect_fixture() -> Result<Arc<Self>, ConnectionError> {
        let value = |name| std::env::var(name).map_err(|_| ConnectionError::InvalidProfile);
        Self::connect(
            SshProfile {
                target: "warpai-test".into(),
                config_file: None,
                remote_root: value("WARP_TEST_REMOTE_ROOT")?,
                companion_path: value("WARP_TEST_COMPANION_PATH")?,
                remote_shell: if cfg!(windows) {
                    super::ssh_remote::RemoteShell::PowerShell
                } else {
                    super::ssh_remote::RemoteShell::Posix
                },
            },
            SshConnection::Native {
                arguments: vec![
                    "-F".into(),
                    value("WARP_TEST_SSH_CONFIG")?,
                    "warpai-test".into(),
                ],
                session: "owned-native-file-fixture".into(),
            },
        )
        .await
    }
    pub async fn connect(
        profile: SshProfile,
        connection: SshConnection,
    ) -> Result<Arc<Self>, ConnectionError> {
        let client = connection.connect(&profile).await?;
        if !client.capabilities().iter().any(|c| c == "project_files") {
            return Err(ConnectionError::FeatureUnavailable);
        }
        let identity = format!(
            "{}:{}:{}:{}",
            connection.scope_key(),
            client.account_id,
            client.fence().unwrap().service_id,
            client.root_identity
        );
        let canonical_root = client.canonical_root.clone();
        let cache = tempfile::Builder::new()
            .prefix("warpai-ssh-")
            .tempdir()
            .map_err(|_| ConnectionError::CapacityExceeded)?;
        Ok(Arc::new(Self {
            profile,
            connection,
            canonical_root,
            identity,
            client: Mutex::new(client),
            connected: AtomicBool::new(true),
            cache,
        }))
    }
    pub fn connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }
    pub fn disconnect(&self) {
        self.connected.store(false, Ordering::Release);
        if let Ok(mut client) = self.client.try_lock() {
            client.disconnect();
        }
    }

    /// Explicit reconnect preserves the original account and project identity.
    pub async fn reconnect(&self) -> Result<(), ConnectionError> {
        let mut previous = self.client.lock().await;
        let mut replacement = self.connection.connect(&self.profile).await?;
        if replacement.account_id != previous.account_id
            || replacement.root_identity != previous.root_identity
            || replacement.canonical_root != previous.canonical_root
            || !replacement
                .capabilities()
                .iter()
                .any(|capability| capability == "project_files")
        {
            replacement.disconnect();
            return Err(ConnectionError::StaleAttachment);
        }
        previous.disconnect();
        *previous = replacement;
        self.connected.store(true, Ordering::Release);
        Ok(())
    }
    pub fn relative(&self, path: &str) -> Result<String, ConnectionError> {
        relative_path(
            &self.canonical_root,
            path,
            matches!(
                self.profile.remote_shell,
                super::ssh_remote::RemoteShell::PowerShell
            ),
        )
    }
    pub fn cache_path(&self, path: &str) -> Result<PathBuf, ConnectionError> {
        cache_path(self.cache.path(), &self.relative(path)?)
    }

    /// Resolve document links using the remote platform, never the desktop filesystem.
    pub fn document_link(&self, document: &str, link: &str) -> Result<String, ConnectionError> {
        if link.contains('\0') || link.len() > 4096 || link.contains("://") {
            return Err(ConnectionError::InvalidInput);
        }
        let link = link.split('#').next().unwrap_or(link);
        let mut decoded = Vec::with_capacity(link.len());
        let mut bytes = link.as_bytes().iter().copied();
        while let Some(byte) = bytes.next() {
            if byte == b'%' {
                let high = bytes.next().and_then(|byte| (byte as char).to_digit(16));
                let low = bytes.next().and_then(|byte| (byte as char).to_digit(16));
                let (high, low) = high.zip(low).ok_or(ConnectionError::InvalidInput)?;
                decoded.push((high * 16 + low) as u8);
            } else {
                decoded.push(byte);
            }
        }
        let link = String::from_utf8(decoded).map_err(|_| ConnectionError::InvalidInput)?;
        let windows = matches!(
            self.profile.remote_shell,
            super::ssh_remote::RemoteShell::PowerShell
        );
        let document = if windows {
            document.replace('\\', "/")
        } else {
            document.into()
        };
        let link = if windows {
            link.replace('\\', "/")
        } else {
            link
        };
        let joined = if link.starts_with('/') || (windows && link.as_bytes().get(1) == Some(&b':'))
        {
            link
        } else {
            format!(
                "{}/{}",
                document
                    .rsplit_once('/')
                    .ok_or(ConnectionError::InvalidInput)?
                    .0,
                link
            )
        };
        let mut parts = Vec::new();
        for part in joined.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop().ok_or(ConnectionError::InvalidInput)?;
                }
                _ => parts.push(part),
            }
        }
        let prefix = if windows && joined.starts_with("//") {
            "//"
        } else if windows {
            ""
        } else {
            "/"
        };
        let path = format!("{prefix}{}", parts.join("/"));
        self.relative(&path)?;
        Ok(path)
    }
    pub async fn control(
        &self,
        request: ProjectFilesRequest,
    ) -> Result<ProjectFilesResult, ConnectionError> {
        if !self.connected() {
            return Err(ConnectionError::ConnectionLost);
        }
        let mut client = self.client.lock().await;
        if !self.connected() {
            client.disconnect();
            return Err(ConnectionError::ConnectionLost);
        }
        let result = client.project_files(request).await;
        if matches!(
            result,
            Err(ConnectionError::ConnectionLost
                | ConnectionError::StaleAttachment
                | ConnectionError::IncompatibleVersion)
        ) {
            client.disconnect();
            self.disconnect();
        }
        result
    }
    pub async fn list(
        &self,
        path: &str,
        generation: u64,
    ) -> Result<RepoMetadataSnapshot, ConnectionError> {
        self.control(ProjectFilesRequest {
            action: ProjectFileAction::ProjectFileList as i32,
            path: self.relative(path)?,
            query_generation: generation,
            ..Default::default()
        })
        .await?
        .snapshot
        .ok_or(ConnectionError::IncompatibleVersion)
    }
    async fn transfer(
        &self,
        staged: &str,
        local: &Path,
        upload: bool,
    ) -> Result<(), ConnectionError> {
        let result = async {
            let local = self.connection.local_sftp_path(local).await?;
            let remote = if matches!(
                self.profile.remote_shell,
                super::ssh_remote::RemoteShell::PowerShell
            ) {
                staged.replace('\\', "/")
            } else {
                staged.into()
            };
            let (verb, source, destination) = if upload {
                ("put", local, remote)
            } else {
                ("get", remote, local)
            };
            let batch = format!(
                "{verb} {} {}\n",
                batch_path(&source)?,
                batch_path(&destination)?
            );
            transfer_batch(self.connection.sftp_command()?, &batch).await
        }
        .await;
        if result.is_err() {
            self.disconnect();
        }
        result
    }
    pub async fn download(&self, path: &str) -> Result<(PathBuf, String), ConnectionError> {
        let relative = self.relative(path)?;
        let mut cancellation = TransferCancellation(Some(self));
        let staged = self
            .control(ProjectFilesRequest {
                action: ProjectFileAction::ProjectFilePrepareRead as i32,
                path: relative,
                ..Default::default()
            })
            .await;
        let staged = match staged {
            Ok(staged) => staged,
            Err(error) => {
                cancellation.0 = None;
                return Err(error);
            }
        };
        let result = self.download_staged(path, staged).await;
        cancellation.0 = None;
        result
    }
    pub async fn download_base(
        &self,
        path: &str,
        reference: &str,
    ) -> Result<(PathBuf, String), ConnectionError> {
        let relative = self.relative(path)?;
        let mut cancellation = TransferCancellation(Some(self));
        let staged = self
            .control(ProjectFilesRequest {
                action: ProjectFileAction::ProjectGitPrepareBase as i32,
                path: relative,
                destination: reference.into(),
                ..Default::default()
            })
            .await;
        let staged = match staged {
            Ok(staged) => staged,
            Err(error) => {
                cancellation.0 = None;
                return Err(error);
            }
        };
        // Base versions use a separate cache namespace so they cannot overwrite working files.
        let result = self
            .download_staged(
                &format!(
                    "{}{}base-{}",
                    self.canonical_root,
                    if self.canonical_root.ends_with('/') {
                        ""
                    } else {
                        "/"
                    },
                    staged.transfer_id
                ),
                staged,
            )
            .await;
        cancellation.0 = None;
        result
    }
    async fn download_staged(
        &self,
        path: &str,
        staged: ProjectFilesResult,
    ) -> Result<(PathBuf, String), ConnectionError> {
        let result = async {
            let destination = self.cache_path(path)?;
            std::fs::create_dir_all(destination.parent().unwrap())
                .map_err(|_| ConnectionError::CapacityExceeded)?;
            let temporary = tempfile::NamedTempFile::new_in(destination.parent().unwrap())
                .map_err(|_| ConnectionError::CapacityExceeded)?;
            self.transfer(&staged.transfer_path, temporary.path(), false)
                .await?;
            let metadata = temporary
                .as_file()
                .metadata()
                .map_err(|_| ConnectionError::InvalidInput)?;
            if metadata.len() != staged.size
                || metadata.len() > super::companion::files::MAX_FILE_BYTES
            {
                return Err(ConnectionError::CapacityExceeded);
            }
            let bytes =
                std::fs::read(temporary.path()).map_err(|_| ConnectionError::InvalidInput)?;
            if format!("{:x}", Sha256::digest(&bytes)) != staged.sha256 {
                self.disconnect();
                return Err(ConnectionError::StaleAttachment);
            }
            // Do not evict cache files that may back unsaved editor buffers.
            let entries = std::fs::read_dir(self.cache.path())
                .map_err(|_| ConnectionError::CapacityExceeded)?;
            let mut total = metadata.len();
            let mut count = 0;
            for entry in entries {
                let entry = entry.map_err(|_| ConnectionError::CapacityExceeded)?;
                if entry.path() == destination || entry.path() == temporary.path() {
                    continue;
                }
                total = total.saturating_add(
                    entry
                        .metadata()
                        .map_err(|_| ConnectionError::CapacityExceeded)?
                        .len(),
                );
                count += 1;
                if total > 128 * 1024 * 1024 || count > 1024 {
                    return Err(ConnectionError::CapacityExceeded);
                }
            }
            temporary
                .persist(&destination)
                .map_err(|_| ConnectionError::CapacityExceeded)?;
            Ok((destination, staged.sha256.clone()))
        }
        .await;
        let release = self
            .control(ProjectFilesRequest {
                action: ProjectFileAction::ProjectFileRelease as i32,
                transfer_id: staged.transfer_id,
                ..Default::default()
            })
            .await;
        result.and_then(|value| release.map(|_| value))
    }
    pub async fn save(
        &self,
        path: &str,
        content: &[u8],
        expected_hash: &str,
    ) -> Result<String, ConnectionError> {
        use std::io::Write;
        if content.len() as u64 > super::companion::files::MAX_FILE_BYTES {
            return Err(ConnectionError::CapacityExceeded);
        }
        let relative = self.relative(path)?;
        let mut cancellation = TransferCancellation(Some(self));
        let staged = self
            .control(ProjectFilesRequest {
                action: ProjectFileAction::ProjectFilePrepareWrite as i32,
                path: relative.clone(),
                expected_hash: expected_hash.into(),
                ..Default::default()
            })
            .await;
        let staged = match staged {
            Ok(staged) => staged,
            Err(error) => {
                cancellation.0 = None;
                return Err(error);
            }
        };
        let result = async {
            let mut temporary = tempfile::NamedTempFile::new_in(self.cache.path())
                .map_err(|_| ConnectionError::CapacityExceeded)?;
            temporary
                .write_all(content)
                .map_err(|_| ConnectionError::CapacityExceeded)?;
            self.transfer(&staged.transfer_path, temporary.path(), true)
                .await?;
            let hash = format!("{:x}", Sha256::digest(content));
            let reply = self
                .control(ProjectFilesRequest {
                    action: ProjectFileAction::ProjectFileCommitWrite as i32,
                    path: relative,
                    transfer_id: staged.transfer_id.clone(),
                    expected_hash: hash.clone(),
                    ..Default::default()
                })
                .await?;
            if reply.sha256 != hash {
                self.disconnect();
                return Err(ConnectionError::StaleAttachment);
            }
            Ok(hash)
        }
        .await;
        if result.is_err() {
            let _ = self
                .control(ProjectFilesRequest {
                    action: ProjectFileAction::ProjectFileRelease as i32,
                    transfer_id: staged.transfer_id,
                    ..Default::default()
                })
                .await;
        }
        cancellation.0 = None;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[tokio::test(start_paused = true)]
    async fn missing_failed_and_stalled_transfer_processes_return_errors() {
        assert_eq!(
            transfer_batch(Command::new("/nonexistent-warpai-test-sftp"), "quit\n")
                .await
                .unwrap_err(),
            ConnectionError::SshUnavailable
        );
        assert_eq!(
            transfer_batch(Command::new("/usr/bin/false"), "quit\n")
                .await
                .unwrap_err(),
            ConnectionError::ConnectionLost
        );
        let mut stalled = Command::new("/bin/sleep");
        stalled.arg("60");
        assert_eq!(
            transfer_batch(stalled, "quit\n").await.unwrap_err(),
            ConnectionError::ConnectionLost
        );
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn system_sftp_transfers_literal_paths_and_binary_bytes() {
        // Run the real native subsystem without authentication or personal SSH config.
        let server = ["/usr/libexec/sftp-server", "/usr/lib/openssh/sftp-server"]
            .into_iter()
            .find(|path| Path::new(path).is_file())
            .expect("System SFTP subsystem");
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("quotes ' \" bracket[1]*? 多语言.bin");
        let downloaded = directory.path().join("downloaded.bin");
        let uploaded = directory.path().join("uploaded ' \" [2]*?.bin");
        let bytes = b"\0literal binary\xff\r\n";
        std::fs::write(&original, bytes).unwrap();
        let batch = format!(
            "get {} {}\nput {} {}\n",
            batch_path(original.to_str().unwrap()).unwrap(),
            batch_path(downloaded.to_str().unwrap()).unwrap(),
            batch_path(downloaded.to_str().unwrap()).unwrap(),
            batch_path(uploaded.to_str().unwrap()).unwrap()
        );
        let mut child = Command::new("sftp")
            .args(["-D", server, "-b", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        input.write_all(batch.as_bytes()).await.unwrap();
        drop(input);
        let output =
            tokio::time::timeout(std::time::Duration::from_secs(10), child.wait_with_output())
                .await
                .unwrap()
                .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(std::fs::read(downloaded).unwrap(), bytes);
        assert_eq!(std::fs::read(uploaded).unwrap(), bytes);
    }
    #[test]
    fn native_paths_cannot_escape_or_alias_in_the_desktop_cache() {
        assert_eq!(
            relative_path("/", "/project/x", false).unwrap(),
            "project/x"
        );
        assert_eq!(relative_path("/project", "/project", false).unwrap(), "");
        assert_eq!(
            relative_path("C:/Project", "c:/project/FILE.md", true).unwrap(),
            "FILE.md"
        );
        assert!(relative_path("C:/Project", "c:/project-other/file", true).is_err());

        assert_eq!(
            relative_path("C:\\project", "C:\\project\\目录\\file.md", true).unwrap(),
            "目录/file.md"
        );
        assert_eq!(
            relative_path("\\\\server\\share", "\\\\server\\share\\file", true).unwrap(),
            "file"
        );
        for bad in [
            "/project-other/x",
            "/project/../x",
            "/project/./x",
            "/project//x",
            "/project/x\nput /other",
        ] {
            assert!(relative_path("/project", bad, false).is_err());
        }
        let cache = tempfile::tempdir().unwrap();
        let mut unique = std::collections::HashSet::new();
        for remote in [
            "C:/reset.md",
            "CON",
            "NUL.txt",
            "a:b",
            "A.txt",
            "a.txt",
            "a.",
            "a",
            "a ",
            "a/b",
            "a\\b",
            "Unicode 多语言.md",
        ] {
            let local = cache_path(cache.path(), remote).unwrap();
            assert_eq!(local.parent(), Some(cache.path()));
            assert!(unique.insert(local));
        }
        assert!(cache_path(cache.path(), "").is_err());
        assert_eq!(
            cache_path(cache.path(), "目录/file.md")
                .unwrap()
                .extension()
                .unwrap(),
            "md"
        );
    }

    #[test]
    fn malformed_routes_are_rejected_before_sftp_launch() {
        for input in [
            vec![],
            vec!["-host"],
            vec!["-p", "host"],
            vec!["-o", "", "host"],
            vec!["-oBad\ncommand", "host"],
            vec!["-L", "port", "host"],
        ] {
            let args = input.into_iter().map(str::to_owned).collect::<Vec<_>>();
            assert!(sftp_arguments(&args).is_err());
        }
        assert!(sftp_arguments(&vec!["-4".into(); 129]).is_err());
        let original = [
            "-Fconf",
            "-iidentity",
            "-Jjump",
            "-oUser=user",
            "-p2222",
            "-Binterface",
            "-ccipher",
            "-mmac",
            "-Iprovider",
            "-4",
            "--",
            "user@[::1]",
        ];
        assert_eq!(
            sftp_arguments(&original.map(str::to_owned)).unwrap(),
            [
                "-F",
                "conf",
                "-i",
                "identity",
                "-J",
                "jump",
                "-o",
                "User=user",
                "-P",
                "2222",
                "-oBindInterface=interface",
                "-oCiphers=cipher",
                "-oMACs=mac",
                "-oPKCS11Provider=provider",
                "-4",
                "user@[::1]"
            ]
        );
        for bad in ["", "name\0", "name\r", "name\t"] {
            assert!(batch_path(bad).is_err());
        }
    }
    #[test]
    fn sftp_preserves_route_without_ssh_flag_collisions_or_shell_expansion() {
        let input = [
            "-p",
            "2222",
            "-lresearch",
            "-b",
            "127.0.0.1",
            "-J",
            "jump",
            "-i",
            "/key with spaces",
            "user@host",
        ]
        .map(str::to_owned);
        assert_eq!(
            sftp_arguments(&input).unwrap(),
            [
                "-P",
                "2222",
                "-oUser=research",
                "-oBindAddress=127.0.0.1",
                "-J",
                "jump",
                "-i",
                "/key with spaces",
                "user@host"
            ]
        );
        assert_eq!(
            batch_path("/notes/it's $(literal) [1]*?.md").unwrap(),
            "\"/notes/it's $(literal) [1]*?.md\""
        );
        assert!(batch_path("/name\nput other").is_err());
        assert!(sftp_arguments(&["-L".into(), "forward".into(), "host".into()]).is_err());
    }
}
