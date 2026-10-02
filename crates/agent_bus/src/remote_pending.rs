//! Participant receipts are durable intents, never a second task engine or credential store.
use super::*;

const MAX_ROWS: i64 = 1000;
const MAX_BYTES: i64 = 16 * 1024 * 1024;
const COLUMNS: &str = "coordinator, device, space, actor, epoch, request_id, operation, response";

#[derive(Clone, QueryableByName)]
pub struct RemoteIntent {
    #[diesel(sql_type = Text)]
    pub coordinator: String,
    #[diesel(sql_type = Text)]
    pub device: String,
    #[diesel(sql_type = Text)]
    pub space: String,
    #[diesel(sql_type = Text)]
    pub actor: String,
    #[diesel(sql_type = Text)]
    pub epoch: String,
    #[diesel(sql_type = Text)]
    pub request_id: String,
    #[diesel(sql_type = Text)]
    pub operation: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub response: Option<String>,
}

impl Store {
    /// Persist the verified original mutation before any network write; unknown outcomes remain.
    pub fn stage_remote_intent(
        &self,
        coordinator: Uuid,
        device: Uuid,
        actor: &RemoteActor,
        operation: &Operation,
    ) -> Result<RemoteIntent> {
        let space = actor
            .actor
            .project
            .strip_prefix("space:")
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or_else(|| invalid_input("Remote space identity is required"))?;
        let native = actor
            .actor
            .terminal
            .strip_prefix(&format!("remote:{device}:"))
            .and_then(|id| Uuid::parse_str(id).ok());
        ensure!(
            !coordinator.is_nil()
                && !device.is_nil()
                && !space.is_nil()
                && native.is_some_and(|id| !id.is_nil())
                && Uuid::parse_str(&actor.actor.id).is_ok_and(|id| !id.is_nil())
                && Uuid::parse_str(&actor.epoch).is_ok_and(|id| !id.is_nil()),
            invalid_input("Original remote authority is required")
        );
        let request_id = operation
            .request_id()
            .ok_or_else(|| invalid_input("Only original mutations require durable intents"))?;
        ensure!(
            Uuid::parse_str(request_id).is_ok_and(|id| !id.is_nil()),
            invalid_input("Original request UUID is required")
        );
        let serialized = serde_json::to_string(operation)?;
        ensure!(
            serialized.len() < crate::MAX_FRAME / 2,
            capacity_exceeded("Remote intent exceeds the protocol budget")
        );
        let coordinator = coordinator.to_string();
        let device = device.to_string();
        let space = space.to_string();
        self.transaction(|| {
            if let Some(existing) = self.remote_intent(&coordinator, &actor.actor.id, request_id)? {
                ensure!(existing.device == device && existing.space == space
                    && existing.epoch == actor.epoch && existing.operation == serialized,
                    request_conflict("Original remote intent cannot change"));
                return Ok(existing);
            }
            self.budget_available()?;
            self.remote_intent_budget(serialized.len() as i64, true)?;
            diesel::sql_query("INSERT INTO remote_pending_intents(coordinator,device,space,actor,epoch,request_id,operation,created_at) VALUES (?,?,?,?,?,?,?,?)")
                .bind::<Text,_>(&coordinator).bind::<Text,_>(&device).bind::<Text,_>(&space)
                .bind::<Text,_>(&actor.actor.id).bind::<Text,_>(&actor.epoch)
                .bind::<Text,_>(request_id).bind::<Text,_>(&serialized)
                .bind::<BigInt,_>(now() as i64).execute(&mut *self.connection.borrow_mut())?;
            self.remote_intent(&coordinator, &actor.actor.id, request_id)?
                .ok_or_else(|| invalid_state("Durable remote intent is unavailable"))
        })
    }

    pub fn remote_intent(
        &self,
        coordinator: &str,
        actor: &str,
        request: &str,
    ) -> Result<Option<RemoteIntent>> {
        Ok(diesel::sql_query(format!("SELECT {COLUMNS} FROM remote_pending_intents WHERE coordinator=? AND actor=? AND request_id=?"))
            .bind::<Text,_>(coordinator).bind::<Text,_>(actor).bind::<Text,_>(request)
            .get_result::<RemoteIntent>(&mut *self.connection.borrow_mut()).optional()?)
    }

    /// Recover bounded pages after restart, without discarding unknown or expired outcomes.
    pub fn remote_intents(
        &self,
        coordinator: Uuid,
        device: Uuid,
        after: Option<(&str, &str)>,
    ) -> Result<Vec<RemoteIntent>> {
        let (actor, request) = after.unwrap_or(("", ""));
        Ok(diesel::sql_query(format!("SELECT {COLUMNS} FROM remote_pending_intents WHERE coordinator=? AND device=? AND (actor,request_id)>(?,?) ORDER BY actor,request_id LIMIT 50"))
            .bind::<Text,_>(coordinator.to_string()).bind::<Text,_>(device.to_string())
            .bind::<Text,_>(actor).bind::<Text,_>(request)
            .load::<RemoteIntent>(&mut *self.connection.borrow_mut())?)
    }

    /// Retain only a matched successful operation/committed reconciliation result.
    pub fn resolve_remote_intent(&self, intent: &RemoteIntent, result: &Value) -> Result<()> {
        let response = serde_json::to_string(result)?;
        ensure!(
            response.len() < crate::MAX_FRAME,
            capacity_exceeded("Remote receipt exceeds the protocol budget")
        );
        self.transaction(|| {
            let existing = self.remote_intent(&intent.coordinator, &intent.actor, &intent.request_id)?
                .ok_or_else(|| invalid_state("Original remote intent is unavailable"))?;
            ensure!(existing.device == intent.device && existing.space == intent.space
                && existing.epoch == intent.epoch && existing.operation == intent.operation,
                request_conflict("Receipt context does not match the original intent"));
            if let Some(prior) = existing.response {
                ensure!(prior == response, request_conflict("Confirmed receipt cannot change"));
                return Ok(());
            }
            self.budget_available()?;
            self.remote_intent_budget(response.len() as i64, false)?;
            diesel::sql_query("UPDATE remote_pending_intents SET response=? WHERE coordinator=? AND actor=? AND request_id=?")
                .bind::<Text,_>(response).bind::<Text,_>(&intent.coordinator)
                .bind::<Text,_>(&intent.actor).bind::<Text,_>(&intent.request_id)
                .execute(&mut *self.connection.borrow_mut())?;
            Ok(())
        })
    }

    /// Only explicit consumption of a retained result may remove an intent.
    pub fn forget_resolved_remote_intent(
        &self,
        coordinator: &str,
        actor: &str,
        request: &str,
    ) -> Result<()> {
        self.transaction(|| {
            let existing = self.remote_intent(coordinator, actor, request)?
                .ok_or_else(|| invalid_state("Original remote intent is unavailable"))?;
            ensure!(existing.response.is_some(), execution_unknown("Reconcile the pending intent before removal"));
            diesel::sql_query("DELETE FROM remote_pending_intents WHERE coordinator=? AND actor=? AND request_id=?")
                .bind::<Text,_>(coordinator).bind::<Text,_>(actor).bind::<Text,_>(request)
                .execute(&mut *self.connection.borrow_mut())?;
            Ok(())
        })
    }

    fn remote_intent_budget(&self, extra: i64, new_row: bool) -> Result<()> {
        ensure!(
            !new_row
                || self.count("SELECT COUNT(*) AS count FROM remote_pending_intents", &[])?
                    < MAX_ROWS,
            capacity_exceeded("Reconcile retained remote intents before sending more work")
        );
        ensure!(self.count("SELECT COALESCE(SUM(length(CAST(operation AS BLOB))+COALESCE(length(CAST(response AS BLOB)),0)),0) AS count FROM remote_pending_intents", &[])? + extra <= MAX_BYTES,
            capacity_exceeded("Remote intent storage is full; reconcile retained work"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(device: Uuid) -> RemoteActor {
        RemoteActor {
            actor: Agent {
                id: Uuid::new_v4().to_string(),
                terminal: format!("remote:{device}:{}", Uuid::new_v4()),
                name: "fixture".into(),
                program: "codex".into(),
                project: format!("space:{}", Uuid::new_v4()),
            },
            epoch: Uuid::new_v4().to_string(),
            expires_at: now() + 60000,
        }
    }
    fn mutation(body: &str) -> Operation {
        Operation::AgentSend {
            to: "receiver".into(),
            body: body.into(),
            subject: None,
            thread_id: None,
            reply_to: None,
            task_id: None,
            request_id: Uuid::new_v4().to_string(),
        }
    }

    #[test]
    fn original_pending_intent_and_receipt_survive_restart_and_cannot_change() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("participant.sqlite");
        let coordinator = Uuid::new_v4();
        let device = Uuid::new_v4();
        let actor = fixture(device);
        let operation = mutation("Original task intent");
        let store = Store::open(path.to_str().unwrap()).unwrap();
        let original = store
            .stage_remote_intent(coordinator, device, &actor, &operation)
            .unwrap();
        assert!(store
            .forget_resolved_remote_intent(
                &original.coordinator,
                &original.actor,
                &original.request_id
            )
            .is_err());
        drop(store);
        let store = Store::open(path.to_str().unwrap()).unwrap();
        let page = store.remote_intents(coordinator, device, None).unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].operation, original.operation);
        assert_eq!(page[0].epoch, original.epoch);
        assert!(page[0].response.is_none());
        assert!(store
            .remote_intents(coordinator, Uuid::new_v4(), None)
            .unwrap()
            .is_empty());
        let mut changed = operation.clone();
        if let Operation::AgentSend { body, .. } = &mut changed {
            *body = "Changed intent".into();
        }
        assert!(store
            .stage_remote_intent(coordinator, device, &actor, &changed)
            .is_err());
        let mut replacement = actor.clone();
        replacement.epoch = Uuid::new_v4().to_string();
        assert!(store
            .stage_remote_intent(coordinator, device, &replacement, &operation)
            .is_err());
        let result = json!({"task_id":"original-result"});
        store.resolve_remote_intent(&original, &result).unwrap();
        store.resolve_remote_intent(&original, &result).unwrap();
        assert!(store
            .resolve_remote_intent(&original, &json!({"task_id":"changed"}))
            .is_err());
        drop(store);
        let store = Store::open(path.to_str().unwrap()).unwrap();
        assert_eq!(
            store.remote_intents(coordinator, device, None).unwrap()[0].response,
            Some(result.to_string())
        );
        store
            .forget_resolved_remote_intent(
                &original.coordinator,
                &original.actor,
                &original.request_id,
            )
            .unwrap();
        assert!(store
            .remote_intents(coordinator, device, None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn participant_intent_limits_reject_new_work_without_evicting_pending_rows() {
        let store = Store::open(":memory:").unwrap();
        let coordinator = Uuid::new_v4();
        let device = Uuid::new_v4();
        let actor = fixture(device);
        let operation = mutation("bounded");
        let original = store
            .stage_remote_intent(coordinator, device, &actor, &operation)
            .unwrap();
        diesel::sql_query("WITH RECURSIVE n(value) AS (SELECT 1 UNION ALL SELECT value+1 FROM n WHERE value<999) INSERT INTO remote_pending_intents SELECT coordinator,device,space,actor,epoch,CAST(value AS TEXT),operation,NULL,created_at FROM remote_pending_intents CROSS JOIN n")
            .execute(&mut *store.connection.borrow_mut()).unwrap();
        assert!(store
            .stage_remote_intent(coordinator, device, &actor, &mutation("new"))
            .is_err());
        assert!(store
            .stage_remote_intent(coordinator, device, &actor, &operation)
            .is_ok());
        let page = store.remote_intents(coordinator, device, None).unwrap();
        assert_eq!(page.len(), 50);
        let last = page.last().unwrap();
        let next = store
            .remote_intents(coordinator, device, Some((&last.actor, &last.request_id)))
            .unwrap();
        assert_eq!(next.len(), 50);
        assert!(next.iter().all(|row| row.request_id > last.request_id));
        diesel::sql_query("DELETE FROM remote_pending_intents WHERE request_id!=?")
            .bind::<Text, _>(&original.request_id)
            .execute(&mut *store.connection.borrow_mut())
            .unwrap();
        diesel::sql_query("UPDATE remote_pending_intents SET response=printf('%.*c',?,'x')")
            .bind::<Integer, _>(MAX_BYTES as i32)
            .execute(&mut *store.connection.borrow_mut())
            .unwrap();
        assert!(store
            .stage_remote_intent(coordinator, device, &actor, &mutation("new"))
            .is_err());
        assert!(store
            .stage_remote_intent(
                coordinator,
                device,
                &actor,
                &mutation(&"x".repeat(crate::MAX_FRAME))
            )
            .is_err());
        assert_eq!(
            store
                .remote_intents(coordinator, device, None)
                .unwrap()
                .len(),
            1
        );
    }
}
