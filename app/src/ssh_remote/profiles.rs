//! Versioned connection metadata; credential values are not accepted fields.
use serde::{Deserialize, Serialize};
use std::{io::Read, path::Path};
use uuid::Uuid;
use warp_agent_bus::ssh_remote::SshProfile;

const MAX_BYTES: usize = 128 * 1024;
const MAX_PROFILES: usize = 64;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Profiles {
    pub(super) version: u32,
    pub(super) profiles: Vec<SshProfile>,
    pub(super) selected: Option<Uuid>,
}
impl Default for Profiles {
    fn default() -> Self {
        Self {
            version: 1,
            profiles: vec![],
            selected: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidMetadata,
    UnsupportedVersion,
    CapacityExceeded,
    StorageUnavailable,
}

impl Profiles {
    pub(super) fn load(path: &Path) -> Result<Self, Error> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // FILE_FLAG_OPEN_REPARSE_POINT: inspect the entry, never follow it.
            options.custom_flags(0x00200000);
        }
        let file = match options.open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(_) => return Err(Error::StorageUnavailable),
        };
        let metadata = file.metadata().map_err(|_| Error::StorageUnavailable)?;
        if !metadata.is_file() {
            return Err(Error::StorageUnavailable);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(Error::StorageUnavailable);
            }
        }
        let mut bytes = Vec::new();
        file.take((MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::StorageUnavailable)?;
        Self::decode(&bytes)
    }

    pub(super) fn save(&self, path: &Path) -> Result<(), Error> {
        let bytes = self.encode()?;
        crate::agent_communication::setup::atomic_write(path, &bytes)
            .map_err(|_| Error::StorageUnavailable)
    }

    fn validate(&self) -> Result<(), Error> {
        if self.version != 1 {
            return Err(Error::UnsupportedVersion);
        }
        if self.profiles.len() > MAX_PROFILES {
            return Err(Error::CapacityExceeded);
        }
        let mut ids = std::collections::HashSet::new();
        for profile in &self.profiles {
            if profile.validate().is_err() || !ids.insert(profile.id) {
                return Err(Error::InvalidMetadata);
            }
        }
        if self.selected.is_some_and(|id| !ids.contains(&id)) {
            return Err(Error::InvalidMetadata);
        }
        Ok(())
    }
    pub(super) fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_BYTES {
            return Err(Error::CapacityExceeded);
        }
        // Deserializer prose may echo external content; only the fixed enum escapes.
        let profiles: Self = serde_json::from_slice(bytes).map_err(|_| Error::InvalidMetadata)?;
        profiles.validate()?;
        Ok(profiles)
    }
    pub(super) fn encode(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|_| Error::InvalidMetadata)?;
        if bytes.len() > MAX_BYTES {
            return Err(Error::CapacityExceeded);
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profiles_reject_secrets_duplicate_ids_and_stale_selection_without_reading_key_references() {
        let profile = serde_json::json!({"id": Uuid::new_v4(), "display_name": "Research",
            "target": "research-alias", "user": null, "port": null, "identity_file": "/nonexistent/reference-only",
            "config_file": null, "jump_alias": null, "remote_root": "/srv/project", "companion_path": null,
            "remote_shell": "posix"});
        let mut data =
            serde_json::json!({"version":1,"profiles":[profile.clone()],"selected":null});
        let decoded = Profiles::decode(&serde_json::to_vec(&data).unwrap()).unwrap();
        assert_eq!(
            decoded.profiles[0].identity_file.as_ref().unwrap().to_str(),
            Some("/nonexistent/reference-only")
        );
        assert!(Profiles::decode(&decoded.encode().unwrap()).is_ok());
        data["profiles"][0]["password"] = serde_json::json!("forbidden test value");
        assert!(matches!(
            Profiles::decode(&serde_json::to_vec(&data).unwrap()),
            Err(Error::InvalidMetadata)
        ));
        data["profiles"] = serde_json::json!([profile.clone(), profile.clone()]);
        assert!(matches!(
            Profiles::decode(&serde_json::to_vec(&data).unwrap()),
            Err(Error::InvalidMetadata)
        ));
        data["profiles"] = serde_json::json!([profile]);
        data["selected"] = serde_json::json!(Uuid::new_v4());
        assert!(matches!(
            Profiles::decode(&serde_json::to_vec(&data).unwrap()),
            Err(Error::InvalidMetadata)
        ));
        data["selected"] = serde_json::Value::Null;
        data["version"] = serde_json::json!(2);
        assert!(matches!(
            Profiles::decode(&serde_json::to_vec(&data).unwrap()),
            Err(Error::UnsupportedVersion)
        ));
        assert!(matches!(
            Profiles::decode(&vec![b' '; MAX_BYTES + 1]),
            Err(Error::CapacityExceeded)
        ));
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ssh-projects.json");
        assert!(Profiles::load(&path).unwrap().profiles.is_empty());
        decoded.save(&path).unwrap();
        assert_eq!(
            Profiles::load(&path).unwrap().profiles[0].id,
            decoded.profiles[0].id
        );
        let original = std::fs::read(&path).unwrap();
        let mut invalid = decoded.clone();
        invalid.selected = Some(Uuid::new_v4());
        assert_eq!(invalid.save(&path), Err(Error::InvalidMetadata));
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::write(&path, b"invalid metadata").unwrap();
        assert!(matches!(Profiles::load(&path), Err(Error::InvalidMetadata)));
        #[cfg(unix)]
        {
            let link = directory.path().join("metadata-link.json");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert!(matches!(
                Profiles::load(&link),
                Err(Error::StorageUnavailable)
            ));
            assert_eq!(decoded.save(&link), Err(Error::StorageUnavailable));
            assert_eq!(std::fs::read(&path).unwrap(), b"invalid metadata");
        }
    }
}
