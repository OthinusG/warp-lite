//! Device-qualified workspace and run authority; participant paths are never host filesystem roots.
use super::*;

#[derive(Clone)]
pub(crate) struct RemoteWorkspace {
    pub id: String,
    pub device: String,
    pub space: String,
    pub checkout: String,
}
#[derive(Clone)]
pub struct RemoteActor {
    pub actor: Agent,
    pub epoch: String,
    pub expires_at: u64,
}
#[derive(QueryableByName)]
struct Binding {
    #[diesel(sql_type = Text)]
    agent: String,
    #[diesel(sql_type = Text)]
    device: String,
    #[diesel(sql_type = Text)]
    workspace_id: String,
    #[diesel(sql_type = Text)]
    space_id: String,
    #[diesel(sql_type = Text)]
    native_id: String,
    #[diesel(sql_type = Nullable<Text>)]
    current_epoch: Option<String>,
}
#[derive(QueryableByName)]
struct Run {
    #[diesel(sql_type = Text)]
    epoch: String,
    #[diesel(sql_type = BigInt)]
    expires_at: i64,
    #[diesel(sql_type = BigInt)]
    closed: i64,
}
const BINDING: &str = "agent, device, workspace_id, space_id, native_id, current_epoch";

impl Store {
    pub(crate) fn map_remote_workspace(
        &self,
        principal: &RemotePrincipal,
        space: Uuid,
        checkout: Uuid,
        label: &str,
        repository: Option<Uuid>,
    ) -> Result<RemoteWorkspace> {
        self.transaction(|| {
            self.map_remote_workspace_in_transaction(principal, space, checkout, label, repository)
        })
    }

    pub(super) fn map_remote_workspace_in_transaction(
        &self,
        principal: &RemotePrincipal,
        space: Uuid,
        checkout: Uuid,
        label: &str,
        repository: Option<Uuid>,
    ) -> Result<RemoteWorkspace> {
        ensure!(
            !space.is_nil() && !checkout.is_nil() && repository.is_none_or(|id| !id.is_nil()),
            invalid_input("Workspace identities must be non-nil UUIDs")
        );
        ensure!(
            !label.trim().is_empty() && label.len() <= 4096 && !label.chars().any(char::is_control),
            invalid_input("Workspace display metadata must contain 1-4096 printable bytes")
        );
        let space = space.to_string();
        let checkout = checkout.to_string();
        self.authorize_remote(principal, &space, true)?;
        self.budget_available()?;
        let prior = diesel::sql_query(
            "SELECT id AS value FROM remote_workspaces WHERE device=? AND checkout=?",
        )
        .bind::<Text, _>(&principal.device)
        .bind::<Text, _>(&checkout)
        .get_result::<ValueRow>(&mut *self.connection.borrow_mut())
        .optional()?;
        ensure!(
            prior.is_some()
                || self.count("SELECT COUNT(*) AS count FROM remote_workspaces", &[])? < 1000,
            capacity_exceeded("Remote workspace capacity reached")
        );
        let id = prior
            .map(|row| row.value)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let actors = diesel::sql_query(format!(
            "SELECT {BINDING} FROM remote_actor_bindings WHERE workspace_id=? AND space_id!=?"
        ))
        .bind::<Text, _>(&id)
        .bind::<Text, _>(&space)
        .load::<Binding>(&mut *self.connection.borrow_mut())?;
        for actor in actors {
            self.revoke_remote_actor(&actor.agent, &actor.space_id)?;
        }
        diesel::sql_query("INSERT INTO remote_workspaces(id,device,space_id,checkout,label,repository_id,created_at) VALUES (?,?,?,?,?,?,?) ON CONFLICT(device,checkout) DO UPDATE SET space_id=excluded.space_id,label=excluded.label,repository_id=excluded.repository_id")
                .bind::<Text, _>(&id).bind::<Text, _>(&principal.device).bind::<Text, _>(&space)
                .bind::<Text, _>(&checkout).bind::<Text, _>(label)
                .bind::<Nullable<Text>, _>(repository.map(|id| id.to_string()))
                .bind::<BigInt, _>(now() as i64).execute(&mut *self.connection.borrow_mut())?;
        self.record(
            &format!("space:{space}"),
            "remote_workspace_mapped",
            &principal.device,
            Some(&id),
            None,
            json!({"device_id":principal.device,"workspace_id":id,"checkout_id":checkout}),
        )?;
        Ok(RemoteWorkspace {
            id,
            device: principal.device.clone(),
            space,
            checkout,
        })
    }

    fn remote_binding(&self, agent: &str) -> Result<Option<Binding>> {
        Ok(diesel::sql_query(format!(
            "SELECT {BINDING} FROM remote_actor_bindings WHERE agent=?"
        ))
        .bind::<Text, _>(agent)
        .get_result::<Binding>(&mut *self.connection.borrow_mut())
        .optional()?)
    }

    fn check_remote_workspace(
        &self,
        principal: &RemotePrincipal,
        workspace: &RemoteWorkspace,
    ) -> Result<()> {
        ensure!(
            workspace.device == principal.device,
            scope_denied("Workspace belongs to another device")
        );
        self.authorize_remote(principal, &workspace.space, true)?;
        ensure!(self.count("SELECT COUNT(*) AS count FROM remote_workspaces WHERE id=? AND device=? AND space_id=? AND checkout=?",
            &[&workspace.id, &principal.device, &workspace.space, &workspace.checkout])? == 1,
            scope_denied("Remote workspace mapping changed"));
        Ok(())
    }

    pub(crate) fn register_remote_actor(
        &self,
        principal: &RemotePrincipal,
        workspace: &RemoteWorkspace,
        native: Uuid,
        native_run: Uuid,
        program: &str,
        name: &str,
    ) -> Result<RemoteActor> {
        self.check_remote_workspace(principal, workspace)?;
        ensure!(
            !native.is_nil() && !native_run.is_nil(),
            invalid_input("Native identities must be non-nil UUIDs")
        );
        let domain = format!("space:{}", workspace.space);
        let native = native.to_string();
        let native_run = native_run.to_string();
        self.transaction(|| {
            let by_native = diesel::sql_query(format!("SELECT {BINDING} FROM remote_actor_bindings WHERE device=? AND native_id=?"))
                .bind::<Text, _>(&principal.device).bind::<Text, _>(&native)
                .get_result::<Binding>(&mut *self.connection.borrow_mut()).optional()?;
            if let Some(binding) = &by_native {
                ensure!(binding.workspace_id == workspace.id && binding.space_id == workspace.space,
                    scope_denied("Native identity belongs to another workspace"));
            }
            if let Some(existing) = self.agent_by_name(&domain, name)? {
                let owned = self.remote_binding(&existing.id)?;
                ensure!(owned.is_some_and(|binding| binding.device == principal.device
                    && binding.workspace_id == workspace.id && binding.space_id == workspace.space
                    && binding.native_id == native), scope_denied("Name belongs to another identity"));
            }
            let actor = self.register_inner(&format!("remote:{}:{native}", principal.device), program, &domain, name)?;
            if let Some(binding) = &by_native {
                ensure!(binding.agent == actor.id, scope_denied("Native identity cannot be rebound"));
            }
            let previous = diesel::sql_query("SELECT epoch,expires_at,closed FROM remote_runs WHERE agent=? AND native_run=?")
                .bind::<Text, _>(&actor.id).bind::<Text, _>(&native_run)
                .get_result::<Run>(&mut *self.connection.borrow_mut()).optional()?;
            let (epoch, expires_at, fresh) = if let Some(run) = previous {
                ensure!(run.closed == 0 && run.expires_at as u64 > now()
                    && by_native.as_ref().and_then(|binding| binding.current_epoch.as_ref()) == Some(&run.epoch),
                    epoch_expired("Original remote run is closed or expired"));
                (run.epoch, run.expires_at as u64, false)
            } else {
                // ponytail: retain up to 10k run tombstones; add durable compaction fences before pruning.
                ensure!(self.count("SELECT COUNT(*) AS count FROM remote_runs", &[])? < 10_000,
                    capacity_exceeded("Remote run capacity reached"));
                (Uuid::new_v4().to_string(), now() + crate::MUTATION_EPOCH.as_millis() as u64, true)
            };
            diesel::sql_query("INSERT INTO remote_actor_bindings(agent,device,workspace_id,space_id,native_id,current_epoch,revoked) VALUES (?,?,?,?,?,?,0) ON CONFLICT(agent) DO UPDATE SET current_epoch=excluded.current_epoch,revoked=0")
                .bind::<Text, _>(&actor.id).bind::<Text, _>(&principal.device).bind::<Text, _>(&workspace.id)
                .bind::<Text, _>(&workspace.space).bind::<Text, _>(&native).bind::<Text, _>(&epoch)
                .execute(&mut *self.connection.borrow_mut())?;
            self.space_join(&domain, &Self::operator(&domain), &workspace.space, &actor.id)?;
            if fresh {
                diesel::sql_query("UPDATE remote_runs SET closed=1 WHERE agent=?")
                    .bind::<Text, _>(&actor.id).execute(&mut *self.connection.borrow_mut())?;
                diesel::sql_query("INSERT INTO remote_runs(epoch,agent,native_run,expires_at,closed) VALUES (?,?,?,?,0)")
                    .bind::<Text, _>(&epoch).bind::<Text, _>(&actor.id).bind::<Text, _>(&native_run)
                    .bind::<BigInt, _>(expires_at as i64).execute(&mut *self.connection.borrow_mut())?;
                self.recover_in_transaction(&actor, &epoch)?;
            }
            Ok(RemoteActor { actor, epoch, expires_at })
        })
    }

    pub(crate) fn resolve_remote_run(
        &self,
        principal: &RemotePrincipal,
        agent: &str,
        epoch: &str,
        write: bool,
    ) -> Result<Agent> {
        let binding = self
            .remote_binding(agent)?
            .ok_or_else(|| scope_denied("Remote actor not found"))?;
        ensure!(
            binding.device == principal.device,
            scope_denied("Actor belongs to another device")
        );
        self.authorize_remote(principal, &binding.space_id, write)?;
        ensure!(
            binding.current_epoch.as_deref() == Some(epoch),
            epoch_expired("Remote run was replaced")
        );
        ensure!(self.count("SELECT COUNT(*) AS count FROM remote_runs WHERE agent=? AND epoch=? AND closed=0 AND expires_at>?",
            &[agent, epoch, &now().to_string()])? == 1, epoch_expired("Remote run is closed or expired"));
        let actor = self.agent(&format!("space:{}", binding.space_id), agent)?;
        self.authorize(&actor)?;
        Ok(actor)
    }

    /// Connectivity loss changes certainty only; it never confirms stopped effects or a task outcome.
    pub(crate) fn observe_remote_disconnect(&self, agent: &str, epoch: &str) -> Result<()> {
        let binding = self
            .remote_binding(agent)?
            .ok_or_else(|| scope_denied("Remote actor not found"))?;
        self.transaction(|| {
            let attempts = diesel::sql_query("SELECT id,task_id,revision,owner,run,certainty,outcome,started_at,finished_at FROM attempts WHERE owner=? AND run=? AND certainty='active' AND outcome IS NULL AND finished_at IS NULL")
                .bind::<Text,_>(agent).bind::<Text,_>(epoch)
                .load::<AttemptRow>(&mut *self.connection.borrow_mut())?;
            for attempt in attempts.into_iter().map(AttemptRow::attempt) {
                let mut task = self.operator_task(&format!("space:{}", binding.space_id), &attempt.task_id)?;
                if !matches!(task.state.as_str(), "running" | "cancel_requested") { continue; }
                diesel::sql_query("UPDATE attempts SET certainty='unknown' WHERE id=?")
                    .bind::<Text,_>(&attempt.id).execute(&mut *self.connection.borrow_mut())?;
                task.version += 1;
                self.update_task_state(&task)?;
                self.record(&task.project, "task_execution_unknown", agent, Some(&task.id), Some(&attempt.id),
                    json!({"revision":task.revision,"reason":"remote_presence_lost","execution_stopped":false}))?;
            }
            Ok(())
        })
    }

    /// Read an original receipt without executing it, including after that run was closed.
    pub(crate) fn reconcile_remote_request(
        &self,
        principal: &RemotePrincipal,
        agent: &str,
        epoch: &str,
        operation: &Operation,
    ) -> Result<Value> {
        let binding = self
            .remote_binding(agent)?
            .ok_or_else(|| scope_denied("Remote actor not found"))?;
        ensure!(
            binding.device == principal.device,
            scope_denied("Actor belongs to another device")
        );
        self.authorize_remote(principal, &binding.space_id, false)?;
        let actor = self.agent(&format!("space:{}", binding.space_id), agent)?;
        self.authorize(&actor)?;
        ensure!(
            self.count(
                "SELECT COUNT(*) AS count FROM remote_runs WHERE agent=? AND epoch=?",
                &[agent, epoch]
            )? == 1,
            epoch_expired("Original remote run is unavailable")
        );
        let request_id = operation
            .request_id()
            .ok_or_else(|| invalid_input("Reconciliation requires an original mutation"))?;
        ensure!(
            Uuid::parse_str(request_id).is_ok(),
            invalid_input("Original request UUID is required")
        );
        if let Some(row) = self.request_row(agent, request_id)? {
            ensure!(
                row.epoch == epoch,
                epoch_expired("Receipt belongs to another original run")
            );
            ensure!(
                row.fingerprint == serde_json::to_string(operation)?,
                request_conflict("Original request content does not match its receipt")
            );
            return Ok(
                json!({"status":"committed", "result":serde_json::from_str::<Value>(&row.response)?}),
            );
        }
        // An absent receipt permits only replay under this still-authorized original epoch.
        self.resolve_remote_run(principal, agent, epoch, true)?;
        Ok(json!({"status":"not_committed"}))
    }

    pub(super) fn remote_authorized(&self, actor: &Agent) -> Result<Option<bool>> {
        if self.remote_binding(&actor.id)?.is_none() {
            return Ok(None);
        }
        Ok(Some(self.count("SELECT COUNT(*) AS count FROM remote_actor_bindings AS b JOIN remote_workspaces AS w ON w.id=b.workspace_id AND w.device=b.device AND w.space_id=b.space_id JOIN devices AS d ON d.id=b.device AND d.revoked=0 JOIN device_spaces AS g ON g.device_id=d.id AND g.space_id=b.space_id JOIN space_members AS m ON m.agent=b.agent AND m.space_id=b.space_id WHERE b.agent=? AND b.revoked=0 AND ?='space:' || b.space_id AND g.mode IN ('read','write')",
            &[&actor.id, &actor.project])? == 1))
    }

    pub(super) fn revoke_remote_actor(&self, agent: &str, space: &str) -> Result<()> {
        diesel::sql_query("UPDATE remote_runs SET closed=1 WHERE agent IN (SELECT agent FROM remote_actor_bindings WHERE agent=? AND space_id=?)")
            .bind::<Text, _>(agent).bind::<Text, _>(space).execute(&mut *self.connection.borrow_mut())?;
        diesel::sql_query("UPDATE remote_actor_bindings SET revoked=1,current_epoch=NULL WHERE agent=? AND space_id=?")
            .bind::<Text, _>(agent).bind::<Text, _>(space).execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    pub(super) fn remote_physical_root(&self, actor: &Agent) -> Result<Option<String>> {
        Ok(diesel::sql_query("SELECT 'remote:' || b.device || ':' || w.checkout AS value FROM remote_actor_bindings AS b JOIN remote_workspaces AS w ON w.id=b.workspace_id WHERE b.agent=?")
            .bind::<Text, _>(&actor.id).get_result::<ValueRow>(&mut *self.connection.borrow_mut()).optional()?
            .map(|row| row.value))
    }

    pub(super) fn reservation_path(
        &self,
        actor: &Agent,
        workspace: &str,
        path: &str,
    ) -> Result<String> {
        if self.remote_physical_root(actor)?.is_some() {
            normalize_relative_path(path)
        } else {
            crate::normalize_workspace_path(workspace, path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id() -> String {
        Uuid::new_v4().to_string()
    }
    fn space(store: &Store) -> Uuid {
        Uuid::parse_str(
            store
                .execute_controller(
                    "/coordinator",
                    &ControllerOperation::SpaceCreate {
                        name: id(),
                        request_id: id(),
                    },
                )
                .unwrap()["space_id"]
                .as_str()
                .unwrap(),
        )
        .unwrap()
    }
    fn device(store: &Store, spaces: &[Uuid]) -> RemotePrincipal {
        let issued = store
            .execute_controller(
                "/coordinator",
                &ControllerOperation::InvitationCreate {
                    space_ids: spaces.iter().map(ToString::to_string).collect(),
                    ttl_seconds: None,
                    request_id: id(),
                },
            )
            .unwrap();
        let enrolled = store
            .enroll_remote(issued["invitation"].as_str().unwrap(), "Participant")
            .unwrap();
        store
            .authenticate_remote(enrolled["credential"].as_str().unwrap())
            .unwrap()
    }
    fn reserve() -> Operation {
        Operation::FileReserve {
            paths: vec!["fixtures/source.txt".into()],
            mode: "exclusive".into(),
            task_id: None,
            attempt_id: None,
            ttl_seconds: None,
            request_id: id(),
        }
    }

    #[test]
    fn remote_disconnect_preserves_unknown_effects_without_finishing_the_task() {
        let store = Store::open(":memory:").unwrap();
        let space = space(&store);
        let principal = device(&store, &[space]);
        let workspace = store
            .map_remote_workspace(
                &principal,
                space,
                Uuid::new_v4(),
                "Participant checkout",
                None,
            )
            .unwrap();
        let actor = store
            .register_remote_actor(
                &principal,
                &workspace,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "codex",
                "disconnect-worker",
            )
            .unwrap();
        let domain = format!("space:{space}");
        let task = store
            .execute(
                &Store::operator(&domain),
                OPERATOR_EPOCH,
                &Operation::TaskAssign {
                    to: actor.actor.name.clone(),
                    description: "Disconnected execution".into(),
                    acceptance: "Unknown effects remain visible".into(),
                    reviewer: None,
                    dependencies: vec![],
                    start_deadline: None,
                    execution_timeout_seconds: None,
                    review_timeout_seconds: None,
                    request_id: id(),
                },
            )
            .unwrap();
        let task_id = task["id"].as_str().unwrap();
        store
            .execute(
                &actor.actor,
                &actor.epoch,
                &Operation::TaskStart {
                    task_id: task_id.into(),
                    revision: 1,
                    expected_version: None,
                    request_id: id(),
                },
            )
            .unwrap();
        let before = store.operator_task(&domain, task_id).unwrap();
        store
            .observe_remote_disconnect(&actor.actor.id, &actor.epoch)
            .unwrap();
        store
            .observe_remote_disconnect(&actor.actor.id, &actor.epoch)
            .unwrap();
        let after = store.operator_task(&domain, task_id).unwrap();
        assert_eq!(after.state, "running");
        assert_eq!(after.version, before.version + 1);
        assert_eq!(after.attempts[0].certainty, "unknown");
        assert!(after.attempts[0].finished_at.is_none());
        assert!(after.executing_run.is_none());
        assert!(store
            .execute_controller(
                &domain,
                &ControllerOperation::TaskArchive {
                    task_id: task_id.into(),
                    request_id: id(),
                }
            )
            .is_err());
        assert_eq!(
            store.events(&domain, None, Some(200)).unwrap()["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|event| event["kind"] == "task_execution_unknown")
                .count(),
            1
        );
    }

    #[test]
    fn original_receipt_survives_run_replacement_without_authorizing_new_execution() {
        let store = Store::open(":memory:").unwrap();
        let space = space(&store);
        let principal = device(&store, &[space]);
        let other = device(&store, &[space]);
        let workspace = store
            .map_remote_workspace(
                &principal,
                space,
                Uuid::new_v4(),
                "Participant checkout",
                None,
            )
            .unwrap();
        let session = Uuid::new_v4();
        let actor = store
            .register_remote_actor(
                &principal,
                &workspace,
                session,
                Uuid::new_v4(),
                "codex",
                "receipt-owner",
            )
            .unwrap();
        let operation = reserve();
        assert_eq!(
            store
                .reconcile_remote_request(&principal, &actor.actor.id, &actor.epoch, &operation)
                .unwrap()["status"],
            "not_committed"
        );
        let committed = store
            .execute(&actor.actor, &actor.epoch, &operation)
            .unwrap();
        let replacement = store
            .register_remote_actor(
                &principal,
                &workspace,
                session,
                Uuid::new_v4(),
                "codex",
                "receipt-owner",
            )
            .unwrap();
        assert_ne!(actor.epoch, replacement.epoch);
        let receipt = store
            .reconcile_remote_request(&principal, &actor.actor.id, &actor.epoch, &operation)
            .unwrap();
        assert_eq!(receipt["status"], "committed");
        assert_eq!(receipt["result"], committed);
        assert!(store
            .reconcile_remote_request(&principal, &actor.actor.id, &actor.epoch, &reserve())
            .is_err());
        assert!(store
            .reconcile_remote_request(&other, &actor.actor.id, &actor.epoch, &operation)
            .is_err());
        let mut changed = operation.clone();
        if let Operation::FileReserve { paths, .. } = &mut changed {
            *paths = vec!["fixtures/changed.txt".into()];
        }
        assert!(store
            .reconcile_remote_request(&principal, &actor.actor.id, &actor.epoch, &changed)
            .is_err());
        store
            .execute_controller(
                "/coordinator",
                &ControllerOperation::DeviceRevoke {
                    device_id: principal.device.clone(),
                    request_id: id(),
                },
            )
            .unwrap();
        assert!(store
            .reconcile_remote_request(&principal, &actor.actor.id, &actor.epoch, &operation)
            .is_err());
    }

    #[test]
    fn reviewed_controller_mapping_replays_without_nested_transactions() {
        let store = Store::open(":memory:").unwrap();
        let space = space(&store);
        let principal = device(&store, &[space]);
        let device_id = Uuid::parse_str(&principal.device).unwrap();
        let checkout_id = Uuid::new_v4();
        let request_id = id();
        let intent =
            |generation, label: &str, request_id: String| ControllerOperation::RemoteWorkspaceMap {
                device_id,
                expected_generation: generation,
                space_id: space,
                checkout_id,
                label: label.into(),
                repository_id: None,
                request_id,
            };
        let operation = intent(
            principal.generation,
            "/participant-only/not-opened",
            request_id.clone(),
        );
        let result = store
            .execute_controller("/coordinator", &operation)
            .unwrap();
        assert_eq!(result["device_id"], principal.device);
        assert_eq!(result["checkout_id"], checkout_id.to_string());
        assert_eq!(
            store
                .execute_controller("/coordinator", &operation)
                .unwrap(),
            result
        );
        assert!(store
            .execute_controller(
                "/coordinator",
                &intent(principal.generation + 1, "Changed generation", id())
            )
            .is_err());
        assert!(store
            .execute_controller(
                "/coordinator",
                &intent(principal.generation, "Changed intent", request_id)
            )
            .is_err());
        assert_eq!(
            store
                .count(
                    "SELECT COUNT(*) AS count FROM remote_workspaces WHERE device = ?",
                    &[&principal.device]
                )
                .unwrap(),
            1
        );
    }

    #[test]
    fn remote_devices_paths_names_and_original_runs_are_isolated() {
        let store = Store::open(":memory:").unwrap();
        let space = space(&store);
        let first = device(&store, &[space]);
        let second = device(&store, &[space]);
        let checkout = Uuid::new_v4();
        // Neither label exists on this coordinator; it must never open these paths.
        let a = store
            .map_remote_workspace(&first, space, checkout, "/participant-only/checkout", None)
            .unwrap();
        let b = store
            .map_remote_workspace(&second, space, checkout, "/participant-only/checkout", None)
            .unwrap();
        assert!(a.id != b.id);
        let native = Uuid::new_v4();
        let native_run = Uuid::new_v4();
        let issuer = store
            .register_remote_actor(&first, &a, native, native_run, "codex", "issuer")
            .unwrap();
        assert!(issuer.expires_at > now());
        assert!(store
            .register_remote_actor(
                &second,
                &b,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "codex",
                "issuer"
            )
            .is_err());
        let receiver = store
            .register_remote_actor(
                &second,
                &b,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "codex",
                "receiver",
            )
            .unwrap();
        assert!(
            store.physical_root(&issuer.actor).unwrap()
                != store.physical_root(&receiver.actor).unwrap()
        );
        store
            .execute(&issuer.actor, &issuer.epoch, &reserve())
            .unwrap();
        store
            .execute(&receiver.actor, &receiver.epoch, &reserve())
            .unwrap();
        assert!(store
            .reservation_path(&issuer.actor, "unused", "../escape")
            .is_err());
        assert!(store
            .resolve_remote_run(&second, &issuer.actor.id, &issuer.epoch, false)
            .is_err());
        let resumed = store
            .register_remote_actor(&first, &a, native, native_run, "codex", "issuer")
            .unwrap();
        assert_eq!(resumed.epoch, issuer.epoch);
        let replaced = store
            .register_remote_actor(&first, &a, native, Uuid::new_v4(), "codex", "issuer")
            .unwrap();
        assert!(replaced.epoch != issuer.epoch);
        assert!(store
            .resolve_remote_run(&first, &issuer.actor.id, &issuer.epoch, true)
            .is_err());
        assert!(store
            .register_remote_actor(&first, &a, native, native_run, "codex", "issuer")
            .is_err());
        assert!(store
            .resolve_remote_run(&first, &replaced.actor.id, &replaced.epoch, true)
            .is_ok());
        store
            .execute_controller(
                "/coordinator",
                &ControllerOperation::DeviceRevoke {
                    device_id: first.device.clone(),
                    request_id: id(),
                },
            )
            .unwrap();
        assert!(store
            .resolve_remote_run(&first, &replaced.actor.id, &replaced.epoch, false)
            .is_err());
        assert!(store
            .execute(&replaced.actor, &replaced.epoch, &Operation::AgentList)
            .is_err());
    }

    #[test]
    fn remote_remap_and_leave_never_restore_old_run_authority() {
        let store = Store::open(":memory:").unwrap();
        let original = space(&store);
        let other = space(&store);
        let principal = device(&store, &[original, other]);
        let checkout = Uuid::new_v4();
        let workspace = store
            .map_remote_workspace(&principal, original, checkout, "Participant checkout", None)
            .unwrap();
        let native = Uuid::new_v4();
        let run = Uuid::new_v4();
        let actor = store
            .register_remote_actor(&principal, &workspace, native, run, "codex", "worker")
            .unwrap();
        store
            .map_remote_workspace(&principal, other, checkout, "Participant checkout", None)
            .unwrap();
        assert!(store
            .resolve_remote_run(&principal, &actor.actor.id, &actor.epoch, false)
            .is_err());
        store
            .map_remote_workspace(&principal, original, checkout, "Participant checkout", None)
            .unwrap();
        assert!(store
            .resolve_remote_run(&principal, &actor.actor.id, &actor.epoch, false)
            .is_err());
        assert!(store
            .register_remote_actor(&principal, &workspace, native, run, "codex", "worker")
            .is_err());
        let fresh = store
            .register_remote_actor(
                &principal,
                &workspace,
                native,
                Uuid::new_v4(),
                "codex",
                "worker",
            )
            .unwrap();
        store
            .execute_controller(
                &fresh.actor.project,
                &ControllerOperation::SpaceLeave {
                    space_id: original.to_string(),
                    agent: fresh.actor.id.clone(),
                    request_id: id(),
                },
            )
            .unwrap();
        assert!(store
            .resolve_remote_run(&principal, &fresh.actor.id, &fresh.epoch, false)
            .is_err());
    }

    #[test]
    fn native_replacement_preserves_uncertain_remote_execution() {
        let store = Store::open(":memory:").unwrap();
        let space = space(&store);
        let principal = device(&store, &[space]);
        let workspace = store
            .map_remote_workspace(
                &principal,
                space,
                Uuid::new_v4(),
                "Participant checkout",
                None,
            )
            .unwrap();
        let issuer = store
            .register_remote_actor(
                &principal,
                &workspace,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "codex",
                "issuer",
            )
            .unwrap();
        let native = Uuid::new_v4();
        let receiver = store
            .register_remote_actor(
                &principal,
                &workspace,
                native,
                Uuid::new_v4(),
                "codex",
                "receiver",
            )
            .unwrap();
        let assigned = store
            .execute(
                &issuer.actor,
                &issuer.epoch,
                &Operation::TaskAssign {
                    to: receiver.actor.id.clone(),
                    reviewer: None,
                    description: "Edit a fixture".into(),
                    acceptance: "Check passes".into(),
                    start_deadline: None,
                    execution_timeout_seconds: None,
                    review_timeout_seconds: None,
                    dependencies: vec![],
                    request_id: id(),
                },
            )
            .unwrap();
        let task = assigned["id"].as_str().unwrap();
        store
            .execute(
                &receiver.actor,
                &receiver.epoch,
                &Operation::TaskStart {
                    task_id: task.into(),
                    revision: 1,
                    expected_version: None,
                    request_id: id(),
                },
            )
            .unwrap();
        let replacement = store
            .register_remote_actor(
                &principal,
                &workspace,
                native,
                Uuid::new_v4(),
                "codex",
                "receiver",
            )
            .unwrap();
        let value = serde_json::to_value(store.task(&replacement.actor, task).unwrap()).unwrap();
        assert_eq!(value["state"], "running");
        assert_eq!(value["attempts"][0]["certainty"], "interrupted");
        assert!(value["attempts"][0]["finished_at"].is_null());
        assert!(store
            .resolve_remote_run(&principal, &receiver.actor.id, &receiver.epoch, true)
            .is_err());
    }

    #[test]
    fn v5_upgrade_keeps_host_state_and_creates_no_remote_admissions() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("bus.sqlite");
        let path = database.to_str().unwrap();
        let store = Store::open(path).unwrap();
        let host = store
            .register("host", "codex", directory.path().to_str().unwrap(), "host")
            .unwrap();
        let lease = store.execute(&host, "host-run", &reserve()).unwrap();
        let coordinator = store.coordinator_id().unwrap();
        store.connection.borrow_mut().batch_execute("DROP TABLE remote_runs; DROP TABLE remote_actor_bindings; DROP TABLE remote_workspaces; UPDATE meta SET value='5' WHERE key='schema_version';").unwrap();
        store.set_legacy_payload(SENTINEL_V5).unwrap();
        drop(store);
        let upgraded = Store::open(path).unwrap();
        assert_eq!(upgraded.coordinator_id().unwrap(), coordinator);
        assert_eq!(
            upgraded.meta_version().unwrap().as_deref(),
            Some(SCHEMA_VERSION)
        );
        assert_eq!(
            upgraded
                .count("SELECT COUNT(*) AS count FROM remote_actor_bindings", &[])
                .unwrap(),
            0
        );
        assert_eq!(
            upgraded
                .count("SELECT COUNT(*) AS count FROM reservations", &[])
                .unwrap(),
            1
        );
        assert!(lease["reservation_ids"].is_array());
        assert_eq!(
            read_legacy_payload(&format!("{path}.pre-upgrade-v5"))
                .unwrap()
                .as_deref(),
            Some(SENTINEL_V5)
        );
        assert_eq!(
            read_legacy_payload(path).unwrap().as_deref(),
            Some(SENTINEL)
        );
    }
}
