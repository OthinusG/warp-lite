//! Remote device secrets use the existing platform provider, never preferences or MCP input.
use anyhow::{anyhow, ensure, Result};
use uuid::Uuid;
use warpui_extras::secure_storage::{Error, SecureStorage};

fn key(coordinator: Uuid, device: Uuid) -> Result<String> {
    ensure!(
        !coordinator.is_nil() && !device.is_nil(),
        "Remote credential identities are required"
    );
    Ok(format!("agent-remote-{coordinator}-{device}"))
}

fn valid(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}

fn missing(error: &Error) -> bool {
    if matches!(error, Error::NotFound) {
        return true;
    }
    #[cfg(windows)]
    if let Error::IOError(error) = error {
        return error.kind() == std::io::ErrorKind::NotFound;
    }
    false
}

/// Enrollment must not replace an existing device key; verify persistence before enabling access.
pub(super) fn save(
    storage: &dyn SecureStorage,
    coordinator: Uuid,
    device: Uuid,
    value: &str,
) -> Result<()> {
    let key = key(coordinator, device)?;
    ensure!(valid(value), "Invalid remote credential");
    match storage.read_value(&key) {
        Ok(value) if !value.is_empty() => {
            return Err(anyhow!(
                "Device credential already exists; use its existing connection"
            ))
        }
        Err(error) if missing(&error) => {}
        _ => {
            return Err(anyhow!(
                "Secure storage is unavailable; remote access remains disabled"
            ))
        }
    }
    storage.write_value(&key, value).map_err(|_| {
        anyhow!("Secure storage could not save the credential; remote access remains disabled")
    })?;
    if storage.read_value(&key).is_ok_and(|stored| stored == value) {
        return Ok(());
    }
    let _ = storage.remove_value(&key);
    Err(anyhow!(
        "Secure storage verification failed; request a new invitation"
    ))
}

pub(super) fn load(storage: &dyn SecureStorage, coordinator: Uuid, device: Uuid) -> Result<String> {
    let key = key(coordinator, device)?;
    let value = storage.read_value(&key).map_err(|_| {
        anyhow!("Unlock secure storage or enroll again; remote access remains disabled")
    })?;
    ensure!(
        valid(&value),
        "Secure storage has no valid device credential; remote access remains disabled"
    );
    Ok(value)
}

/// Fence participation before removing its key; storage failure does not re-enable the connection.
pub(super) fn remove(storage: &dyn SecureStorage, coordinator: Uuid, device: Uuid) -> Result<()> {
    let key = key(coordinator, device)?;
    match storage.remove_value(&key) {
        Ok(()) => {}
        Err(error) if missing(&error) => {}
        Err(_) => {
            return Err(anyhow!(
                "Credential cleanup failed; connection remains disabled"
            ))
        }
    }
    match storage.read_value(&key) {
        Err(error) if missing(&error) => Ok(()),
        _ => Err(anyhow!(
            "Credential cleanup could not be verified; connection remains disabled"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::HashMap};

    #[derive(Default)]
    struct Storage {
        values: RefCell<HashMap<String, String>>,
        mode: u8,
    }
    impl SecureStorage for Storage {
        fn read_value(&self, key: &str) -> std::result::Result<String, Error> {
            if self.mode == 1 {
                return Err(Error::Unknown(anyhow!("locked fixture")));
            }
            if self.mode == 2 {
                return Ok(String::new());
            }
            self.values
                .borrow()
                .get(key)
                .cloned()
                .ok_or(Error::NotFound)
        }
        fn write_value(&self, key: &str, value: &str) -> std::result::Result<(), Error> {
            if self.mode == 3 {
                return Err(Error::Unknown(anyhow!("write fixture")));
            }
            self.values
                .borrow_mut()
                .insert(key.into(), if self.mode == 4 { "" } else { value }.into());
            Ok(())
        }
        fn remove_value(&self, key: &str) -> std::result::Result<(), Error> {
            if self.mode != 5 {
                self.values.borrow_mut().remove(key);
            }
            Ok(())
        }
    }

    #[test]
    fn remote_credentials_fail_closed_and_preserve_other_devices() {
        let coordinator = Uuid::new_v4();
        let device = Uuid::new_v4();
        let other = Uuid::new_v4();
        let storage = Storage::default();
        assert!(load(&storage, coordinator, device).is_err());
        assert!(save(&storage, Uuid::nil(), device, "fixture").is_err());
        for invalid in ["", "fixture\n", &"a".repeat(513)] {
            assert!(save(&storage, coordinator, device, invalid).is_err());
        }
        save(&storage, coordinator, device, "synthetic-original").unwrap();
        save(&storage, coordinator, other, "synthetic-other").unwrap();
        assert!(save(&storage, coordinator, device, "replacement").is_err());
        assert_eq!(
            load(&storage, coordinator, device).unwrap(),
            "synthetic-original"
        );
        remove(&storage, coordinator, device).unwrap();
        remove(&storage, coordinator, device).unwrap();
        assert!(load(&storage, coordinator, device).is_err());
        assert_eq!(
            load(&storage, coordinator, other).unwrap(),
            "synthetic-other"
        );
        for mode in 1..=4 {
            let storage = Storage {
                mode,
                ..Default::default()
            };
            assert!(save(&storage, coordinator, device, "synthetic").is_err());
            assert!(storage.values.borrow().is_empty());
            assert!(load(&storage, coordinator, device).is_err());
        }
        let storage = Storage {
            mode: 5,
            ..Default::default()
        };
        save(&storage, coordinator, device, "synthetic").unwrap();
        assert!(remove(&storage, coordinator, device).is_err());
        assert!(key(coordinator, device)
            .unwrap()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-'));
    }
}
