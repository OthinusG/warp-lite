//! Remote-owned project Stores reuse the existing human task engine.
use super::*;
use crate::{
    transport::{Broker, PanelQuery, RunningBroker},
    ControllerOperation, DomainError, Operation,
};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::{collections::BTreeMap, sync::Mutex};

const MAX_COMMAND: usize = 64 * 1024;
pub const MAX_RESULT: usize = 512 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "operation",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Command {
    Panel(PanelQuery),
    Operator(Operation),
    Controller(ControllerOperation),
    Scoped { scope: String, command: Box<Command> },
}

/// Credentials stay in native child environment and disappear with its owner.
pub(super) struct RunBinding {
    broker: Broker,
    terminal: String,
    run: String,
    pub environment: Vec<(String, String)>,
}
impl Drop for RunBinding {
    fn drop(&mut self) {
        self.broker
            .revoke_remote_run(&self.terminal, Some(&self.run));
    }
}
impl Projects {
    pub(super) fn bind_run(
        &self,
        fence: &ManagedFence,
        root: &Path,
        terminal: &str,
        run: &str,
        program: &str,
    ) -> Result<RunBinding, ManagedErrorCode> {
        let (broker, domain) = self.selected_broker(fence, root)?;
        if domain.starts_with("space:") && broker.worktree_binding(&crate::project_root(root)
            .map_err(|_| ManagedErrorCode::ManagedInvalidInput)?)
            .map_err(|_| ManagedErrorCode::ManagedUnavailable)?.is_none() {
            let checkout = crate::project_root(root).map_err(|_| ManagedErrorCode::ManagedInvalidInput)?;
            broker.control(&checkout, &ControllerOperation::WorktreeJoin {
                root: checkout.clone(), request_id: Uuid::new_v4().to_string(),
            }).map_err(|_| ManagedErrorCode::ManagedUnavailable)?;
        }
        let executable = std::env::current_exe().map_err(path_error)?;
        let executable = executable
            .to_str()
            .ok_or(ManagedErrorCode::ManagedUnavailable)?;
        let capability = broker
            .prepare_remote_run(terminal, program, root, run)
            .map_err(|_| ManagedErrorCode::ManagedUnavailable)?;
        let environment = vec![
            (crate::transport::ENDPOINT.into(), broker.endpoint.clone()),
            (crate::transport::CAPABILITY.into(), capability),
            (crate::transport::TERMINAL.into(), terminal.into()),
            ("WARP_AGENT_BIN".into(), executable.into()),
        ];
        Ok(RunBinding {
            broker,
            terminal: terminal.into(),
            run: run.into(),
            environment,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fence() -> ManagedFence {
        ManagedFence {
            service_id: Uuid::new_v4().to_string(),
            service_boot_id: Uuid::new_v4().to_string(),
            connection_id: Uuid::new_v4().to_string(),
            project_id: Uuid::new_v4().to_string(),
        }
    }

    #[test]
    fn worktree_team_spans_remote_project_fences_without_moving_private_history() {
        use crate::worktrees::tests::{Fixture, call, assign, id};
        use crate::transport::{Request, CAPABILITY, ENDPOINT, TERMINAL, PROTOCOL_MAJOR};
        let fixture = Fixture::new();
        let data = tempfile::tempdir().unwrap();
        let other_data = tempfile::tempdir().unwrap();
        let projects = Projects::new(data.path());
        let first = fence();
        let mut second = first.clone();
        second.connection_id = id();
        second.project_id = id();
        let register = |fence: &ManagedFence, root: &Path, name: &str| {
            let binding = projects.bind_run(fence, root, &id(), &id(), "fixture").unwrap();
            let env = |key| binding.environment.iter().find(|(name, _)| name == key).unwrap().1.clone();
            let mut request = Request { protocol_major: PROTOCOL_MAJOR, terminal: env(TERMINAL), capability: env(CAPABILITY), run: None,
                directory: Some(root.to_str().unwrap().into()), defer_initial_ready: false, native_activity: None,
                operation: Operation::AgentRegister { name: name.into() } };
            let endpoint = env(ENDPOINT);
            request.run = crate::transport::call(&endpoint, &request).unwrap()["run"].as_str().map(str::to_owned);
            (binding, request)
        };
        let (private_binding, private) = register(&first, &fixture.main, "private");
        let private_task = execute(&projects, &first, &fixture.main, &Command::Operator(Operation::TaskCreatePool {
            eligible: vec!["private".into()], description: "Retain private history".into(), acceptance: "Private only".into(),
            reviewer: None, dependencies: vec![], start_deadline: None, execution_timeout_seconds: None, review_timeout_seconds: None, request_id: id()
        }))["value"]["id"].as_str().unwrap().to_owned();
        let join_main = Command::Controller(ControllerOperation::WorktreeJoin { root: fixture.main.to_str().unwrap().into(), request_id: id() });
        assert!(execute(&projects, &first, &fixture.main, &join_main).get("value").is_some());
        let join_linked = Command::Controller(ControllerOperation::WorktreeJoin { root: fixture.linked.to_str().unwrap().into(), request_id: id() });
        assert!(execute(&projects, &second, &fixture.linked, &join_linked).get("value").is_some());
        let (lead_binding, lead) = register(&first, &fixture.main, "lead");
        let (worker_binding, worker) = register(&second, &fixture.linked, "worker");
        let lead_agent = call(&lead_binding.broker, &lead, Operation::AgentRegister { name:"".into() }).unwrap()["agent"]["id"].as_str().unwrap().to_owned();
        let worker_agent = call(&worker_binding.broker, &worker, Operation::AgentRegister { name:"".into() }).unwrap()["agent"]["id"].as_str().unwrap().to_owned();
        assert!(execute(&projects, &first, &fixture.main, &Command::Controller(ControllerOperation::WorktreeCoordinator {
            root:fixture.main.to_str().unwrap().into(), agent:lead_agent, run:lead.run.clone().unwrap(), request_id:id(),
        })).get("value").is_some());
        assert!(execute(&projects, &second, &fixture.linked, &Command::Controller(ControllerOperation::WorktreeWorker {
            root:fixture.linked.to_str().unwrap().into(), agent:worker_agent, run:worker.run.clone().unwrap(), request_id:id(),
        })).get("value").is_some());
        assert_ne!(lead_binding.broker.endpoint, worker_binding.broker.endpoint);
        assert_eq!(private_binding.broker.endpoint, lead_binding.broker.endpoint);
        assert!(call(&lead_binding.broker, &lead, Operation::TaskGet { task_id: private_task.clone() }).is_err());
        assert!(call(&private_binding.broker, &private, Operation::AgentList).unwrap().as_array().unwrap().is_empty());
        let task = call(&lead_binding.broker, &lead, assign("worker")).unwrap()["id"].as_str().unwrap().to_owned();
        call(&worker_binding.broker, &worker, Operation::TaskStart { task_id: task.clone(), revision: 1, expected_version: None, request_id: id() }).unwrap();
        let head = crate::worktrees::tests::git(&fixture.linked, &["rev-parse", "HEAD"]);
        let evidence = call(&worker_binding.broker, &worker, Operation::EvidenceAdd {
            task_id: task.clone(), kind: "commit".into(), attempt_id: None,
            commit: Some(head.trim().into()), path: None, hash: None,
            repository: Some(crate::worktrees::Worktree::discover(&fixture.linked).unwrap().repository),
            branch: Some("feature".into()), base: None, head: None, command: None,
            outcome: None, exit_code: None, summary: None, request_id: id(),
        }).unwrap()["evidence_id"].as_str().unwrap().to_owned();
        call(&worker_binding.broker, &worker, Operation::TaskSubmit { task_id: task.clone(), revision: 1, result: "Feature ready".into(), evidence: "Owned fixture check passed".into(), evidence_ids: vec![evidence.clone()], attempt_id: None, expected_version: None, request_id: id() }).unwrap();
        let panel = Command::Panel(PanelQuery { worktree:true, ..Default::default() });
        let first_panel = execute(&projects, &first, &fixture.main, &panel);
        let second_panel = execute(&projects, &second, &fixture.linked, &panel);
        assert_eq!(first_panel["value"]["collaboration_scope"], second_panel["value"]["collaboration_scope"]);
        assert_ne!(first_panel["value"]["project"], second_panel["value"]["project"]);
        let scope = first_panel["value"]["collaboration_scope"].as_str().unwrap().to_owned();
        assert!(execute(&projects, &first, &fixture.main, &Command::Scoped {
            scope: scope.clone(), command: Box::new(Command::Controller(ControllerOperation::EvidenceVerify {
                evidence_id: evidence, verified: true, request_id: id(),
            })),
        }).get("value").is_some());
        let review = Operation::TaskReview { task_id: task.clone(), revision: 1, accepted: true, feedback: "Ready to integrate".into(), expected_version: Some(3), request_id: id() };
        assert_eq!(execute(&projects, &first, &fixture.main, &Command::Scoped { scope: first.project_id.clone(), command: Box::new(Command::Operator(review.clone())) })["error"]["code"], "scope_denied");
        assert_eq!(execute(&projects, &first, &fixture.main, &Command::Scoped { scope: scope.clone(), command: Box::new(Command::Operator(review)) })["value"]["state"], "accepted");
        let unjoined = execute(&projects, &fence(), &fixture.unjoined, &Command::Panel(PanelQuery::default()));
        assert!(unjoined["value"]["tasks"].as_array().unwrap().is_empty());
        let independent = Projects::new(other_data.path());
        assert!(execute(&independent, &first, &fixture.main, &panel)["value"]["tasks"].as_array().unwrap().is_empty());
        let leave = Command::Controller(ControllerOperation::WorktreeLeave { root: fixture.linked.to_str().unwrap().into(), request_id: id() });
        assert!(execute(&projects, &second, &fixture.linked, &leave).get("value").is_some());
        assert!(call(&worker_binding.broker, &worker, Operation::AgentList).is_err());
        // Lost membership responses reconcile in the same repository Store after routing changes.
        assert!(execute(&projects, &second, &fixture.linked, &leave).get("value").is_some());
        assert!(execute(&projects, &second, &fixture.linked, &join_linked).get("value").is_some());
        assert_eq!(execute(&projects, &second, &fixture.linked, &panel)["value"]["worktree_joined"], false,
            "replaying an old join must not undo a later leave");
        let private_return = Command::Controller(ControllerOperation::WorktreeLeave { root: fixture.main.to_str().unwrap().into(), request_id: id() });
        execute(&projects, &first, &fixture.main, &private_return);
        let history = execute(&projects, &first, &fixture.main, &Command::Panel(PanelQuery::default()));
        assert_eq!(history["value"]["tasks"][0]["id"], private_task);
    }
    fn execute(
        projects: &Projects,
        fence: &ManagedFence,
        root: &Path,
        command: &Command,
    ) -> serde_json::Value {
        let result = projects
            .execute(
                ProjectTasksRequest {
                    fence: Some(fence.clone()),
                    query_generation: 17,
                    command_json: serde_json::to_vec(command).unwrap(),
                },
                fence,
                root,
            )
            .unwrap();
        assert_eq!(result.fence.as_ref(), Some(fence));
        assert_eq!(result.query_generation, 17);
        decode_result(&result.result_json).unwrap()
    }
    #[test]
    fn private_run_credentials_fence_replacement_directory_and_revoked_replay() {
        use crate::transport::{call, Request, CAPABILITY, PROTOCOL_MAJOR};
        let data = tempfile::tempdir().unwrap();
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("project");
        std::fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let projects = Projects::new(data.path());
        let fence = fence();
        let terminal = Uuid::new_v4().to_string();
        let run = Uuid::new_v4().to_string();
        let binding = projects
            .bind_run(&fence, &root, &terminal, &run, "fixture")
            .unwrap();
        let capability = binding
            .environment
            .iter()
            .find(|(key, _)| key == CAPABILITY)
            .unwrap()
            .1
            .clone();
        let mut request = Request {
            protocol_major: PROTOCOL_MAJOR,
            terminal: terminal.clone(),
            capability: capability.clone(),
            run: None,
            defer_initial_ready: false,
            native_activity: None,
            directory: None,
            operation: Operation::AgentRegister {
                name: "worker".into(),
            },
        };
        let registration = call(&binding.broker.endpoint, &request).unwrap();
        assert_eq!(registration["run"], run);
        request.run = Some(run.clone());
        request.directory = Some(base.path().to_str().unwrap().into());
        assert!(call(&binding.broker.endpoint, &request).is_err());
        request.directory = None;
        request.operation = Operation::AgentSend {
            to: "worker".into(),
            body: "Bound original intent".into(),
            subject: None,
            thread_id: None,
            reply_to: None,
            task_id: None,
            request_id: Uuid::new_v4().to_string(),
        };
        let original = call(&binding.broker.endpoint, &request).unwrap();
        assert_eq!(original, call(&binding.broker.endpoint, &request).unwrap());
        std::fs::rename(&root, base.path().join("old-project")).unwrap();
        std::fs::create_dir(&root).unwrap();
        assert!(call(&binding.broker.endpoint, &request).is_err());
        std::fs::remove_dir(&root).unwrap();
        std::fs::rename(base.path().join("old-project"), &root).unwrap();
        let broker = binding.broker.clone();
        drop(binding);
        assert!(call(&broker.endpoint, &request).is_err());
        let replacement = projects
            .bind_run(
                &fence,
                &root,
                &terminal,
                &Uuid::new_v4().to_string(),
                "fixture",
            )
            .unwrap();
        assert!(replacement
            .environment
            .iter()
            .find(|(key, _)| key == CAPABILITY)
            .is_some_and(|(_, value)| value != &capability));
        assert!(call(&broker.endpoint, &request).is_err());
    }
    #[test]
    fn task_history_is_remote_owned_persistent_and_project_isolated() {
        let data = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let projects = Projects::new(data.path());
        let first = fence();
        let broker = projects
            .broker(&first.project_id, root.to_str().unwrap())
            .unwrap();
        let terminal = Uuid::new_v4().to_string();
        let capability = broker.prepare(&terminal).unwrap();
        broker
            .activate(&terminal, "fixture", root.to_str().unwrap(), false)
            .unwrap();
        // Actual private IPC admission, never the trusted GUI operator route.
        let registration = crate::transport::call(
            &broker.endpoint,
            &crate::transport::Request {
                protocol_major: crate::transport::PROTOCOL_MAJOR,
                terminal,
                capability,
                run: None,
                defer_initial_ready: false,
                native_activity: None,
                directory: None,
                operation: Operation::AgentRegister {
                    name: "worker".into(),
                },
            },
        )
        .unwrap();
        assert_eq!(registration["agent"]["name"], "worker");
        let request_id = Uuid::new_v4().to_string();
        let mut operation = Operation::TaskCreatePool {
            description: "Remote fixture task".into(),
            acceptance: "Exact remote history".into(),
            eligible: vec!["worker".into()],
            reviewer: None,
            dependencies: vec![],
            start_deadline: None,
            execution_timeout_seconds: None,
            review_timeout_seconds: None,
            request_id,
        };
        let created = execute(
            &projects,
            &first,
            &root,
            &Command::Operator(operation.clone()),
        );
        assert!(created.get("error").is_none(), "Task creation rejected");
        let mut second = first.clone();
        second.connection_id = Uuid::new_v4().to_string();
        assert_eq!(
            created,
            execute(
                &projects,
                &second,
                &root,
                &Command::Operator(operation.clone())
            )
        );
        if let Operation::TaskCreatePool { description, .. } = &mut operation {
            *description = "Changed intent".into();
        }
        assert_eq!(
            execute(&projects, &second, &root, &Command::Operator(operation))["error"]["code"],
            "request_conflict"
        );
        let panel = Command::Panel(PanelQuery::default());
        let before = execute(&projects, &second, &root, &panel);
        assert_eq!(before["value"]["tasks"].as_array().unwrap().len(), 1);
        assert_eq!(
            before["value"]["agents"][0]["device"],
            format!("remote:{}", first.service_id)
        );
        let other = fence();
        let other_root = tempfile::tempdir().unwrap();
        assert!(
            execute(&projects, &other, other_root.path(), &panel)["value"]["tasks"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        // The same spelling with a new native project identity has a fresh Store.
        let replacement = fence();
        assert!(
            execute(&projects, &replacement, &root, &panel)["value"]["tasks"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(projects
            .broker(&first.project_id, other_root.path().to_str().unwrap())
            .is_err());
        drop(broker);
        drop(projects);
        let reopened = Projects::new(data.path());
        let restored = execute(&reopened, &second, &root, &panel);
        assert_eq!(before["value"]["tasks"], restored["value"]["tasks"]);
        assert_eq!(before["value"]["events"], restored["value"]["events"]);
    }
    #[test]
    fn forbidden_agent_and_device_authority_never_creates_a_store() {
        let data = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let projects = Projects::new(data.path());
        let fence = fence();
        for command in [
            Command::Operator(Operation::AgentRegister {
                name: "spoofed".into(),
            }),
            Command::Controller(ControllerOperation::DeviceList),
            Command::Panel(PanelQuery {
                project: "/caller-selected".into(),
                ..Default::default()
            }),
        ] {
            assert!(projects
                .execute(
                    ProjectTasksRequest {
                        fence: Some(fence.clone()),
                        query_generation: 1,
                        command_json: serde_json::to_vec(&command).unwrap()
                    },
                    &fence,
                    root.path()
                )
                .is_err());
        }
        assert!(!data.path().join("projects").exists());
        let error = decode_result(br#"{"error":{"code":"request_conflict","message":"untrusted prose","retryable":true,"version":9}}"#).unwrap();
        assert_eq!(error["error"]["message"], "Remote project operation failed");
        assert_eq!(error["error"]["retryable"], false);
        assert!(error["error"].get("version").is_none());
        assert!(decode_result(
            br#"{"error":{"code":"unknown","message":"ignored","retryable":true}}"#
        )
        .is_none());
    }
}

pub(crate) fn decode_result(bytes: &[u8]) -> Option<serde_json::Value> {
    if bytes.len() > MAX_RESULT {
        return None;
    }
    let mut value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let object = value.as_object()?;
    if object.len() != 1 {
        return None;
    }
    if object.contains_key("value") {
        return Some(value);
    }
    let mut error: DomainError = serde_json::from_value(object.get("error")?.clone()).ok()?;
    if !matches!(
        error.code.as_str(),
        "invalid_input"
            | "invalid_state"
            | "unauthorized"
            | "scope_denied"
            | "request_conflict"
            | "request_epoch_expired"
            | "capacity_exceeded"
            | "version_conflict"
            | "stale_revision"
            | "stale_attempt"
            | "execution_unknown"
            | "coordinator_unavailable"
            | "dependency_cycle"
            | "dependency_blocked"
            | "reservation_conflict"
            | "storage_full"
            | "binding_inactive"
    ) {
        return None;
    }
    error.message = "Remote project operation failed".into();
    error.retryable &= matches!(
        error.code.as_str(),
        "coordinator_unavailable" | "storage_full" | "binding_inactive"
    );
    if !matches!(
        error.code.as_str(),
        "version_conflict" | "stale_revision" | "stale_attempt"
    ) {
        error.version = None;
    }
    value["error"] = serde_json::to_value(error).ok()?;
    Some(value)
}

pub(super) struct Projects {
    directory: PathBuf,
    owners: Mutex<BTreeMap<String, (String, RunningBroker)>>,
}
impl Projects {
    pub(super) fn new(directory: &Path) -> Self {
        Self {
            directory: directory.join("projects"),
            owners: Mutex::new(BTreeMap::new()),
        }
    }
    pub(super) fn broker(&self, project_id: &str, root: &str) -> Result<Broker, ManagedErrorCode> {
        let id = Uuid::parse_str(project_id).map_err(|_| ManagedErrorCode::ManagedInvalidInput)?;
        if id.is_nil() || id.to_string() != project_id {
            return Err(ManagedErrorCode::ManagedInvalidInput);
        }
        let mut owners = self
            .owners
            .lock()
            .map_err(|_| ManagedErrorCode::ManagedUnavailable)?;
        if let Some((original, owner)) = owners.get(project_id) {
            if original != root {
                return Err(ManagedErrorCode::ManagedConflict);
            }
            return Ok(owner.broker.clone());
        }
        if owners.len() >= 64 {
            return Err(ManagedErrorCode::ManagedCapacityExceeded);
        }
        identity::private_directory(&self.directory).map_err(path_error)?;
        let directory = self.directory.join(project_id);
        identity::private_directory(&directory).map_err(path_error)?;
        // A renamed root needs an explicit domain migration, never a fresh scope
        // silently hiding prior work under the same native project identity.
        let mut binding =
            identity::private_file(&directory.join("root.binding")).map_err(path_error)?;
        if binding.metadata().map_err(path_error)?.len() == 0 {
            binding.write_all(root.as_bytes()).map_err(path_error)?;
            binding.sync_all().map_err(path_error)?;
        } else {
            let mut original = String::new();
            binding
                .take(4097)
                .read_to_string(&mut original)
                .map_err(path_error)?;
            if original != root {
                return Err(ManagedErrorCode::ManagedConflict);
            }
        }
        let database = directory.join("tasks.sqlite");
        drop(identity::private_file(&database).map_err(path_error)?);
        // ponytail: reuse one native broker per project/team (64 maximum);
        // share runtimes only if this measured ceiling becomes too costly.
        let owner =
            RunningBroker::start(&database).map_err(|_| ManagedErrorCode::ManagedUnavailable)?;
        let broker = owner.broker.clone();
        owners.insert(project_id.into(), (root.into(), owner));
        Ok(broker)
    }

    fn worktree_broker(&self, root: &Path) -> Result<(Broker, crate::worktrees::Worktree), ManagedErrorCode> {
        let checkout = crate::worktrees::Worktree::discover(root)
            .map_err(|_| ManagedErrorCode::ManagedInvalidInput)?;
        let broker = self.broker(&checkout.repository, checkout.common.to_str().ok_or(ManagedErrorCode::ManagedInvalidInput)?)?;
        Ok((broker, checkout))
    }

    fn selected_broker(&self, fence: &ManagedFence, root: &Path) -> Result<(Broker, String), ManagedErrorCode> {
        if let Ok(checkout) = crate::worktrees::Worktree::discover(root) {
            if self.directory.join(&checkout.repository).join("tasks.sqlite").exists() {
                let broker = self.broker(&checkout.repository, checkout.common.to_str().ok_or(ManagedErrorCode::ManagedInvalidInput)?)?;
                if broker.worktree_enabled(&checkout.root).map_err(|_| ManagedErrorCode::ManagedStaleAttachment)? {
                    if let Some(domain) = broker.worktree_domain(&checkout.root)
                        .map_err(|_| ManagedErrorCode::ManagedStaleAttachment)? {
                        return Ok((broker, domain));
                    }
                }
            }
        }
        Ok((self.broker(&fence.project_id, root.to_str().ok_or(ManagedErrorCode::ManagedUnavailable)?)?,
            root.to_str().ok_or(ManagedErrorCode::ManagedUnavailable)?.into()))
    }
    pub(super) fn execute(
        &self,
        request: ProjectTasksRequest,
        fence: &ManagedFence,
        root: &Path,
    ) -> Result<ProjectTasksResult, ManagedErrorCode> {
        if request.command_json.len() > MAX_COMMAND {
            return Err(ManagedErrorCode::ManagedCapacityExceeded);
        }
        let command: Command = serde_json::from_slice(&request.command_json)
            .map_err(|_| ManagedErrorCode::ManagedInvalidInput)?;
        let (expected_scope, command) = match command {
            Command::Scoped { scope, command } => (Some(scope), *command),
            command => (None, command),
        };
        if matches!(&command, Command::Scoped { .. }) {
            return Err(ManagedErrorCode::ManagedInvalidInput);
        }
        let root_path = root;
        let root = root.to_str().ok_or(ManagedErrorCode::ManagedUnavailable)?;
        // Reject forbidden authority/lifecycle fields before opening a Store.
        match &command {
            Command::Operator(
                Operation::TaskReview {
                    expected_version: None,
                    ..
                }
                | Operation::TaskCancel {
                    expected_version: None,
                    ..
                }
                | Operation::TaskRetry {
                    expected_version: None,
                    ..
                }
                | Operation::TaskReassign {
                    expected_version: None,
                    ..
                }
                | Operation::TaskSetDependencies {
                    expected_version: None,
                    ..
                },
            )
            | Command::Controller(
                ControllerOperation::TaskForceCancel {
                    expected_version: None,
                    ..
                }
                | ControllerOperation::HistoryPurge {
                    expected_sequence: None,
                    ..
                },
            ) => return Err(ManagedErrorCode::ManagedInvalidInput),
            Command::Panel(query)
                if !query.project.is_empty() || query.terminal.is_some() || query.spaces =>
            {
                return Err(ManagedErrorCode::ManagedInvalidInput)
            }
            Command::Operator(operation)
                if !matches!(
                    operation,
                    Operation::AgentList
                        | Operation::AgentSend { .. }
                        | Operation::TaskAssign { .. }
                        | Operation::TaskList { .. }
                        | Operation::TaskGet { .. }
                        | Operation::TaskHistory { .. }
                        | Operation::TaskReview { .. }
                        | Operation::TaskCreatePool { .. }
                        | Operation::TaskCancel { .. }
                        | Operation::TaskRetry { .. }
                        | Operation::TaskReassign { .. }
                        | Operation::TaskSetDependencies { .. }
                        | Operation::ThreadGet { .. }
                        | Operation::MessageSearch { .. }
                ) =>
            {
                return Err(ManagedErrorCode::ManagedFeatureUnavailable)
            }
            Command::Controller(operation)
                if !matches!(
                    operation,
                    ControllerOperation::WorktreeCoordinator { .. }
                        | ControllerOperation::WorktreeWorker { .. }
                        | ControllerOperation::WorktreeCreate { .. }
                        | ControllerOperation::WorktreeJoin { .. }
                        | ControllerOperation::WorktreeLeave { .. }
                        | ControllerOperation::EvidenceVerify { .. }
                        | ControllerOperation::TaskForceCancel { .. }
                        | ControllerOperation::TaskArchive { .. }
                        | ControllerOperation::ArchiveAged { .. }
                        | ControllerOperation::HistoryExport { .. }
                        | ControllerOperation::PurgePreview
                        | ControllerOperation::HistoryPurge { .. }
                        | ControllerOperation::ReservationUpdate { .. }
                ) =>
            {
                return Err(ManagedErrorCode::ManagedFeatureUnavailable)
            }
            Command::Controller(ControllerOperation::ReservationUpdate { workspace, .. })
                if workspace != root =>
            {
                return Err(ManagedErrorCode::ManagedInvalidInput)
            }
            Command::Controller(ControllerOperation::WorktreeJoin { root: selected, .. }
                | ControllerOperation::WorktreeLeave { root: selected, .. }
                | ControllerOperation::WorktreeCreate { root: selected, .. })
                if crate::project_root(root_path).ok().as_deref() != Some(selected.as_str()) =>
            {
                return Err(ManagedErrorCode::ManagedInvalidInput)
            }
            _ => (),
        }
        if let Command::Controller(operation @ (ControllerOperation::WorktreeCoordinator { root: selected, .. }
            | ControllerOperation::WorktreeWorker { root: selected, .. })) = &command {
            let (team, checkout) = self.worktree_broker(root_path)?;
            let worker = crate::worktrees::Worktree::discover(Path::new(selected)).map_err(|_| ManagedErrorCode::ManagedInvalidInput)?;
            if checkout.repository != worker.repository {
                return Err(ManagedErrorCode::ManagedInvalidInput);
            }
            let sources: Vec<_> = self.owners.lock().map_err(|_| ManagedErrorCode::ManagedUnavailable)?
                .values().map(|(_, owner)| owner.broker.clone()).collect();
            let mut result = Err(crate::scope_denied("Selected Agent run is no longer active"));
            let receipt = team.worktree_role_receipt(&worker.root, operation).map_err(|_| ManagedErrorCode::ManagedConflict)?;
            if let Some(receipt) = receipt { result = Ok(receipt); }
            for source in sources {
                if result.is_ok() { break; }
                let candidates = source.worktree_candidates(&checkout.root).unwrap_or_default();
                let (agent, run) = match operation {
                    ControllerOperation::WorktreeCoordinator { agent, run, .. } | ControllerOperation::WorktreeWorker { agent, run, .. } => (agent,run),
                    _ => unreachable!(),
                };
                if candidates.iter().any(|candidate| candidate["agent"]["id"].as_str() == Some(agent) && candidate["run"].as_str() == Some(run)) {
                    result = team.enroll_worktree(&source, operation);
                    break;
                }
            }
            if result.is_ok() && matches!(operation, ControllerOperation::WorktreeCoordinator { .. }) {
                let sources: Vec<_> = self.owners.lock().map_err(|_| ManagedErrorCode::ManagedUnavailable)?
                    .values().map(|(_, owner)| owner.broker.clone()).collect();
                for source in sources {
                    team.enroll_worktree_agents(&source, &checkout.root)
                        .map_err(|_| ManagedErrorCode::ManagedUnavailable)?;
                }
            }
            let value = match result { Ok(value) => serde_json::json!({"value":value}), Err(error) => {
                let error = DomainError::from_error(error);
                serde_json::json!({"error":{"code":error.code,"message":"Remote enrollment failed","retryable":error.retryable}})
            }};
            return Ok(ProjectTasksResult { fence:Some(fence.clone()), query_generation:request.query_generation,
                result_json:serde_json::to_vec(&value).map_err(|_| ManagedErrorCode::ManagedUnavailable)? });
        }
        let membership = matches!(&command, Command::Controller(ControllerOperation::WorktreeJoin { .. } | ControllerOperation::WorktreeLeave { .. }));
        let (broker, domain) = if membership {
            let (broker, _) = self.worktree_broker(root_path)?;
            (broker, root.to_owned())
        } else if matches!(&command, Command::Panel(query) if query.worktree) && root_path.join(".git").exists() {
            let (broker, checkout) = self.worktree_broker(root_path)?;
            let sources: Vec<_> = self.owners.lock().map_err(|_| ManagedErrorCode::ManagedUnavailable)?
                .values().map(|(_, owner)| owner.broker.clone()).collect();
            for source in sources {
                broker.enroll_worktree_agents(&source, &checkout.root)
                    .map_err(|_| ManagedErrorCode::ManagedUnavailable)?;
            }
            let domain = broker.worktree_domain(&checkout.root).map_err(|_| ManagedErrorCode::ManagedStaleAttachment)?
                .unwrap_or(checkout.root);
            (broker,domain)
        } else if matches!(&command, Command::Panel(query) if !query.worktree) {
            (self.broker(&fence.project_id, root)?, root.to_owned())
        } else if expected_scope.as_deref() == Some(fence.project_id.as_str()) {
            (self.broker(&fence.project_id, root)?, root.to_owned())
        } else { self.selected_broker(fence, root_path)? };
        let scope = if domain.starts_with("space:") { domain.clone() } else { fence.project_id.clone() };
        if !membership && !matches!(&command, Command::Panel(_)) &&
            (expected_scope.as_ref().is_some_and(|expected| expected != &scope)
                || (domain.starts_with("space:") && expected_scope.is_none())) {
            return Ok(ProjectTasksResult { fence: Some(fence.clone()), query_generation: request.query_generation,
                result_json: serde_json::to_vec(&serde_json::json!({"error": {"code": "scope_denied", "message": "Remote project operation failed", "retryable": false}})).unwrap() });
        }
        let result = match command {
            Command::Panel(mut query) => {
                let worktree = query.worktree;
                query.project = if domain.starts_with("space:") {
                    crate::project_root(root_path).map_err(|_| ManagedErrorCode::ManagedStaleAttachment)?
                } else { root.into() };
                query.scope =
                    (query.scope.as_deref() == Some(scope.as_str())).then(|| domain.clone());
                broker.operator_panel(&query).map(|mut value| {
                    value["project"] = serde_json::json!(fence.project_id);
                    value["collaboration_scope"] = serde_json::json!(scope);
                    value["worktree_joined"] = serde_json::json!(broker.worktree_binding(root).ok().flatten().is_some());
                    if worktree {
                        let sources: Vec<_> = self.owners.lock().map(|owners| owners.values().map(|(_, owner)| owner.broker.clone()).collect()).unwrap_or_default();
                        let candidates: Vec<_> = sources.into_iter().flat_map(|source| source.worktree_candidates(root).unwrap_or_default()).take(50).collect();
                        value["candidates"] = serde_json::json!(candidates);
                    }
                    let checkout_root = crate::project_root(root_path).unwrap_or_else(|_| root.into());
                    value["worktree_available"] = serde_json::json!(Path::new(&checkout_root).join(".git").exists());
                    value["worktree_root"] = serde_json::json!(checkout_root);
                    value["worktree_branch"] = serde_json::json!(crate::worktrees::branch(&checkout_root));
                    if let Some(agents) = value["agents"].as_array_mut() {
                        for row in agents {
                            row["device"] =
                                serde_json::json!(format!("remote:{}", fence.service_id));
                            if !row["observation_source"].is_null() {
                                row["observation_source"] = serde_json::json!("remote observation");
                            }
                        }
                    }
                    value
                })
            }
            Command::Operator(operation) => broker.operator(&domain, &operation),
            Command::Controller(operation) => broker.control(&domain, &operation),
            Command::Scoped { .. } => unreachable!(),
        };
        let value = match result {
            Ok(value) => serde_json::json!({"value": value}),
            Err(error) => {
                let domain = DomainError::from_error(error);
                serde_json::json!({"error": {"code": domain.code, "message": "Remote project operation failed", "retryable": domain.retryable, "version": domain.version}})
            }
        };
        let result_json =
            serde_json::to_vec(&value).map_err(|_| ManagedErrorCode::ManagedUnavailable)?;
        if result_json.len() > MAX_RESULT {
            return Err(ManagedErrorCode::ManagedCapacityExceeded);
        }
        Ok(ProjectTasksResult {
            fence: Some(fence.clone()),
            query_generation: request.query_generation,
            result_json,
        })
    }
}
