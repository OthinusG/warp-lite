//! Enrollment storage only. SSH and a trusted app gateway must bind these principals.
use super::*;
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub(crate) struct RemotePrincipal {
    pub device: String,
    pub generation: u64,
}

fn secret(id: &str) -> Result<String> {
    let mut random = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut random)
        .map_err(|_| crate::coordinator_unavailable("OS randomness unavailable"))?;
    let suffix: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!("{id}.{suffix}"))
}
fn verifier(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
fn secret_id(value: &str) -> Result<&str> {
    ensure!(
        value.len() == 101,
        unauthorized("Invalid enrollment credential")
    );
    let (id, suffix) = value
        .split_once('.')
        .ok_or_else(|| unauthorized("Invalid enrollment credential"))?;
    ensure!(
        Uuid::parse_str(id).is_ok()
            && suffix.len() == 64
            && suffix.bytes().all(|byte| byte.is_ascii_hexdigit()),
        unauthorized("Invalid enrollment credential")
    );
    Ok(id)
}
fn matches_verifier(value: &str, expected: &str) -> bool {
    verifier(value).as_bytes().ct_eq(expected.as_bytes()).into()
}

impl Store {
    /// Persist only a hash and safe receipt. Dedup replay never reissues or stores the invitation secret.
    pub(super) fn create_invitation(
        &self,
        project: &str,
        operation: &ControllerOperation,
    ) -> Result<Value> {
        let ControllerOperation::InvitationCreate {
            space_ids,
            ttl_seconds,
            request_id,
        } = operation
        else {
            return Err(invalid_input("Expected invitation creation"));
        };
        ensure!(
            Uuid::parse_str(request_id).is_ok(),
            invalid_input("request_id must be a UUID")
        );
        let ttl = ttl_seconds.unwrap_or(300);
        ensure!(
            (1..=300).contains(&ttl)
                && !space_ids.is_empty()
                && space_ids.len() <= 32
                && space_ids.iter().collect::<HashSet<_>>().len() == space_ids.len(),
            invalid_input(
                "Invitation requires 1-32 unique spaces and a lifetime of at most five minutes"
            )
        );
        for space in space_ids {
            self.space(space)?
                .ok_or_else(|| scope_denied("Space not found"))?;
        }
        let mut issued = None;
        let serialized = serde_json::to_string(operation)?;
        let mut receipt = self.execute_mutation(&Self::operator(project).id, OPERATOR_EPOCH, request_id, &serialized, true, || {
            self.budget_available()?;
            diesel::sql_query("DELETE FROM invitations WHERE expires_at <= ? OR consumed != 0")
                .bind::<BigInt, _>(now() as i64).execute(&mut *self.connection.borrow_mut())?;
            ensure!(self.count("SELECT COUNT(*) AS count FROM invitations", &[])? < 1000, capacity_exceeded("Invitation capacity reached"));
            let id = Uuid::new_v4().to_string();
            let code = secret(&id)?;
            let expires_at = now() + ttl * 1000;
            diesel::sql_query("INSERT INTO invitations(id, verifier, space_ids, expires_at, consumed) VALUES (?,?,?,?,0)")
                .bind::<Text, _>(&id).bind::<Text, _>(verifier(&code))
                .bind::<Text, _>(serde_json::to_string(space_ids)?).bind::<BigInt, _>(expires_at as i64)
                .execute(&mut *self.connection.borrow_mut())?;
            self.record(project, "invitation_created", OPERATOR_EPOCH, Some(&id), None, json!({"space_ids": space_ids, "expires_at": expires_at}))?;
            issued = Some(code);
            Ok(json!({"invitation_id": id, "space_ids": space_ids, "expires_at": expires_at}))
        })?;
        receipt["invitation"] = json!(issued);
        receipt["secret_available"] = json!(receipt["invitation"].is_string());
        Ok(receipt)
    }

    /// Called only after verified SSH negotiation; credential delivery is in memory, once.
    pub(crate) fn enroll_remote(&self, invitation: &str, name: &str) -> Result<Value> {
        let id = secret_id(invitation)?;
        ensure!(
            !name.trim().is_empty() && name.len() <= 64 && !name.chars().any(char::is_control),
            invalid_input("Device name must contain 1-64 printable bytes")
        );
        self.transaction(|| {
            let row = diesel::sql_query("SELECT verifier, space_ids, expires_at, consumed FROM invitations WHERE id=?")
                .bind::<Text, _>(id).get_result::<InvitationRow>(&mut *self.connection.borrow_mut()).optional()?
                .ok_or_else(|| unauthorized("Invitation is unavailable or expired"))?;
            ensure!(row.consumed == 0 && row.expires_at as u64 > now() && matches_verifier(invitation, &row.verifier), unauthorized("Invitation is unavailable or expired"));
            ensure!(self.count("SELECT COUNT(*) AS count FROM devices", &[])? < 1000, capacity_exceeded("Device capacity reached"));
            let spaces: Vec<String> = serde_json::from_str(&row.space_ids).map_err(|_| invalid_state("Invitation grants are unavailable"))?;
            for space in &spaces { self.space(space)?.ok_or_else(|| scope_denied("Space not found"))?; }
            let device = Uuid::new_v4().to_string();
            let credential = secret(&device)?;
            diesel::sql_query("INSERT INTO devices(id,name,verifier,generation,revoked,created_at) VALUES (?,?,?,1,0,?)")
                .bind::<Text, _>(&device).bind::<Text, _>(name).bind::<Text, _>(verifier(&credential)).bind::<BigInt, _>(now() as i64)
                .execute(&mut *self.connection.borrow_mut())?;
            for space in &spaces {
                diesel::sql_query("INSERT INTO device_spaces(device_id,space_id,mode) VALUES (?,?,'write')")
                    .bind::<Text, _>(&device).bind::<Text, _>(space).execute(&mut *self.connection.borrow_mut())?;
                self.record(&format!("space:{space}"), "device_enrolled", OPERATOR_EPOCH, Some(&device), None, json!({"device_id": device}))?;
            }
            diesel::sql_query("UPDATE invitations SET consumed=1 WHERE id=?").bind::<Text, _>(id).execute(&mut *self.connection.borrow_mut())?;
            Ok(json!({"device_id": device, "credential": credential, "generation": 1, "space_ids": spaces}))
        })
    }

    pub(crate) fn authenticate_remote(&self, credential: &str) -> Result<RemotePrincipal> {
        let device = secret_id(credential)?;
        let row = self
            .device_row(device)?
            .ok_or_else(|| unauthorized("Device authentication failed"))?;
        ensure!(
            row.revoked == 0 && row.generation >= 1 && matches_verifier(credential, &row.verifier),
            unauthorized("Device authentication failed")
        );
        Ok(RemotePrincipal {
            device: row.id,
            generation: row.generation as u64,
        })
    }

    /// Revalidate durable revocation and grants on every operation, not just on connection setup.
    pub(crate) fn authorize_remote(
        &self,
        principal: &RemotePrincipal,
        space: &str,
        write: bool,
    ) -> Result<()> {
        let row = self.device_row(&principal.device)?.ok_or_else(|| {
            crate::domain(
                "device_revoked",
                "Device participation was revoked",
                false,
                None,
            )
        })?;
        ensure!(
            row.revoked == 0 && row.generation as u64 == principal.generation,
            crate::domain(
                "device_revoked",
                "Device participation was revoked",
                false,
                None
            )
        );
        let mode = diesel::sql_query(
            "SELECT mode AS value FROM device_spaces WHERE device_id=? AND space_id=?",
        )
        .bind::<Text, _>(&principal.device)
        .bind::<Text, _>(space)
        .get_result::<ValueRow>(&mut *self.connection.borrow_mut())
        .optional()?;
        ensure!(
            mode.is_some_and(|mode| mode.value == "write" || (!write && mode.value == "read")),
            scope_denied("Device is not granted this operation in the selected space")
        );
        self.space(space)?
            .ok_or_else(|| scope_denied("Space not found"))?;
        Ok(())
    }

    pub(crate) fn invalidate_remote_sessions(&self) -> Result<()> {
        ensure!(
            self.count(
                "SELECT COUNT(*) AS count FROM devices WHERE generation=9223372036854775807",
                &[]
            )? == 0,
            invalid_state("Device generation exhausted")
        );
        diesel::sql_query("UPDATE devices SET generation=generation+1 WHERE revoked=0")
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    pub(crate) fn remote_grants(&self, principal: &RemotePrincipal) -> Result<Vec<Uuid>> {
        let rows = diesel::sql_query(
            "SELECT space_id AS value FROM device_spaces WHERE device_id=? ORDER BY space_id",
        )
        .bind::<Text, _>(&principal.device)
        .load::<ValueRow>(&mut *self.connection.borrow_mut())?;
        let mut spaces = Vec::new();
        for row in rows {
            self.authorize_remote(principal, &row.value, false)?;
            spaces.push(
                Uuid::parse_str(&row.value)
                    .map_err(|_| invalid_state("Device grants unavailable"))?,
            );
        }
        ensure!(
            !spaces.is_empty(),
            scope_denied("Device has no granted spaces")
        );
        Ok(spaces)
    }

    fn device_row(&self, id: &str) -> Result<Option<DeviceRow>> {
        Ok(
            diesel::sql_query("SELECT id,name,verifier,generation,revoked FROM devices WHERE id=?")
                .bind::<Text, _>(id)
                .get_result::<DeviceRow>(&mut *self.connection.borrow_mut())
                .optional()?,
        )
    }
    pub(super) fn update_device_grant(
        &self,
        project: &str,
        device: &str,
        expected_generation: u64,
        space: &str,
        mode: Option<&str>,
    ) -> Result<Value> {
        ensure!(
            Uuid::parse_str(device).is_ok_and(|id| !id.is_nil())
                && Uuid::parse_str(space).is_ok_and(|id| !id.is_nil())
                && mode.is_none_or(|mode| matches!(mode, "read" | "write")),
            invalid_input("Review a device, space and read/write grant or removal")
        );
        let row = self
            .device_row(device)?
            .ok_or_else(|| scope_denied("Device not found"))?;
        ensure!(
            row.revoked == 0,
            scope_denied("Revoked devices require new enrollment")
        );
        ensure!(
            row.generation as u64 == expected_generation,
            version_conflict(
                "Device grants changed; review the current generation",
                row.generation as u64
            )
        );
        ensure!(
            row.generation < i64::MAX,
            invalid_state("Device generation exhausted")
        );
        self.space(space)?
            .ok_or_else(|| scope_denied("Space not found"))?;
        self.budget_available()?;
        if let Some(mode) = mode {
            ensure!(
                self.count(
                    "SELECT COUNT(*) AS count FROM device_spaces WHERE device_id=? AND space_id=?",
                    &[device, space]
                )? != 0
                    || self.count(
                        "SELECT COUNT(*) AS count FROM device_spaces WHERE device_id=?",
                        &[device]
                    )? < 32,
                capacity_exceeded("Device already has 32 granted spaces")
            );
            diesel::sql_query("INSERT INTO device_spaces(device_id,space_id,mode) VALUES (?,?,?) ON CONFLICT(device_id,space_id) DO UPDATE SET mode=excluded.mode")
                .bind::<Text,_>(device).bind::<Text,_>(space).bind::<Text,_>(mode)
                .execute(&mut *self.connection.borrow_mut())?;
        } else {
            diesel::sql_query("DELETE FROM device_spaces WHERE device_id=? AND space_id=?")
                .bind::<Text, _>(device)
                .bind::<Text, _>(space)
                .execute(&mut *self.connection.borrow_mut())?;
        }
        diesel::sql_query("UPDATE devices SET generation=generation+1 WHERE id=?")
            .bind::<Text, _>(device)
            .execute(&mut *self.connection.borrow_mut())?;
        self.record(project, "device_grant_changed", OPERATOR_EPOCH, Some(device), None,
            json!({"device_id":device,"space_id":space,"mode":mode,"generation":row.generation+1,"execution_stopped":false}))?;
        Ok(json!({"device_id":device,"space_id":space,"mode":mode,"generation":row.generation+1}))
    }

    pub(super) fn device_list(&self) -> Result<Value> {
        let rows = diesel::sql_query(
            "SELECT id,name,verifier,generation,revoked FROM devices ORDER BY created_at,id",
        )
        .load::<DeviceRow>(&mut *self.connection.borrow_mut())?;
        let mut devices = Vec::new();
        for row in rows {
            let spaces = diesel::sql_query(
                "SELECT space_id,mode FROM device_spaces WHERE device_id=? ORDER BY space_id",
            )
            .bind::<Text, _>(&row.id)
            .load::<DeviceGrant>(&mut *self.connection.borrow_mut())?;
            devices.push(json!({"id": row.id, "name": row.name, "generation": row.generation, "revoked": row.revoked != 0,
                "space_ids": spaces.iter().map(|grant| &grant.space_id).collect::<Vec<_>>(),
                "grants": spaces.iter().map(|grant| json!({"space_id":grant.space_id,"mode":grant.mode})).collect::<Vec<_>>()}));
        }
        Ok(json!({"devices": devices}))
    }
    pub(super) fn revoke_device(&self, project: &str, device: &str) -> Result<Value> {
        let row = self
            .device_row(device)?
            .ok_or_else(|| scope_denied("Device not found"))?;
        ensure!(
            row.generation < i64::MAX,
            invalid_state("Device generation exhausted")
        );
        diesel::sql_query("UPDATE devices SET revoked=1,generation=generation+1 WHERE id=?")
            .bind::<Text, _>(device)
            .execute(&mut *self.connection.borrow_mut())?;
        self.record(
            project,
            "device_revoked",
            OPERATOR_EPOCH,
            Some(device),
            None,
            json!({"device_id": device, "generation": row.generation + 1}),
        )?;
        Ok(json!({"device_id": device, "generation": row.generation + 1, "revoked": true}))
    }
}

#[derive(QueryableByName)]
struct DeviceGrant {
    #[diesel(sql_type = Text)]
    space_id: String,
    #[diesel(sql_type = Text)]
    mode: String,
}

#[derive(QueryableByName)]
struct InvitationRow {
    #[diesel(sql_type = Text)]
    verifier: String,
    #[diesel(sql_type = Text)]
    space_ids: String,
    #[diesel(sql_type = BigInt)]
    expires_at: i64,
    #[diesel(sql_type = Integer)]
    consumed: i32,
}
#[derive(QueryableByName)]
struct DeviceRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    verifier: String,
    #[diesel(sql_type = BigInt)]
    generation: i64,
    #[diesel(sql_type = Integer)]
    revoked: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id() -> String {
        Uuid::new_v4().to_string()
    }
    #[test]
    fn reviewed_grant_updates_fence_old_connections_and_preserve_original_receipts() {
        let store = Store::open(":memory:").unwrap();
        let space = store
            .execute_controller(
                "/private",
                &ControllerOperation::SpaceCreate {
                    name: "Grant review".into(),
                    request_id: id(),
                },
            )
            .unwrap()["space_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let invitation = store
            .execute_controller(
                "/private",
                &ControllerOperation::InvitationCreate {
                    space_ids: vec![space.clone()],
                    ttl_seconds: None,
                    request_id: id(),
                },
            )
            .unwrap();
        let enrolled = store
            .enroll_remote(invitation["invitation"].as_str().unwrap(), "Participant")
            .unwrap();
        let credential = enrolled["credential"].as_str().unwrap();
        let principal = store.authenticate_remote(credential).unwrap();
        let update = ControllerOperation::DeviceGrantUpdate {
            device_id: principal.device.clone(),
            expected_generation: principal.generation,
            space_id: space.clone(),
            mode: Some("read".into()),
            request_id: id(),
        };
        let result = store.execute_controller("/private", &update).unwrap();
        assert_eq!(
            store.execute_controller("/private", &update).unwrap(),
            result
        );
        assert!(store.authorize_remote(&principal, &space, false).is_err());
        let current = store.authenticate_remote(credential).unwrap();
        assert_eq!(current.generation, principal.generation + 1);
        store.authorize_remote(&current, &space, false).unwrap();
        assert!(store.authorize_remote(&current, &space, true).is_err());
        let list = store
            .execute_controller("/private", &ControllerOperation::DeviceList)
            .unwrap();
        assert_eq!(list["devices"][0]["grants"][0]["mode"], "read");
        let grant = |generation, mode: Option<&str>| ControllerOperation::DeviceGrantUpdate {
            device_id: principal.device.clone(),
            expected_generation: generation,
            space_id: space.clone(),
            mode: mode.map(str::to_owned),
            request_id: id(),
        };
        assert!(store
            .execute_controller("/private", &grant(principal.generation, Some("write")))
            .is_err());
        assert!(store
            .execute_controller(
                "/private",
                &grant(current.generation, Some("administrator"))
            )
            .is_err());
        store.authorize_remote(&current, &space, false).unwrap();
        store
            .execute_controller("/private", &grant(current.generation, Some("write")))
            .unwrap();
        let write = store.authenticate_remote(credential).unwrap();
        store.authorize_remote(&write, &space, true).unwrap();
        store
            .execute_controller("/private", &grant(write.generation, None))
            .unwrap();
        let removed = store.authenticate_remote(credential).unwrap();
        assert!(store.authorize_remote(&removed, &space, false).is_err());
        assert!(store.remote_grants(&removed).is_err());
        assert!(store
            .execute_controller("/private", &ControllerOperation::DeviceList)
            .unwrap()["devices"][0]["grants"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(
            store.events("/private", None, Some(200)).unwrap()["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|event| event["kind"] == "device_grant_changed")
                .count(),
            3
        );
    }

    #[test]
    fn enrollment_secrets_are_single_use_and_never_persisted_in_receipts_or_events() {
        let store = Store::open(":memory:").unwrap();
        let space = store
            .execute_controller(
                "/private",
                &ControllerOperation::SpaceCreate {
                    name: "Remote fixtures".into(),
                    request_id: id(),
                },
            )
            .unwrap()["space_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let operation = ControllerOperation::InvitationCreate {
            space_ids: vec![space.clone()],
            ttl_seconds: None,
            request_id: id(),
        };
        let issued = store.execute_controller("/private", &operation).unwrap();
        let code = issued["invitation"].as_str().unwrap();
        assert_eq!(code.len(), 101);
        let replay = store.execute_controller("/private", &operation).unwrap();
        assert!(replay["invitation"].is_null());
        assert_eq!(replay["invitation_id"], issued["invitation_id"]);
        let mut wrong = code.as_bytes().to_vec();
        wrong[100] = if wrong[100] == b'a' { b'b' } else { b'a' };
        assert!(store
            .enroll_remote(std::str::from_utf8(&wrong).unwrap(), "Participant")
            .is_err());
        let enrolled = store.enroll_remote(code, "Participant").unwrap();
        let credential = enrolled["credential"].as_str().unwrap();
        assert!(credential != code);
        assert!(store.enroll_remote(code, "Again").is_err());
        let principal = store.authenticate_remote(credential).unwrap();
        store.authorize_remote(&principal, &space, true).unwrap();
        assert!(store.authorize_remote(&principal, &id(), false).is_err());
        for query in [
            "SELECT response AS value FROM requests",
            "SELECT payload AS value FROM events",
            "SELECT verifier AS value FROM invitations",
            "SELECT verifier AS value FROM devices",
        ] {
            let rows = diesel::sql_query(query)
                .load::<ValueRow>(&mut *store.connection.borrow_mut())
                .unwrap();
            assert!(rows
                .iter()
                .all(|row| !row.value.contains(code) && !row.value.contains(credential)));
        }
        let devices = store
            .execute_controller("/private", &ControllerOperation::DeviceList)
            .unwrap();
        assert!(!devices.to_string().contains("verifier"));
        let revoke = ControllerOperation::DeviceRevoke {
            device_id: principal.device.clone(),
            request_id: id(),
        };
        let receipt = store.execute_controller("/private", &revoke).unwrap();
        assert_eq!(
            store.execute_controller("/private", &revoke).unwrap(),
            receipt
        );
        assert!(store.authenticate_remote(credential).is_err());
        assert_eq!(
            store
                .authorize_remote(&principal, &space, false)
                .unwrap_err()
                .downcast_ref::<crate::DomainError>()
                .unwrap()
                .code,
            "device_revoked"
        );
    }

    #[test]
    fn expired_invitations_bad_inputs_and_read_only_grants_fail_closed() {
        let store = Store::open(":memory:").unwrap();
        let space = store
            .execute_controller(
                "/private",
                &ControllerOperation::SpaceCreate {
                    name: "Shared".into(),
                    request_id: id(),
                },
            )
            .unwrap()["space_id"]
            .as_str()
            .unwrap()
            .to_owned();
        for (spaces, ttl) in [
            (vec![], 300),
            (vec![space.clone(), space.clone()], 300),
            (vec![space.clone()], 301),
        ] {
            assert!(store
                .execute_controller(
                    "/private",
                    &ControllerOperation::InvitationCreate {
                        space_ids: spaces,
                        ttl_seconds: Some(ttl),
                        request_id: id()
                    }
                )
                .is_err());
        }
        let issued = store
            .execute_controller(
                "/private",
                &ControllerOperation::InvitationCreate {
                    space_ids: vec![space.clone()],
                    ttl_seconds: None,
                    request_id: id(),
                },
            )
            .unwrap();
        let code = issued["invitation"].as_str().unwrap();
        assert!(store.enroll_remote(code, "\n").is_err());
        diesel::sql_query("UPDATE invitations SET expires_at=1")
            .execute(&mut *store.connection.borrow_mut())
            .unwrap();
        assert!(store.enroll_remote(code, "Participant").is_err());
        let fresh = store
            .execute_controller(
                "/private",
                &ControllerOperation::InvitationCreate {
                    space_ids: vec![space.clone()],
                    ttl_seconds: None,
                    request_id: id(),
                },
            )
            .unwrap();
        let credential = store
            .enroll_remote(fresh["invitation"].as_str().unwrap(), "Participant")
            .unwrap();
        let principal = store
            .authenticate_remote(credential["credential"].as_str().unwrap())
            .unwrap();
        diesel::sql_query("UPDATE device_spaces SET mode='read'")
            .execute(&mut *store.connection.borrow_mut())
            .unwrap();
        store.authorize_remote(&principal, &space, false).unwrap();
        assert!(store.authorize_remote(&principal, &space, true).is_err());
        for bad in ["", "invalid", "\n", code] {
            assert!(store.authenticate_remote(bad).is_err());
        }
        let mut unique = HashSet::new();
        let subject = id();
        for _ in 0..100 {
            assert!(unique.insert(secret(&subject).unwrap()));
        }
    }
}
