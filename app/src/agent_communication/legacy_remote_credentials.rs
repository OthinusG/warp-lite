//! Removal of known legacy keys only; SSH authentication remains system-owned.
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

/// Retired keys are never loaded; a provider error leaves cleanup pending.
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
    Ok(())
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
        fn read_value(&self, _: &str) -> std::result::Result<String, Error> {
            panic!("Legacy cleanup must not read credentials")
        }
        fn write_value(&self, _: &str, _: &str) -> std::result::Result<(), Error> {
            panic!("Legacy cleanup must not issue credentials")
        }
        fn remove_value(&self, key: &str) -> std::result::Result<(), Error> {
            if self.mode == 1 {
                return Err(Error::Unknown(anyhow!("locked fixture")));
            }
            self.values.borrow_mut().remove(key);
            Ok(())
        }
    }

    #[test]
    fn legacy_credential_cleanup_preserves_other_keys_and_reports_failures() {
        let coordinator = Uuid::new_v4();
        let device = Uuid::new_v4();
        let other = Uuid::new_v4();
        let storage = Storage::default();
        let owned = key(coordinator, device).unwrap();
        let unrelated = key(coordinator, other).unwrap();
        storage.values.borrow_mut().insert(owned.clone(), "synthetic-owned".into());
        storage.values.borrow_mut().insert(unrelated.clone(), "synthetic-other".into());
        remove(&storage, coordinator, device).unwrap();
        remove(&storage, coordinator, device).unwrap();
        assert!(!storage.values.borrow().contains_key(&owned));
        assert!(storage.values.borrow().contains_key(&unrelated));
        assert!(remove(&storage, Uuid::nil(), device).is_err());
        let locked = Storage { mode: 1, ..Default::default() };
        assert!(remove(&locked, coordinator, device).is_err());
        assert!(owned.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    }
}
