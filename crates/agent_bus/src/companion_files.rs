//! Project-scoped file control. Content crosses SSH only through SFTP staging.
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

use remote_protocol::proto::*;
use sha2::{Digest, Sha256};
#[cfg(windows)]
use std::fs::OpenOptions;
use uuid::Uuid;

use super::{identity, path_error, Project};

pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ENTRIES: usize = 4000;
const MAX_METADATA_BYTES: usize = 512 * 1024;
const MAX_TRANSFERS: usize = 8;
type Result<T> = std::result::Result<T, ManagedErrorCode>;

struct Transfer {
    directory: PathBuf,
    target: Option<String>,
    expected_hash: String,
}
impl Drop for Transfer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.directory.join("content"));
        let _ = std::fs::remove_dir(&self.directory);
    }
}

/// Only the account service holding its exclusive lock may reap crash leftovers.
pub(super) fn clean_abandoned(directory: &Path) -> std::io::Result<()> {
    let directory = directory.join("file-transfers");
    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    identity::private_directory(&directory)?;
    for entry in entries {
        let entry = entry?;
        if entry
            .file_name()
            .to_str()
            .is_none_or(|name| Uuid::parse_str(name).is_err())
        {
            continue;
        }
        identity::private_directory(&entry.path())?;
        match std::fs::remove_file(entry.path().join("content")) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error),
        }
        std::fs::remove_dir(entry.path())?;
    }
    Ok(())
}

pub(super) struct Files {
    directory: PathBuf,
    transfers: HashMap<String, Transfer>,
}

impl Files {
    pub fn new(directory: &Path) -> Self {
        Self {
            directory: directory.join("file-transfers"),
            transfers: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.transfers.clear();
    }

    #[cfg(test)]
    pub(super) fn transfers_for_test(&self) -> Vec<String> {
        self.transfers.keys().cloned().collect()
    }

    pub fn execute(
        &mut self,
        request: ProjectFilesRequest,
        project: &Project,
    ) -> Result<ProjectFilesResult> {
        let action = ProjectFileAction::try_from(request.action)
            .map_err(|_| ManagedErrorCode::ManagedInvalidInput)?;
        let mut result = ProjectFilesResult {
            fence: request.fence.clone(),
            query_generation: request.query_generation,
            ..Default::default()
        };
        match action {
            ProjectFileAction::ProjectGitBranches => {
                result.git_output = git_output(
                    project,
                    &[
                        "for-each-ref",
                        "--format=%(refname:short)",
                        "refs/heads",
                        "refs/remotes",
                    ],
                )?;
            }
            ProjectFileAction::ProjectGitRoot => {
                result.git_output = git_output(project, &["rev-parse", "--show-toplevel"])?;
            }
            ProjectFileAction::ProjectGitStatus
            | ProjectFileAction::ProjectGitDiff
            | ProjectFileAction::ProjectGitPrepareBase => {
                if action == ProjectFileAction::ProjectGitStatus {
                    result.git_base = git_output(project, &["rev-parse", "--verify", "HEAD"])
                        .unwrap_or_default()
                        .trim()
                        .into();
                    if !request.destination.is_empty() {
                        let reference = if request.destination == "@main" {
                            ["main", "master", "origin/main", "origin/master"]
                                .into_iter()
                                .find(|name| {
                                    git_output(
                                        project,
                                        &["rev-parse", "--verify", &format!("{name}^{{commit}}")],
                                    )
                                    .is_ok()
                                })
                                .ok_or(ManagedErrorCode::ManagedUnavailable)?
                        } else {
                            &request.destination
                        };
                        if reference.len() > 256 || reference.chars().any(char::is_control) {
                            return Err(ManagedErrorCode::ManagedInvalidInput);
                        }
                        let resolved = git_output(
                            project,
                            &[
                                "rev-parse",
                                "--verify",
                                "--end-of-options",
                                &format!("{reference}^{{commit}}"),
                            ],
                        )?;
                        result.git_base =
                            git_output(project, &["merge-base", "HEAD", resolved.trim()])?
                                .trim()
                                .into();
                        result.git_name_status = git_output(
                            project,
                            &[
                                "diff",
                                "--no-ext-diff",
                                "--no-textconv",
                                "--name-status",
                                "-z",
                                &result.git_base,
                                "--",
                                ".",
                            ],
                        )?;
                    }
                    result.git_output = git_output(
                        project,
                        &[
                            "--no-optional-locks",
                            "status",
                            "--porcelain=2",
                            "-z",
                            "--untracked-files=all",
                            "--",
                            ".",
                        ],
                    )?;
                } else {
                    components(&request.path)?;
                    // Resolve a commit before using it as an argument or object selector.
                    let reference = if request.destination.is_empty() {
                        "HEAD"
                    } else {
                        &request.destination
                    };
                    if reference.len() > 256 || reference.chars().any(char::is_control) {
                        return Err(ManagedErrorCode::ManagedInvalidInput);
                    }
                    let commit = git_output(
                        project,
                        &[
                            "rev-parse",
                            "--verify",
                            "--end-of-options",
                            &format!("{reference}^{{commit}}"),
                        ],
                    )?;
                    let commit = commit.trim();
                    if !matches!(commit.len(), 40 | 64)
                        || !commit.bytes().all(|b| b.is_ascii_hexdigit())
                    {
                        return Err(ManagedErrorCode::ManagedInvalidInput);
                    }
                    if action == ProjectFileAction::ProjectGitDiff {
                        let before = match Directory::parent(project, &request.path)
                            .and_then(|(parent, name)| parent.open_file(&name))
                            .and_then(|mut file| digest(&mut file))
                        {
                            Ok(hash) => Some(hash),
                            Err(ManagedErrorCode::ManagedNotFound) => None,
                            Err(error) => return Err(error),
                        };
                        let mut arguments = vec![
                            "--no-optional-locks",
                            "diff",
                            "--no-ext-diff",
                            "--no-textconv",
                            "--no-color",
                            commit,
                            "--",
                            &request.path,
                        ];
                        if !request.git_previous_path.is_empty() {
                            components(&request.git_previous_path)?;
                            arguments.push(&request.git_previous_path);
                        }
                        result.git_output = git_output(project, &arguments)?;
                        let after = match Directory::parent(project, &request.path)
                            .and_then(|(parent, name)| parent.open_file(&name))
                            .and_then(|mut file| digest(&mut file))
                        {
                            Ok(hash) => Some(hash),
                            Err(ManagedErrorCode::ManagedNotFound) => None,
                            Err(error) => return Err(error),
                        };
                        if before != after {
                            return Err(ManagedErrorCode::ManagedConflict);
                        }
                        result.sha256 = after.unwrap_or_default();
                    } else {
                        if self.transfers.len() >= MAX_TRANSFERS {
                            return Err(ManagedErrorCode::ManagedCapacityExceeded);
                        }
                        let bytes = git_bytes(
                            project,
                            &[
                                "show",
                                "--no-textconv",
                                &format!("{commit}:{}", request.path),
                            ],
                            MAX_FILE_BYTES as usize,
                        )?;
                        identity::private_directory(&self.directory).map_err(path_error)?;
                        let id = Uuid::new_v4().to_string();
                        let directory = self.directory.join(&id);
                        identity::private_directory(&directory).map_err(path_error)?;
                        let transfer = Transfer {
                            directory,
                            target: None,
                            expected_hash: String::new(),
                        };
                        let path = transfer.directory.join("content");
                        use std::io::Write;
                        let mut file = identity::private_file(&path).map_err(path_error)?;
                        file.write_all(&bytes).map_err(path_error)?;
                        file.sync_all().map_err(path_error)?;
                        result.transfer_path = path
                            .to_str()
                            .ok_or(ManagedErrorCode::ManagedInvalidInput)?
                            .into();
                        result.transfer_id = id.clone();
                        result.sha256 = format!("{:x}", Sha256::digest(&bytes));
                        result.size = bytes.len() as u64;
                        self.transfers.insert(id, transfer);
                    }
                }
            }
            ProjectFileAction::ProjectFileList => {
                let dir = Directory::at(project, &request.path)?;
                let entries = dir.entries()?;
                let parent = project
                    .root
                    .join(&request.path)
                    .to_string_lossy()
                    .into_owned();
                // Leave room for protobuf envelopes under the managed message limit.
                let size = entries.iter().try_fold(parent.len(), |size, (name, _)| {
                    size.checked_add(parent.len() + name.len() * 2 + 128)
                });
                if size.is_none_or(|size| size > MAX_METADATA_BYTES) {
                    return Err(ManagedErrorCode::ManagedCapacityExceeded);
                }
                result.snapshot = Some(RepoMetadataSnapshot {
                    repo_path: project.root.to_str().unwrap().into(),
                    sync_complete: true,
                    entries: vec![RepoMetadataEntryUpdate {
                        parent_path_to_replace: parent.clone(),
                        subtree_metadata: entries
                            .into_iter()
                            .map(|(name, directory)| {
                                let path = project
                                    .root
                                    .join(&request.path)
                                    .join(&name)
                                    .to_string_lossy()
                                    .into_owned();
                                RepoNodeMetadata {
                                    node: Some(if directory {
                                        repo_node_metadata::Node::Directory(DirectoryNodeMetadata {
                                            path,
                                            ignored: false,
                                            loaded: false,
                                        })
                                    } else {
                                        repo_node_metadata::Node::File(FileNodeMetadata {
                                            extension: Path::new(&name)
                                                .extension()
                                                .and_then(|v| v.to_str())
                                                .map(str::to_owned),
                                            path,
                                            ignored: false,
                                        })
                                    }),
                                }
                            })
                            .collect(),
                    }],
                });
            }
            ProjectFileAction::ProjectFilePrepareRead
            | ProjectFileAction::ProjectFilePrepareWrite => {
                if self.transfers.len() >= MAX_TRANSFERS {
                    return Err(ManagedErrorCode::ManagedCapacityExceeded);
                }
                let (parent, name) = Directory::parent(project, &request.path)?;
                let mut source = parent.open_file(&name)?;
                let writing = action == ProjectFileAction::ProjectFilePrepareWrite;
                let hash = digest(&mut source)?;
                if writing && request.expected_hash != hash {
                    return Err(ManagedErrorCode::ManagedConflict);
                }
                identity::private_directory(&self.directory).map_err(path_error)?;
                let id = Uuid::new_v4().to_string();
                let directory = self.directory.join(&id);
                identity::private_directory(&directory).map_err(path_error)?;
                let transfer = Transfer {
                    directory,
                    target: writing.then(|| request.path.clone()),
                    expected_hash: hash.clone(),
                };
                let path = transfer.directory.join("content");
                let mut staged = identity::private_file(&path).map_err(path_error)?;
                if !writing {
                    use std::io::{Seek, SeekFrom};
                    source.seek(SeekFrom::Start(0)).map_err(path_error)?;
                    let size = std::io::copy(&mut source.take(MAX_FILE_BYTES + 1), &mut staged)
                        .map_err(path_error)?;
                    if size > MAX_FILE_BYTES {
                        return Err(ManagedErrorCode::ManagedCapacityExceeded);
                    }
                    staged.sync_all().map_err(path_error)?;
                    staged.seek(SeekFrom::Start(0)).map_err(path_error)?;
                    // A source modified during staging must not produce a mismatched snapshot.
                    if digest(&mut staged)? != hash {
                        return Err(ManagedErrorCode::ManagedConflict);
                    }
                    result.size = size;
                }
                result.sha256 = hash;
                result.transfer_path = path
                    .to_str()
                    .ok_or(ManagedErrorCode::ManagedInvalidInput)?
                    .into();
                result.transfer_id = id.clone();
                self.transfers.insert(id, transfer);
            }
            ProjectFileAction::ProjectFileRelease => {
                self.transfers
                    .remove(&request.transfer_id)
                    .ok_or(ManagedErrorCode::ManagedInvalidInput)?;
            }
            ProjectFileAction::ProjectFileCommitWrite => {
                let transfer = self
                    .transfers
                    .get(&request.transfer_id)
                    .ok_or(ManagedErrorCode::ManagedInvalidInput)?;
                let target = transfer
                    .target
                    .as_ref()
                    .ok_or(ManagedErrorCode::ManagedInvalidInput)?;
                if *target != request.path {
                    return Err(ManagedErrorCode::ManagedInvalidInput);
                }
                let (parent, name) = Directory::parent(project, target)?;
                let mut original = parent.open_file(&name)?;
                if digest(&mut original)? != transfer.expected_hash {
                    return Err(ManagedErrorCode::ManagedConflict);
                }
                let mut staged = identity::private_file(&transfer.directory.join("content"))
                    .map_err(path_error)?;
                result.sha256 = digest(&mut staged)?;
                if result.sha256 != request.expected_hash {
                    return Err(ManagedErrorCode::ManagedConflict);
                }
                result.size = staged.metadata().map_err(path_error)?.len();
                let permissions = original.metadata().map_err(path_error)?.permissions();
                drop(original);
                parent.replace(&name, &mut staged, permissions, &result.sha256)?;
                self.transfers.remove(&request.transfer_id);
            }
            ProjectFileAction::ProjectFileCreate | ProjectFileAction::ProjectDirectoryCreate => {
                let (parent, name) = Directory::parent(project, &request.path)?;
                parent.create(&name, action == ProjectFileAction::ProjectDirectoryCreate)?;
            }
            ProjectFileAction::ProjectFileRename => {
                let (parent, name) = Directory::parent(project, &request.path)?;
                let (destination, new_name) = Directory::parent(project, &request.destination)?;
                parent.rename(&name, &destination, &new_name)?;
            }
            ProjectFileAction::ProjectFileDelete => {
                let (parent, name) = Directory::parent(project, &request.path)?;
                // Check the known subtree budget before making destructive changes.
                let mut remaining = MAX_ENTRIES;
                parent.check_remove(&name, &mut remaining, 0)?;
                remaining = MAX_ENTRIES;
                parent.remove(&name, &mut remaining, 0)?;
            }
            ProjectFileAction::Unspecified => return Err(ManagedErrorCode::ManagedInvalidInput),
        }
        Ok(result)
    }
}

/// Bound subprocess lifetime and output before returning metadata over the control wire.
fn git_output(project: &Project, arguments: &[&str]) -> Result<String> {
    String::from_utf8(git_bytes(project, arguments, MAX_METADATA_BYTES)?)
        .map_err(|_| ManagedErrorCode::ManagedInvalidInput)
}

/// Share the bounded private Git runner with worktree identity discovery.
pub(crate) fn git_metadata(root: &Path, arguments: &[&str]) -> Result<String> {
    let project = Project { root: root.to_owned(), handle: super::open_root(root).map_err(path_error)? };
    let result = git_output(&project, arguments);
    #[cfg(debug_assertions)]
    if let Err(error) = result {
        if let Some(directory) = std::env::var_os("WARP_INTEGRATION_TEST_ARTIFACTS_DIR") {
            let _ = std::fs::write(Path::new(&directory).join("checkpoint-private-git.txt"), format!("Native private Git error: code={}", error as i32));
        }
    }
    result
}

fn git_bytes(project: &Project, arguments: &[&str], limit: usize) -> Result<Vec<u8>> {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    if !super::root_matches(&project.handle, &project.root).map_err(path_error)? {
        return Err(ManagedErrorCode::ManagedStaleAttachment);
    }
    let git = crate::installation::git_executable().map_err(path_error)?;
    let mut command = Command::new(&git);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Background metadata reads must not allocate a GUI process's console.
        command.creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
    }
    // Inherited Git overrides must not select a different repository than the admitted cwd.
    for name in ["GIT_DIR", "GIT_WORK_TREE", "GIT_COMMON_DIR", "GIT_INDEX_FILE", "GIT_OBJECT_DIRECTORY", "GIT_ALTERNATE_OBJECT_DIRECTORIES", "GIT_CONFIG", "GIT_CONFIG_COUNT", "GIT_CONFIG_PARAMETERS"] {
        command.env_remove(name);
    }
    if git.is_absolute() {
        // Git may re-exec itself for submodules; keep the private runtime on this child's PATH only.
        let mut paths = vec![git.parent().unwrap().to_path_buf()];
        if let Some(path) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&path));
        }
        command.env("PATH", std::env::join_paths(paths).map_err(|_| ManagedErrorCode::ManagedUnavailable)?);
    }
    let mut child = command
        .args(["-c", "core.fsmonitor=false", "-c", "core.hooksPath="])
        .args(arguments)
        .current_dir(&project.root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_LITERAL_PATHSPECS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ManagedErrorCode::ManagedUnavailable)?;
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut output = Vec::new();
        let result = stdout.take((limit + 1) as u64).read_to_end(&mut output);
        let _ = sender.send(result.map(|_| output));
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    #[cfg(debug_assertions)]
    let (mut diagnostic_stage, mut diagnostic_exit) = (1, -1);
    let result = (|| {
        let output = receiver
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| ManagedErrorCode::ManagedUnavailable)?
            .map_err(path_error)?;
        if output.len() > limit {
            return Err(ManagedErrorCode::ManagedCapacityExceeded);
        }
        #[cfg(debug_assertions)]
        { diagnostic_stage = 2; }
        loop {
            if let Some(status) = child.try_wait().map_err(path_error)? {
                #[cfg(debug_assertions)]
                { diagnostic_exit = status.code().unwrap_or(-1); }
                if !status.success() {
                    return Err(ManagedErrorCode::ManagedUnavailable);
                }
                break;
            }
            if Instant::now() >= deadline {
                return Err(ManagedErrorCode::ManagedUnavailable);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        #[cfg(debug_assertions)]
        { diagnostic_stage = 3; }
        if !super::root_matches(&project.handle, &project.root).map_err(path_error)? {
            return Err(ManagedErrorCode::ManagedStaleAttachment);
        }
        Ok(output)
    })();
    if result.is_err() {
        #[cfg(debug_assertions)]
        { diagnostic_exit = child.try_wait().ok().flatten().and_then(|status| status.code()).unwrap_or(diagnostic_exit); }
        #[cfg(debug_assertions)]
        if let Some(directory) = std::env::var_os("WARP_INTEGRATION_TEST_ARTIFACTS_DIR") {
            let _ = std::fs::write(Path::new(&directory).join("checkpoint-private-git-runner.txt"), format!("Native private Git runner: stage={diagnostic_stage}, exit={diagnostic_exit}"));
        }
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

fn components(path: &str) -> Result<Vec<&std::ffi::OsStr>> {
    if path.len() > 4096
        || path.chars().any(char::is_control)
        || path
            .split(|c| c == '/' || (cfg!(windows) && c == '\\'))
            .any(|part| matches!(part, "." | ".."))
    {
        return Err(ManagedErrorCode::ManagedInvalidInput);
    }
    Path::new(path)
        .components()
        .map(|c| match c {
            Component::Normal(name)
                if !cfg!(windows) || name.to_str().is_some_and(windows_name) =>
            {
                Ok(name)
            }
            _ => Err(ManagedErrorCode::ManagedInvalidInput),
        })
        .collect()
}

fn windows_name(name: &str) -> bool {
    // Prevent alternate data streams, Win32 trimming aliases and device names.
    if name.ends_with(['.', ' ']) || name.contains([':', '<', '>', '"', '|', '?', '*']) {
        return false;
    }
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    !matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) && !(stem.starts_with("COM") || stem.starts_with("LPT"))
        .then(|| &stem[3..])
        .is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn win32_aliases_devices_and_streams_are_not_project_files() {
        for invalid in [
            "CON",
            "nul.txt",
            "COM1.log",
            "LPT²",
            "CONIN$",
            "one:stream",
            "trailing.",
            "trailing ",
            "glob*",
        ] {
            assert!(!windows_name(invalid), "{invalid}");
        }
        for valid in [
            "console.txt",
            "COM10",
            "文件.txt",
            "two words.md",
            "-literal",
            "LPT0",
        ] {
            assert!(windows_name(valid), "{valid}");
        }
        for path in ["one/./two", "one/../two", "/absolute", ".."] {
            assert!(components(path).is_err());
        }
    }
}

fn digest(file: &mut File) -> Result<String> {
    use std::io::{Seek, SeekFrom};
    let metadata = file.metadata().map_err(path_error)?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err(ManagedErrorCode::ManagedCapacityExceeded);
    }
    file.seek(SeekFrom::Start(0)).map_err(path_error)?;
    let mut digest = Sha256::new();
    let mut count = 0u64;
    let mut buffer = [0; 8192];
    loop {
        let n = file.read(&mut buffer).map_err(path_error)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > MAX_FILE_BYTES {
            return Err(ManagedErrorCode::ManagedCapacityExceeded);
        }
        digest.update(&buffer[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

/// Pins every traversed directory; all Unix mutations use descriptor-relative syscalls.
struct Directory {
    file: File,
    #[cfg(windows)]
    path: PathBuf,
    #[cfg(windows)]
    _parents: Vec<File>,
}

impl Directory {
    fn at(project: &Project, path: &str) -> Result<Self> {
        let parts = components(path)?;
        #[cfg(unix)]
        {
            let mut file = project.handle.try_clone().map_err(path_error)?;
            for part in parts {
                file = unix::open(&file, part, true, false)?;
            }
            Ok(Self { file })
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
            let mut current = project.root.clone();
            let mut parents = Vec::new();
            for part in std::iter::once(None).chain(parts.into_iter().map(Some)) {
                if let Some(part) = part {
                    current.push(part);
                }
                let file = OpenOptions::new()
                    .read(true)
                    // Protect this directory from replacement while permitting child writes.
                    .share_mode(3)
                    .custom_flags(0x02000000 | 0x00200000)
                    .open(&current)
                    .map_err(path_error)?;
                let metadata = file.metadata().map_err(path_error)?;
                if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
                    return Err(ManagedErrorCode::ManagedInvalidInput);
                }
                parents.push(file);
            }
            if !super::root_matches(&project.handle, &project.root).map_err(path_error)? {
                return Err(ManagedErrorCode::ManagedStaleAttachment);
            }
            Ok(Self {
                file: parents.pop().unwrap(),
                path: current,
                _parents: parents,
            })
        }
    }
    fn parent(project: &Project, path: &str) -> Result<(Self, String)> {
        let parts = components(path)?;
        let name = parts
            .last()
            .and_then(|p| p.to_str())
            .ok_or(ManagedErrorCode::ManagedInvalidInput)?
            .to_owned();
        let parent = Path::new(path).parent().unwrap_or(Path::new(""));
        Ok((
            Self::at(
                project,
                parent
                    .to_str()
                    .ok_or(ManagedErrorCode::ManagedInvalidInput)?,
            )?,
            name,
        ))
    }
    fn entries(&self) -> Result<Vec<(String, bool)>> {
        #[cfg(unix)]
        let mut entries = unix::entries(&self.file)?;
        #[cfg(windows)]
        let mut entries = {
            use std::os::windows::fs::MetadataExt;
            let mut entries = Vec::new();
            for entry in std::fs::read_dir(&self.path).map_err(path_error)? {
                let entry = entry.map_err(path_error)?;
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| ManagedErrorCode::ManagedInvalidInput)?;
                let metadata = std::fs::symlink_metadata(entry.path()).map_err(path_error)?;
                entries.push((
                    name,
                    metadata.is_dir() && metadata.file_attributes() & 0x400 == 0,
                ));
                if entries.len() > MAX_ENTRIES {
                    return Err(ManagedErrorCode::ManagedCapacityExceeded);
                }
            }
            entries
        };
        if entries
            .iter()
            .any(|(name, _)| name.chars().any(char::is_control))
        {
            return Err(ManagedErrorCode::ManagedInvalidInput);
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(entries)
    }
    fn open_file(&self, name: &str) -> Result<File> {
        #[cfg(unix)]
        let file = unix::open(&self.file, name.as_ref(), false, false)?;
        #[cfg(windows)]
        let file = {
            use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(0x00200000)
                .open(self.path.join(name))
                .map_err(path_error)?;
            if file.metadata().map_err(path_error)?.file_attributes() & 0x400 != 0 {
                return Err(ManagedErrorCode::ManagedInvalidInput);
            }
            file
        };
        if !file.metadata().map_err(path_error)?.is_file() {
            return Err(ManagedErrorCode::ManagedInvalidInput);
        }
        Ok(file)
    }
    fn create(&self, name: &str, directory: bool) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            if directory {
                let name = unix::name(name.as_ref())?;
                unix::check(unsafe { libc::mkdirat(self.file.as_raw_fd(), name.as_ptr(), 0o755) })?;
            } else {
                unix::open(&self.file, name.as_ref(), false, true)?;
            }
        }
        #[cfg(windows)]
        {
            if directory {
                std::fs::create_dir(self.path.join(name)).map_err(path_error)?;
            } else {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(self.path.join(name))
                    .map_err(path_error)?;
            }
        }
        Ok(())
    }
    fn rename(&self, name: &str, destination: &Directory, new_name: &str) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            let name = unix::name(name.as_ref())?;
            let new_name = unix::name(new_name.as_ref())?;
            #[cfg(target_os = "linux")]
            let result = unsafe {
                libc::renameat2(
                    self.file.as_raw_fd(),
                    name.as_ptr(),
                    destination.file.as_raw_fd(),
                    new_name.as_ptr(),
                    libc::RENAME_NOREPLACE,
                )
            };
            #[cfg(target_os = "macos")]
            let result = unsafe {
                libc::renameatx_np(
                    self.file.as_raw_fd(),
                    name.as_ptr(),
                    destination.file.as_raw_fd(),
                    new_name.as_ptr(),
                    libc::RENAME_EXCL,
                )
            };
            unix::check(result)?;
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows::{
                core::PCWSTR,
                Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH},
            };
            let from: Vec<u16> = self
                .path
                .join(name)
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect();
            let to: Vec<u16> = destination
                .path
                .join(new_name)
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect();
            // std::fs::rename may replace an existing file on Windows.
            unsafe {
                MoveFileExW(
                    PCWSTR(from.as_ptr()),
                    PCWSTR(to.as_ptr()),
                    MOVEFILE_WRITE_THROUGH,
                )
            }
            .map_err(|_| ManagedErrorCode::ManagedConflict)?;
        }
        Ok(())
    }
    fn replace(
        &self,
        name: &str,
        staged: &mut File,
        permissions: std::fs::Permissions,
        expected_hash: &str,
    ) -> Result<()> {
        use std::io::{Seek, SeekFrom};
        let temporary = format!(".warpai-save-{}", Uuid::new_v4());
        #[cfg(unix)]
        let mut target = unix::open(&self.file, temporary.as_ref(), false, true)?;
        #[cfg(windows)]
        let mut target = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(self.path.join(&temporary))
            .map_err(path_error)?;
        let result = (|| {
            staged.seek(SeekFrom::Start(0)).map_err(path_error)?;
            let size = std::io::copy(&mut staged.take(MAX_FILE_BYTES + 1), &mut target)
                .map_err(path_error)?;
            if size > MAX_FILE_BYTES {
                return Err(ManagedErrorCode::ManagedCapacityExceeded);
            }
            if digest(&mut target)? != expected_hash {
                return Err(ManagedErrorCode::ManagedConflict);
            }
            target.set_permissions(permissions).map_err(path_error)?;
            target.sync_all().map_err(path_error)?;
            drop(target);
            #[cfg(unix)]
            {
                use std::os::fd::AsRawFd;
                let from = unix::name(temporary.as_ref())?;
                let to = unix::name(name.as_ref())?;
                unix::check(unsafe {
                    libc::renameat(
                        self.file.as_raw_fd(),
                        from.as_ptr(),
                        self.file.as_raw_fd(),
                        to.as_ptr(),
                    )
                })?;
            }
            #[cfg(windows)]
            {
                use std::os::windows::ffi::OsStrExt;
                use windows::{
                    core::PCWSTR,
                    Win32::Storage::FileSystem::{
                        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
                    },
                };
                let from: Vec<u16> = self
                    .path
                    .join(&temporary)
                    .as_os_str()
                    .encode_wide()
                    .chain(Some(0))
                    .collect();
                let to: Vec<u16> = self
                    .path
                    .join(name)
                    .as_os_str()
                    .encode_wide()
                    .chain(Some(0))
                    .collect();
                unsafe {
                    MoveFileExW(
                        PCWSTR(from.as_ptr()),
                        PCWSTR(to.as_ptr()),
                        MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                    )
                }
                .map_err(|_| ManagedErrorCode::ManagedUnavailable)?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = self.remove(&temporary, &mut 1, 0);
        }
        result
    }
    fn check_remove(&self, name: &str, remaining: &mut usize, depth: usize) -> Result<()> {
        if *remaining == 0 || depth > 64 {
            return Err(ManagedErrorCode::ManagedCapacityExceeded);
        }
        *remaining -= 1;
        #[cfg(unix)]
        let child = unix::open(&self.file, name.as_ref(), true, false)
            .ok()
            .map(|file| Directory { file });
        #[cfg(windows)]
        let child = {
            use std::os::windows::fs::MetadataExt;
            let metadata = std::fs::symlink_metadata(self.path.join(name)).map_err(path_error)?;
            if metadata.is_dir() && metadata.file_attributes() & 0x400 == 0 {
                let project = Project {
                    root: self.path.clone(),
                    handle: self.file.try_clone().map_err(path_error)?,
                };
                Some(Directory::at(&project, name)?)
            } else {
                None
            }
        };
        if let Some(child) = child {
            for (name, _) in child.entries()? {
                child.check_remove(&name, remaining, depth + 1)?;
            }
        }
        Ok(())
    }
    fn remove(&self, name: &str, remaining: &mut usize, depth: usize) -> Result<()> {
        if *remaining == 0 || depth > 64 {
            return Err(ManagedErrorCode::ManagedCapacityExceeded);
        }
        *remaining -= 1;
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            let directory = unix::open(&self.file, name.as_ref(), true, false).ok();
            if let Some(file) = directory {
                let child = Directory { file };
                for (name, _) in child.entries()? {
                    child.remove(&name, remaining, depth + 1)?;
                }
                let name = unix::name(name.as_ref())?;
                unix::check(unsafe {
                    libc::unlinkat(self.file.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR)
                })?;
            } else {
                let name = unix::name(name.as_ref())?;
                unix::check(unsafe { libc::unlinkat(self.file.as_raw_fd(), name.as_ptr(), 0) })?;
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            let path = self.path.join(name);
            let metadata = std::fs::symlink_metadata(&path).map_err(path_error)?;
            if metadata.is_dir() && metadata.file_attributes() & 0x400 == 0 {
                // Each recursive directory is re-opened without following reparse points.
                let project = Project {
                    root: self.path.clone(),
                    handle: self.file.try_clone().map_err(path_error)?,
                };
                let child = Directory::at(&project, name)?;
                for (name, _) in child.entries()? {
                    child.remove(&name, remaining, depth + 1)?;
                }
                drop(child);
                std::fs::remove_dir(path).map_err(path_error)?;
            } else if metadata.file_attributes() & 0x10 != 0 {
                std::fs::remove_dir(path).map_err(path_error)?;
            } else {
                std::fs::remove_file(path).map_err(path_error)?;
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::{
        ffi::{CStr, CString, OsStr},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::ffi::OsStrExt,
        },
    };
    pub fn name(name: &OsStr) -> Result<CString> {
        CString::new(name.as_bytes()).map_err(|_| ManagedErrorCode::ManagedInvalidInput)
    }
    pub fn check(result: i32) -> Result<()> {
        if result < 0 {
            let error = std::io::Error::last_os_error();
            Err(if error.kind() == std::io::ErrorKind::AlreadyExists {
                ManagedErrorCode::ManagedConflict
            } else {
                path_error(error)
            })
        } else {
            Ok(())
        }
    }
    pub fn open(parent: &File, leaf: &OsStr, directory: bool, create: bool) -> Result<File> {
        let leaf = name(leaf)?;
        let flags = libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | if directory {
                libc::O_RDONLY | libc::O_DIRECTORY
            } else if create {
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL
            } else {
                libc::O_RDONLY
            };
        let fd = unsafe { libc::openat(parent.as_raw_fd(), leaf.as_ptr(), flags, 0o600) };
        check(fd)?;
        Ok(unsafe { File::from_raw_fd(fd) })
    }
    pub fn entries(file: &File) -> Result<Vec<(String, bool)>> {
        // dup shares directory offsets; openat(".") creates an independent enumeration.
        let fresh = open(file, OsStr::new("."), true, false)?;
        use std::os::fd::IntoRawFd;
        let fd = fresh.into_raw_fd();
        let dir = unsafe { libc::fdopendir(fd) };
        if dir.is_null() {
            unsafe {
                libc::close(fd);
            }
            return Err(path_error(std::io::Error::last_os_error()));
        }
        struct OwnedDir(*mut libc::DIR);
        impl Drop for OwnedDir {
            fn drop(&mut self) {
                unsafe {
                    libc::closedir(self.0);
                }
            }
        }
        let _owned = OwnedDir(dir);
        let mut entries = Vec::new();
        loop {
            // readdir returns null for both EOF and errors; never publish partial data.
            #[cfg(target_os = "macos")]
            let errno = unsafe { libc::__error() };
            #[cfg(target_os = "linux")]
            let errno = unsafe { libc::__errno_location() };
            unsafe {
                *errno = 0;
            }
            let entry = unsafe { libc::readdir(dir) };
            if entry.is_null() {
                if unsafe { *errno } != 0 {
                    return Err(path_error(std::io::Error::last_os_error()));
                }
                break;
            }
            let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
            if bytes.to_bytes() == b"." || bytes.to_bytes() == b".." {
                continue;
            }
            let name = bytes
                .to_str()
                .map_err(|_| ManagedErrorCode::ManagedInvalidInput)?
                .to_owned();
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            check(unsafe {
                libc::fstatat(
                    file.as_raw_fd(),
                    bytes.as_ptr(),
                    stat.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            })?;
            let stat = unsafe { stat.assume_init() };
            entries.push((name, stat.st_mode & libc::S_IFMT == libc::S_IFDIR));
            if entries.len() > MAX_ENTRIES {
                return Err(ManagedErrorCode::ManagedCapacityExceeded);
            }
        }
        Ok(entries)
    }
}
