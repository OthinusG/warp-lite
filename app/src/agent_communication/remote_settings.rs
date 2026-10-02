//! Native enrollment persists only reviewed connection metadata after secure credential verification.
use super::{remote_credentials, setup, setup::RemoteProfile as Profile, AgentCommunication};
use anyhow::{anyhow, ensure, Result};
use uuid::Uuid;
use warp_agent_bus::{remote::Connection, transport::remote_control::AuthenticationFrame};
use warpui::ModelContext;
use warpui_extras::secure_storage::AppContextExt;

// No Debug/Serialize: this value is transferred in memory and consumed by the native secure provider.
pub(super) struct Enrollment {
    profile: Profile,
    credential: String,
}

impl AgentCommunication {
    pub(crate) fn remote_enrolling(&self) -> bool {
        self.remote_pending.is_some()
    }
    pub(crate) fn enroll_remote(
        &mut self,
        alias: String,
        coordinator: Uuid,
        name: String,
        invitation: String,
        ctx: &mut ModelContext<Self>,
    ) -> Result<()> {
        ensure!(
            self.preferences.enabled && !self.busy && self.remote_pending.is_none(),
            "Enable communication and wait for current setup to finish"
        );
        ensure!(
            self.preferences.remote_profiles.len() < 32,
            "Connection capacity reached"
        );
        warp_agent_bus::remote::validate_alias(&alias)?;
        ensure!(
            !coordinator.is_nil()
                && !name.trim().is_empty()
                && name.len() <= 64
                && !name.chars().any(char::is_control)
                && !invitation.is_empty()
                && invitation.len() <= 512
                && !invitation.chars().any(char::is_control),
            "Enter the expected coordinator UUID, device name and a current invitation"
        );
        let (sender, receiver) = std::sync::mpsc::channel();
        let (cancel, mut cancelled) = tokio::sync::watch::channel(false);
        self.remote_cancel = Some(cancel);
        self.remote_pending = Some(receiver);
        self.remote_status = "Connecting using system SSH; enrollment is not yet saved.".into();
        std::thread::spawn(move || {
            let result = (|| -> Result<Enrollment> {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| anyhow!("Remote connection worker is unavailable"))?;
                runtime.block_on(async move {
                    tokio::select! {
                        _ = cancelled.changed() => Err(anyhow!("Enrollment cancelled")),
                        result = async move {
                        let mut connection = Connection::open(&alias, coordinator).await?;
                        let AuthenticationFrame::EnrollResult {
                            device_id,
                            credential,
                            generation,
                            space_ids,
                        } = connection.enroll(invitation, name).await?
                        else {
                            return Err(anyhow!("Unexpected enrollment result"));
                        };
                        let profile = Profile {
                            alias,
                            coordinator,
                            device: device_id,
                            generation,
                            spaces: space_ids,
                        };
                        profile.validate()?;
                        Ok(Enrollment {
                            profile,
                            credential,
                        })
                        } => result,
                    }
                })
            })();
            // Peer diagnostics never enter logs or the settings projection.
            let _ = sender.send(result.map_err(|_| anyhow!(
                "Enrollment failed. Check SSH authentication, known hosts, the host app and gateway PATH, then request a current invitation.")));
        });
        ctx.notify();
        Ok(())
    }

    pub(crate) fn cancel_remote_enrollment(&mut self, ctx: &mut ModelContext<Self>) {
        if self.remote_pending.is_none() {
            return;
        }
        self.remote_pending = None;
        self.remote_cancel = None;
        self.remote_status = "Enrollment cancelled; no connection was enabled. Ask the host to remove any newly enrolled device.".into();
        ctx.notify();
    }

    pub(super) fn poll_remote_enrollment(&mut self, ctx: &mut ModelContext<Self>) {
        let Some(receiver) = self.remote_pending.as_ref() else {
            return;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(anyhow!(
                "Enrollment worker stopped; participation remains disabled"
            )),
        };
        self.remote_pending = None;
        self.remote_cancel = None;
        let result = result.and_then(|enrollment| {
            ensure!(
                self.preferences.enabled && !self.busy,
                "Communication changed; enrollment was not saved"
            );
            ensure!(
                self.preferences.remote_profiles.len() < 32
                    && !self
                        .preferences
                        .remote_profiles
                        .iter()
                        .any(
                            |profile| profile.coordinator == enrollment.profile.coordinator
                                && profile.device == enrollment.profile.device
                        ),
                "This device connection already exists or capacity was reached"
            );
            let profile = enrollment.profile;
            remote_credentials::save(
                ctx.secure_storage(),
                profile.coordinator,
                profile.device,
                &enrollment.credential,
            )?;
            let mut preferences = self.preferences.clone();
            preferences.remote_profiles.push(profile.clone());
            if setup::save_preferences(&self.preferences_path, &preferences).is_err() {
                let _ = remote_credentials::remove(
                    ctx.secure_storage(),
                    profile.coordinator,
                    profile.device,
                );
                return Err(anyhow!(
                    "Connection metadata could not be saved; participation remains disabled"
                ));
            }
            self.preferences = preferences;
            Ok(())
        });
        self.remote_status = match result {
            Ok(()) => {
                "Device credential saved securely. Review a checkout mapping before participation."
                    .into()
            }
            Err(error) => error.to_string(),
        };
        ctx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_profile_metadata_excludes_secrets_and_validates_reviewed_identity() {
        let profile = Profile {
            alias: "owned-host".into(),
            coordinator: Uuid::new_v4(),
            device: Uuid::new_v4(),
            generation: 1,
            spaces: vec![Uuid::new_v4()],
        };
        profile.validate().unwrap();
        let encoded = serde_json::to_value(&profile).unwrap();
        assert_eq!(encoded.as_object().unwrap().len(), 5);
        let decoded: Profile = serde_json::from_value(encoded.clone()).unwrap();
        decoded.validate().unwrap();
        for field in ["credential", "invitation", "capability", "private_key"] {
            let mut injected = encoded.clone();
            injected[field] = serde_json::json!("synthetic");
            assert!(serde_json::from_value::<Profile>(injected).is_err());
        }
        for alias in ["-F", "host;command", "host name", "../host"] {
            assert!(Profile {
                alias: alias.into(),
                ..profile.clone()
            }
            .validate()
            .is_err());
        }
        assert!(Profile {
            coordinator: Uuid::nil(),
            ..profile.clone()
        }
        .validate()
        .is_err());
        assert!(Profile {
            device: Uuid::nil(),
            ..profile.clone()
        }
        .validate()
        .is_err());
        assert!(Profile {
            generation: 0,
            ..profile.clone()
        }
        .validate()
        .is_err());
        for spaces in [
            vec![],
            vec![Uuid::nil()],
            vec![profile.spaces[0]; 2],
            (0..33).map(|_| Uuid::new_v4()).collect(),
        ] {
            assert!(Profile {
                spaces,
                ..profile.clone()
            }
            .validate()
            .is_err());
        }
        let old: setup::Preferences = serde_json::from_value(serde_json::json!({
            "enabled": false, "selected": {},
        }))
        .unwrap();
        assert!(old.remote_profiles.is_empty());
        let (cancel, mut cancelled) = tokio::sync::watch::channel(false);
        drop(cancel);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            assert!(
                tokio::time::timeout(std::time::Duration::from_secs(1), cancelled.changed())
                    .await
                    .unwrap()
                    .is_err()
            );
        });
    }
}
