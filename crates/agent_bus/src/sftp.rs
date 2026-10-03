//! Bounded SFTP v3 operations over native SSH, without shell filenames.
use sha2::{Digest, Sha256};
use std::{process::Stdio, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, ChildStdin, ChildStdout},
};
use uuid::Uuid;

use crate::ssh_remote::{RemoteShell, SshProfile};

const MAX_PACKET: usize = 1024 * 1024;
const MAX_FILE: usize = 16 * 1024 * 1024;
const MAX_ENTRIES: usize = 2000;
const CHUNK: u32 = 32768;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SftpError {
    InvalidName,
    Protocol,
    ConnectionLost,
    Unavailable,
    Missing,
    PermissionDenied,
    Unsupported,
    PathEscape,
    CapacityExceeded,
    EndOfFile,
    Conflict,
    CommitUnknown,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Attributes {
    pub size: Option<u64>,
    pub permissions: Option<u32>,
    pub modified_unix_seconds: Option<u32>,
}

impl Attributes {
    pub fn is_directory(&self) -> bool {
        self.permissions.is_some_and(|p| p & 0o170000 == 0o040000)
    }
    pub fn is_symlink(&self) -> bool {
        self.permissions.is_some_and(|p| p & 0o170000 == 0o120000)
    }
    pub fn is_regular_file(&self) -> bool {
        self.permissions.is_some_and(|p| p & 0o170000 == 0o100000)
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub attributes: Attributes,
}

pub struct Directory {
    pub entries: Vec<Entry>,
    pub truncated: bool,
}

/// Original new-file intent; no file contents or credentials are retained.
#[derive(Clone, Debug)]
pub struct UploadReceipt {
    pub profile_id: Uuid,
    pub canonical_root: String,
    pub destination: String,
    pub partial: String,
    pub size: u64,
    pub sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UploadState {
    Prepared,
    Confirmed,
    Conflict,
    Unknown,
}

struct Cursor<'a>(&'a [u8]);
impl<'a> Cursor<'a> {
    fn bytes(&mut self, len: usize) -> Result<&'a [u8], SftpError> {
        if len > self.0.len() {
            return Err(SftpError::Protocol);
        }
        let (value, rest) = self.0.split_at(len);
        self.0 = rest;
        Ok(value)
    }
    fn u32(&mut self) -> Result<u32, SftpError> {
        Ok(u32::from_be_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, SftpError> {
        Ok(u64::from_be_bytes(self.bytes(8)?.try_into().unwrap()))
    }
    fn string(&mut self) -> Result<&'a [u8], SftpError> {
        let len = self.u32()? as usize;
        self.bytes(len)
    }
    fn finish(&self) -> Result<(), SftpError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(SftpError::Protocol)
        }
    }
    fn attributes(&mut self) -> Result<Attributes, SftpError> {
        let flags = self.u32()?;
        if flags & !0x8000000f != 0 {
            return Err(SftpError::Protocol);
        }
        let size = if flags & 1 != 0 {
            Some(self.u64()?)
        } else {
            None
        };
        if flags & 2 != 0 {
            self.u32()?;
            self.u32()?;
        }
        let permissions = if flags & 4 != 0 {
            Some(self.u32()?)
        } else {
            None
        };
        let modified_unix_seconds = if flags & 8 != 0 {
            self.u32()?;
            Some(self.u32()?)
        } else {
            None
        };
        if flags & 0x80000000 != 0 {
            let count = self.u32()?;
            if count > 128 {
                return Err(SftpError::Protocol);
            }
            for _ in 0..count {
                self.string()?;
                self.string()?;
            }
        }
        Ok(Attributes {
            size,
            permissions,
            modified_unix_seconds,
        })
    }
}

fn string(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(&(value.len() as u32).to_be_bytes());
    output.extend_from_slice(value);
}

fn name(bytes: &[u8]) -> Result<String, SftpError> {
    let value = std::str::from_utf8(bytes).map_err(|_| SftpError::InvalidName)?;
    if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control) {
        return Err(SftpError::InvalidName);
    }
    Ok(value.into())
}

fn status(cursor: &mut Cursor<'_>) -> Result<(), SftpError> {
    let code = cursor.u32()?;
    // Ignore peer error prose; only the standards-defined code can reach the UI.
    cursor.string()?;
    cursor.string()?;
    cursor.finish()?;
    match code {
        0 => Ok(()),
        1 => Err(SftpError::EndOfFile),
        2 => Err(SftpError::Missing),
        3 => Err(SftpError::PermissionDenied),
        8 => Err(SftpError::Unsupported),
        _ => Err(SftpError::Unavailable),
    }
}

pub struct SftpClient {
    child: Child,
    input: ChildStdin,
    output: ChildStdout,
    pub connection_id: Uuid,
    pub canonical_root: String,
    profile_id: Uuid,
    windows: bool,
    sequence: u32,
    alive: bool,
}

impl SftpClient {
    pub async fn connect(profile: &SshProfile) -> Result<Self, SftpError> {
        let mut command = profile
            .ssh_command(true)
            .map_err(|_| SftpError::Unavailable)?;
        command
            .arg("-s")
            .arg("--")
            .arg(&profile.target)
            .arg("sftp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command.spawn().map_err(|_| SftpError::Unavailable)?;
        let input = child.stdin.take().ok_or(SftpError::ConnectionLost)?;
        let output = child.stdout.take().ok_or(SftpError::ConnectionLost)?;
        let mut client = Self {
            child,
            input,
            output,
            connection_id: Uuid::new_v4(),
            canonical_root: String::new(),
            profile_id: profile.id,
            windows: matches!(profile.remote_shell, RemoteShell::PowerShell),
            sequence: 0,
            alive: true,
        };
        let handshake = tokio::time::timeout(Duration::from_secs(10), async {
            client.write_packet(&[1, 0, 0, 0, 3]).await?;
            client.read_packet().await
        })
        .await
        .map_err(|_| SftpError::ConnectionLost)??;
        if handshake.first() != Some(&2) {
            return Err(SftpError::Protocol);
        }
        let mut cursor = Cursor(&handshake[1..]);
        if cursor.u32()? != 3 {
            return Err(SftpError::Unsupported);
        }
        let mut extensions = 0;
        while !cursor.0.is_empty() {
            if extensions == 128 {
                return Err(SftpError::Protocol);
            }
            cursor.string()?;
            cursor.string()?;
            extensions += 1;
        }
        client.canonical_root = client.realpath(&profile.remote_root).await?;
        let attrs = client.stat_path(&client.canonical_root.clone()).await?;
        if !attrs.is_directory() {
            return Err(SftpError::Unsupported);
        }
        Ok(client)
    }

    fn relative_path(&self, relative: &str) -> Result<String, SftpError> {
        if relative.len() > 4096
            || relative.chars().any(char::is_control)
            || relative.starts_with('/')
            || relative.split('/').any(|part| matches!(part, "." | ".."))
            || (self.windows && (relative.contains('\\') || relative.contains(':')))
        {
            return Err(SftpError::InvalidName);
        }
        if relative.is_empty() {
            return Ok(self.canonical_root.clone());
        }
        Ok(format!(
            "{}/{}",
            self.canonical_root.trim_end_matches('/'),
            relative
        ))
    }

    async fn scoped_path(&mut self, relative: &str) -> Result<String, SftpError> {
        let path = self.relative_path(relative)?;
        let canonical = self.realpath(&path).await?;
        let root = self.canonical_root.trim_end_matches('/');
        if canonical != self.canonical_root && !canonical.starts_with(&format!("{root}/")) {
            return Err(SftpError::PathEscape);
        }
        Ok(canonical)
    }

    async fn realpath(&mut self, path: &str) -> Result<String, SftpError> {
        let mut payload = Vec::new();
        string(&mut payload, path.as_bytes());
        let reply = self.request(16, payload).await?;
        let mut cursor = Cursor(&reply[1..]);
        if reply[0] != 104 || cursor.u32()? != 1 {
            return Err(SftpError::Protocol);
        }
        let path = name(cursor.string()?)?;
        if !path.starts_with('/') {
            return Err(SftpError::Unsupported);
        }
        cursor.string()?;
        cursor.attributes()?;
        cursor.finish()?;
        Ok(path)
    }

    async fn stat_path(&mut self, path: &str) -> Result<Attributes, SftpError> {
        let mut payload = Vec::new();
        string(&mut payload, path.as_bytes());
        let reply = self.request(7, payload).await?;
        if reply[0] != 105 {
            return Err(SftpError::Protocol);
        }
        let mut cursor = Cursor(&reply[1..]);
        let attributes = cursor.attributes()?;
        cursor.finish()?;
        Ok(attributes)
    }

    pub async fn stat(&mut self, relative: &str) -> Result<Attributes, SftpError> {
        let path = self.scoped_path(relative).await?;
        self.stat_path(&path).await
    }

    async fn handle(&mut self, kind: u8, path: &str) -> Result<Vec<u8>, SftpError> {
        let mut payload = Vec::new();
        string(&mut payload, path.as_bytes());
        if kind == 3 {
            payload.extend_from_slice(&1u32.to_be_bytes());
            payload.extend_from_slice(&0u32.to_be_bytes());
        }
        let reply = self.request(kind, payload).await?;
        if reply[0] != 102 {
            return Err(SftpError::Protocol);
        }
        let mut cursor = Cursor(&reply[1..]);
        let handle = cursor.string()?.to_vec();
        cursor.finish()?;
        if handle.is_empty() || handle.len() > 256 {
            return Err(SftpError::Protocol);
        }
        Ok(handle)
    }

    async fn close_handle(&mut self, handle: &[u8]) -> Result<(), SftpError> {
        let mut payload = Vec::new();
        string(&mut payload, handle);
        let reply = self.request(4, payload).await?;
        if reply == [101] {
            Ok(())
        } else {
            Err(SftpError::Protocol)
        }
    }

    pub async fn list(&mut self, relative: &str) -> Result<Directory, SftpError> {
        let path = self.scoped_path(relative).await?;
        let handle = self.handle(11, &path).await?;
        let result = self.list_handle(&handle).await;
        let closed = self.close_handle(&handle).await;
        match result {
            Ok(list) => {
                closed?;
                Ok(list)
            }
            Err(error) => Err(error),
        }
    }

    async fn list_handle(&mut self, handle: &[u8]) -> Result<Directory, SftpError> {
        let mut entries = Vec::new();
        let mut truncated = false;
        loop {
            let mut payload = Vec::new();
            string(&mut payload, handle);
            let reply = match self.request(12, payload).await {
                Err(SftpError::EndOfFile) => break,
                reply => reply?,
            };
            if reply[0] != 104 {
                return Err(SftpError::Protocol);
            }
            let mut cursor = Cursor(&reply[1..]);
            let count = cursor.u32()?;
            // Each entry needs at least two string lengths and attribute flags.
            if count == 0 || count as usize > cursor.0.len() / 12 {
                return Err(SftpError::Protocol);
            }
            for _ in 0..count {
                let name = name(cursor.string()?)?;
                cursor.string()?;
                let attributes = cursor.attributes()?;
                if matches!(name.as_str(), "." | "..") {
                    continue;
                }
                if name.contains('/') || (self.windows && name.contains('\\')) {
                    return Err(SftpError::Protocol);
                }
                if entries.len() == MAX_ENTRIES {
                    truncated = true;
                } else {
                    entries.push(Entry { name, attributes });
                }
            }
            cursor.finish()?;
            if truncated {
                break;
            }
        }
        entries.sort_by(|a, b| {
            b.attributes
                .is_directory()
                .cmp(&a.attributes.is_directory())
                .then(a.name.cmp(&b.name))
        });
        Ok(Directory { entries, truncated })
    }

    pub async fn read(&mut self, relative: &str) -> Result<Vec<u8>, SftpError> {
        let path = self.scoped_path(relative).await?;
        let attrs = self.stat_path(&path).await?;
        if !attrs.is_regular_file() {
            return Err(SftpError::Unsupported);
        }
        if attrs.size.is_some_and(|size| size > MAX_FILE as u64) {
            return Err(SftpError::CapacityExceeded);
        }
        let handle = self.handle(3, &path).await?;
        let result = self.read_handle(&handle).await;
        let closed = self.close_handle(&handle).await;
        match result {
            Ok(bytes) => {
                closed?;
                Ok(bytes)
            }
            Err(error) => Err(error),
        }
    }

    async fn read_handle(&mut self, handle: &[u8]) -> Result<Vec<u8>, SftpError> {
        let mut bytes = Vec::new();
        loop {
            let mut payload = Vec::new();
            string(&mut payload, handle);
            payload.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
            payload.extend_from_slice(&CHUNK.to_be_bytes());
            let reply = match self.request(5, payload).await {
                Err(SftpError::EndOfFile) => break,
                reply => reply?,
            };
            if reply[0] != 103 {
                return Err(SftpError::Protocol);
            }
            let mut cursor = Cursor(&reply[1..]);
            let chunk = cursor.string()?;
            cursor.finish()?;
            if chunk.is_empty() || chunk.len() > CHUNK as usize {
                return Err(SftpError::Protocol);
            }
            if bytes.len() + chunk.len() > MAX_FILE {
                return Err(SftpError::CapacityExceeded);
            }
            bytes.extend_from_slice(chunk);
        }
        Ok(bytes)
    }

    /// Resolve the parent remotely, preserving the final link as an entry.
    async fn scoped_entry(&mut self, relative: &str) -> Result<String, SftpError> {
        self.relative_path(relative)?;
        let (parent, leaf) = relative.rsplit_once('/').unwrap_or(("", relative));
        if leaf.is_empty() || relative.split('/').any(str::is_empty) {
            return Err(SftpError::InvalidName);
        }
        let parent = self.scoped_path(parent).await?;
        if !self.stat_path(&parent).await?.is_directory() {
            return Err(SftpError::Unsupported);
        }
        Ok(format!("{}/{leaf}", parent.trim_end_matches('/')))
    }

    async fn mutation(&mut self, kind: u8, payload: Vec<u8>) -> Result<(), SftpError> {
        match self.request(kind, payload).await {
            Ok(reply) if reply == [101] => Ok(()),
            Ok(_) | Err(SftpError::Protocol | SftpError::ConnectionLost) => {
                self.disconnect();
                Err(SftpError::CommitUnknown)
            }
            Err(error) => Err(error),
        }
    }

    pub async fn create_directory(&mut self, relative: &str) -> Result<(), SftpError> {
        let path = self.scoped_entry(relative).await?;
        let mut payload = Vec::new();
        string(&mut payload, path.as_bytes());
        payload.extend_from_slice(&4u32.to_be_bytes());
        payload.extend_from_slice(&0o700u32.to_be_bytes());
        self.mutation(14, payload).await
    }

    /// Deletes the explicitly selected entry; never recurses or follows a link.
    pub async fn remove(&mut self, relative: &str) -> Result<(), SftpError> {
        let path = self.scoped_entry(relative).await?;
        let attrs = self.stat_path(&path).await?;
        let mut payload = Vec::new();
        string(&mut payload, path.as_bytes());
        self.mutation(if attrs.is_directory() { 15 } else { 13 }, payload)
            .await
    }

    pub async fn rename_new(&mut self, from: &str, to: &str) -> Result<(), SftpError> {
        let from = self.scoped_entry(from).await?;
        let to = self.scoped_entry(to).await?;
        match self.stat_path(&to).await {
            Err(SftpError::Missing) => (),
            Ok(_) => return Err(SftpError::Conflict),
            Err(error) => return Err(error),
        }
        let mut payload = Vec::new();
        string(&mut payload, from.as_bytes());
        string(&mut payload, to.as_bytes());
        // v3 RENAME must fail when the destination exists; no overwrite extension.
        self.mutation(18, payload).await
    }

    /// Pins the intent before IO so a lost response retains its exact partial path.
    pub fn upload_intent(
        &self,
        destination: &str,
        bytes: &[u8],
    ) -> Result<UploadReceipt, SftpError> {
        self.relative_path(destination)?;
        if destination.is_empty() || destination.split('/').any(str::is_empty) {
            return Err(SftpError::InvalidName);
        }
        if bytes.len() > MAX_FILE {
            return Err(SftpError::CapacityExceeded);
        }
        let parent = destination.rsplit_once('/').map(|(parent, _)| parent);
        let leaf = format!(".warpai-upload-{}.partial", Uuid::new_v4());
        let partial = parent.map_or(leaf.clone(), |parent| format!("{parent}/{leaf}"));
        Ok(UploadReceipt {
            profile_id: self.profile_id,
            canonical_root: self.canonical_root.clone(),
            destination: destination.into(),
            partial,
            size: bytes.len() as u64,
            sha256: Sha256::digest(bytes).into(),
        })
    }

    fn validate_receipt(&self, receipt: &UploadReceipt) -> Result<(), SftpError> {
        if receipt.profile_id != self.profile_id
            || receipt.canonical_root != self.canonical_root
            || receipt.size > MAX_FILE as u64
        {
            return Err(SftpError::Conflict);
        }
        self.relative_path(&receipt.destination)?;
        self.relative_path(&receipt.partial)?;
        let (parent, leaf) = receipt
            .partial
            .rsplit_once('/')
            .unwrap_or(("", &receipt.partial));
        let expected_parent = receipt.destination.rsplit_once('/').map_or("", |(p, _)| p);
        let id = leaf
            .strip_prefix(".warpai-upload-")
            .and_then(|s| s.strip_suffix(".partial"));
        if parent != expected_parent || id.is_none_or(|id| Uuid::parse_str(id).is_err()) {
            return Err(SftpError::InvalidName);
        }
        Ok(())
    }

    pub async fn prepare_upload(
        &mut self,
        receipt: &UploadReceipt,
        bytes: &[u8],
    ) -> Result<(), SftpError> {
        self.validate_receipt(receipt)?;
        if receipt.size != bytes.len() as u64
            || receipt.sha256 != <[u8; 32]>::from(Sha256::digest(bytes))
        {
            return Err(SftpError::Conflict);
        }
        let path = self.scoped_entry(&receipt.partial).await?;
        let mut payload = Vec::new();
        string(&mut payload, path.as_bytes());
        payload.extend_from_slice(&(2u32 | 8 | 32).to_be_bytes()); // WRITE | CREAT | EXCL
        payload.extend_from_slice(&4u32.to_be_bytes());
        payload.extend_from_slice(&0o600u32.to_be_bytes());
        let reply = match self.request(3, payload).await {
            Err(SftpError::ConnectionLost | SftpError::Protocol) => {
                return Err(SftpError::CommitUnknown)
            }
            reply => reply?,
        };
        if reply[0] != 102 {
            self.disconnect();
            return Err(SftpError::CommitUnknown);
        }
        let decoded = (|| {
            let mut cursor = Cursor(&reply[1..]);
            let handle = cursor.string()?.to_vec();
            cursor.finish()?;
            if handle.is_empty() || handle.len() > 256 {
                return Err(SftpError::Protocol);
            }
            Ok(handle)
        })();
        let handle = match decoded {
            Ok(handle) => handle,
            Err(_) => {
                self.disconnect();
                return Err(SftpError::CommitUnknown);
            }
        };
        let result: Result<(), SftpError> = async {
            for (index, chunk) in bytes.chunks(CHUNK as usize).enumerate() {
                let mut payload = Vec::new();
                string(&mut payload, &handle);
                payload.extend_from_slice(&(index as u64 * CHUNK as u64).to_be_bytes());
                string(&mut payload, chunk);
                self.mutation(6, payload).await?;
            }
            Ok(())
        }
        .await;
        let closed = self.close_handle(&handle).await;
        result?;
        closed.map_err(|_| SftpError::CommitUnknown)?;
        if !self.matches_upload(&receipt.partial, receipt).await? {
            return Err(SftpError::Conflict);
        }
        Ok(())
    }

    async fn matches_upload(
        &mut self,
        relative: &str,
        receipt: &UploadReceipt,
    ) -> Result<bool, SftpError> {
        let path = self.scoped_entry(relative).await?;
        let attrs = self.stat_path(&path).await?;
        if !attrs.is_regular_file() || attrs.size != Some(receipt.size) {
            return Ok(false);
        }
        let bytes = self.read(relative).await?;
        Ok(bytes.len() as u64 == receipt.size
            && <[u8; 32]>::from(Sha256::digest(&bytes)) == receipt.sha256)
    }

    pub async fn reconcile_upload(
        &mut self,
        receipt: &UploadReceipt,
    ) -> Result<UploadState, SftpError> {
        self.validate_receipt(receipt)?;
        let partial = self.matches_upload(&receipt.partial, receipt).await;
        let destination = self.matches_upload(&receipt.destination, receipt).await;
        Ok(match (partial, destination) {
            (Err(SftpError::Missing), Ok(true)) => UploadState::Confirmed,
            (Ok(true), Err(SftpError::Missing)) => UploadState::Prepared,
            (Ok(_), Ok(_)) | (Ok(false), _) | (_, Ok(false)) => UploadState::Conflict,
            _ => UploadState::Unknown,
        })
    }

    pub async fn commit_upload(&mut self, receipt: &UploadReceipt) -> Result<(), SftpError> {
        if self.reconcile_upload(receipt).await? != UploadState::Prepared {
            return Err(SftpError::Conflict);
        }
        self.rename_new(&receipt.partial, &receipt.destination)
            .await?;
        match self.reconcile_upload(receipt).await {
            Ok(UploadState::Confirmed) => Ok(()),
            _ => Err(SftpError::CommitUnknown),
        }
    }

    async fn request(&mut self, kind: u8, payload: Vec<u8>) -> Result<Vec<u8>, SftpError> {
        if !self.alive {
            return Err(SftpError::ConnectionLost);
        }
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or(SftpError::CapacityExceeded)?;
        let id = self.sequence;
        let mut packet = vec![kind];
        packet.extend_from_slice(&id.to_be_bytes());
        packet.extend(payload);
        let response = tokio::time::timeout(Duration::from_secs(10), async {
            self.write_packet(&packet).await?;
            self.read_packet().await
        })
        .await;
        let Ok(Ok(packet)) = response else {
            self.disconnect();
            return Err(SftpError::ConnectionLost);
        };
        let mut cursor = Cursor(&packet[1..]);
        if cursor.u32()? != id {
            self.disconnect();
            return Err(SftpError::Protocol);
        }
        if packet[0] == 101 {
            status(&mut cursor)?;
            return Ok(vec![101]);
        }
        let mut payload = vec![packet[0]];
        payload.extend_from_slice(cursor.0);
        Ok(payload)
    }

    async fn write_packet(&mut self, packet: &[u8]) -> Result<(), SftpError> {
        if packet.len() > MAX_PACKET {
            return Err(SftpError::CapacityExceeded);
        }
        self.input
            .write_all(&(packet.len() as u32).to_be_bytes())
            .await
            .map_err(|_| SftpError::ConnectionLost)?;
        self.input
            .write_all(packet)
            .await
            .map_err(|_| SftpError::ConnectionLost)?;
        self.input
            .flush()
            .await
            .map_err(|_| SftpError::ConnectionLost)
    }

    async fn read_packet(&mut self) -> Result<Vec<u8>, SftpError> {
        let size = self
            .output
            .read_u32()
            .await
            .map_err(|_| SftpError::ConnectionLost)? as usize;
        if !(5..=MAX_PACKET).contains(&size) {
            return Err(SftpError::Protocol);
        }
        let mut packet = vec![0; size];
        self.output
            .read_exact(&mut packet)
            .await
            .map_err(|_| SftpError::ConnectionLost)?;
        Ok(packet)
    }

    pub fn disconnect(&mut self) {
        self.alive = false;
        let _ = self.child.start_kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binary_sftp_decoder_bounds_lengths_flags_and_error_prose() {
        let bytes = [0xff; 4];
        assert_eq!(Cursor(&bytes).string(), Err(SftpError::Protocol));
        assert!(Cursor(&[0, 0, 0, 16]).attributes().is_err());
        let mut payload = 3u32.to_be_bytes().to_vec();
        string(&mut payload, b"untrusted peer diagnostic");
        string(&mut payload, b"en");
        assert_eq!(
            status(&mut Cursor(&payload)),
            Err(SftpError::PermissionDenied)
        );
        assert_eq!(name(b"malicious\0name"), Err(SftpError::InvalidName));
        assert_eq!(
            name("Unicode 空格;$(shell)".as_bytes()).unwrap(),
            "Unicode 空格;$(shell)"
        );
    }
}
