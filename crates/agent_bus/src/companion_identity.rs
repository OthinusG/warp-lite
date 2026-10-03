//! Nonsecret account/service identity in an owned private native directory.
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
use uuid::Uuid;

pub(super) struct Identity {
    pub service_id: String,
    pub account_id: String,
}

fn denied() -> std::io::Error {
    std::io::Error::from(std::io::ErrorKind::PermissionDenied)
}

impl Identity {
    pub fn open(directory: &Path) -> std::io::Result<Self> {
        private_directory(directory)?;
        let lock = private_file(&directory.join("identity.lock"))?;
        lock.try_lock().map_err(std::io::Error::other)?;
        let mut file = private_file(&directory.join("service.id"))?;
        let id = match file.metadata()?.len() {
            0 => {
                let id = Uuid::new_v4();
                file.write_all(id.as_bytes())?;
                file.sync_all()?;
                id
            }
            16 => {
                let mut bytes = [0; 16];
                file.read_exact(&mut bytes)?;
                let id = Uuid::from_bytes(bytes);
                if id.is_nil() {
                    return Err(std::io::Error::other("Service identity is unavailable"));
                }
                id
            }
            _ => return Err(std::io::Error::other("Service identity is unavailable")),
        };
        Ok(Self {
            service_id: id.to_string(),
            account_id: account_id()?,
        })
    }

    /// Native directory identity and birth time avoid alias/path-text equality.
    pub fn project(&self, file: &File) -> std::io::Result<(String, String)> {
        let metadata = file.metadata()?;
        let created = metadata
            .created()?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(std::io::Error::other)?;
        let mut digest = Sha256::new();
        digest.update(std::env::consts::OS.as_bytes());
        digest.update([0]);
        digest.update(self.service_id.as_bytes());
        digest.update(self.account_id.as_bytes());
        digest.update(native_file_identity(file)?);
        digest.update(created.as_nanos().to_be_bytes());
        let hash = digest.finalize();
        let id = Uuid::from_bytes(hash[..16].try_into().unwrap());
        let fingerprint = hash.iter().map(|b| format!("{b:02x}")).collect();
        Ok((id.to_string(), fingerprint))
    }
}

#[cfg(unix)]
fn account_id() -> std::io::Result<String> {
    Ok(format!("uid:{}", unsafe { libc::geteuid() }))
}

#[cfg(windows)]
fn account_id() -> std::io::Result<String> {
    crate::transport::windows_pipe::current_user_sid().map_err(std::io::Error::other)
}

#[cfg(unix)]
fn private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)?;
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(denied());
    }
    Ok(())
}

#[cfg(unix)]
fn private_file(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(denied());
    }
    Ok(file)
}

#[cfg(unix)]
fn native_file_identity(file: &File) -> std::io::Result<Vec<u8>> {
    use std::os::unix::fs::MetadataExt;
    let m = file.metadata()?;
    Ok([m.dev().to_be_bytes(), m.ino().to_be_bytes()].concat())
}

#[cfg(windows)]
fn private_file(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .custom_flags(0x00200000)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
        return Err(denied());
    }
    Ok(file)
}

#[cfg(windows)]
fn native_file_identity(file: &File) -> std::io::Result<Vec<u8>> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
    };
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info) }
        .map_err(std::io::Error::other)?;
    Ok([
        info.dwVolumeSerialNumber.to_be_bytes(),
        info.nFileIndexHigh.to_be_bytes(),
        info.nFileIndexLow.to_be_bytes(),
    ]
    .concat())
}

#[cfg(windows)]
fn private_directory(path: &Path) -> std::io::Result<()> {
    use windows::{
        core::{Owned, HSTRING, PWSTR},
        Win32::{
            Foundation::HLOCAL,
            Security::{
                Authorization::{
                    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
                    GetNamedSecurityInfoW, SE_FILE_OBJECT,
                },
                GetAce, ACCESS_ALLOWED_ACE, ACL, DACL_SECURITY_INFORMATION,
                OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES,
            },
            Storage::FileSystem::CreateDirectoryW,
        },
    };
    let sid = account_id()?;
    let sddl = HSTRING::from(format!("O:{sid}D:P(A;OICI;FA;;;{sid})"));
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(&sddl, 1, &mut descriptor, None)
    }
    .map_err(std::io::Error::other)?;
    let _descriptor = unsafe { Owned::new(HLOCAL(descriptor.0)) };
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: false.into(),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let path_string = HSTRING::from(path.as_os_str());
    if !path.try_exists()? {
        unsafe { CreateDirectoryW(&path_string, Some(&attributes)) }
            .map_err(std::io::Error::other)?;
    }
    use std::os::windows::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
        return Err(denied());
    }
    let mut actual = PSECURITY_DESCRIPTOR::default();
    let mut owner = PSID::default();
    let mut dacl: *mut ACL = std::ptr::null_mut();
    unsafe {
        GetNamedSecurityInfoW(
            &path_string,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | OWNER_SECURITY_INFORMATION,
            Some(&mut owner),
            None,
            Some(&mut dacl),
            None,
            &mut actual,
        )
    }
    .ok()
    .map_err(std::io::Error::other)?;
    let _actual = unsafe { Owned::new(HLOCAL(actual.0)) };
    if owner.0.is_null() || dacl.is_null() || unsafe { (*dacl).AceCount } != 1 {
        return Err(denied());
    }
    let mut ace = std::ptr::null_mut();
    unsafe { GetAce(dacl, 0, &mut ace) }.map_err(std::io::Error::other)?;
    let ace = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
    if ace.Header.AceType != 0 || ace.Mask != 0x1f01ff {
        return Err(denied());
    }
    for native_sid in [owner, PSID((&ace.SidStart as *const u32).cast_mut().cast())] {
        let mut text = PWSTR::null();
        unsafe { ConvertSidToStringSidW(native_sid, &mut text) }.map_err(std::io::Error::other)?;
        let _text = unsafe { Owned::new(HLOCAL(text.0.cast())) };
        if unsafe { text.to_string() }.map_err(std::io::Error::other)? != sid {
            return Err(denied());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_persists_without_accepting_corruption_links_or_replaced_roots() {
        let tmp = tempfile::tempdir().unwrap();
        let state = tmp.path().join("state");
        let first = Identity::open(&state).unwrap();
        let second = Identity::open(&state).unwrap();
        assert_eq!(first.service_id, second.service_id);
        assert_eq!(first.account_id, second.account_id);
        let project = tmp.path().join("project");
        std::fs::create_dir(&project).unwrap();
        let handle = super::super::open_root(&project).unwrap();
        let original = first.project(&handle).unwrap();
        assert_eq!(original, second.project(&handle).unwrap());
        std::fs::rename(&project, tmp.path().join("old-project")).unwrap();
        std::fs::create_dir(&project).unwrap();
        assert_ne!(
            original,
            first
                .project(&super::super::open_root(&project).unwrap())
                .unwrap()
        );
        let lock = private_file(&state.join("identity.lock")).unwrap();
        lock.try_lock().unwrap();
        assert!(Identity::open(&state).is_err());
        drop(lock);
        std::fs::write(state.join("service.id"), b"invalid-nonsecret-id").unwrap();
        assert!(Identity::open(&state).is_err());
        #[cfg(unix)]
        {
            std::fs::remove_file(state.join("service.id")).unwrap();
            std::os::unix::fs::symlink(tmp.path().join("outside"), state.join("service.id"))
                .unwrap();
            assert!(Identity::open(&state).is_err());
            assert!(!tmp.path().join("outside").exists());
        }
    }
}
