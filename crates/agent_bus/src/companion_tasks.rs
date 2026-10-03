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
    fn broker(&self, project_id: &str, root: &str) -> Result<Broker, ManagedErrorCode> {
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
        if owners.len() >= 32 {
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
        // ponytail: reuse one native broker per admitted project (32 maximum);
        // share runtimes only if this measured ceiling becomes too costly.
        let owner =
            RunningBroker::start(&database).map_err(|_| ManagedErrorCode::ManagedUnavailable)?;
        let broker = owner.broker.clone();
        owners.insert(project_id.into(), (root.into(), owner));
        Ok(broker)
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
                    ControllerOperation::EvidenceVerify { .. }
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
            _ => (),
        }
        let broker = self.broker(&fence.project_id, root)?;
        let result = match command {
            Command::Panel(mut query) => {
                query.project = root.into();
                query.scope =
                    (query.scope.as_deref() == Some(&fence.project_id)).then(|| root.to_owned());
                broker.operator_panel(&query).map(|mut value| {
                    value["project"] = serde_json::json!(fence.project_id);
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
            Command::Operator(operation) => broker.operator(root, &operation),
            Command::Controller(operation) => broker.control(root, &operation),
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
