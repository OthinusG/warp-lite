//! Local coordination for third-party CLI agents; no model or cloud client lives here.
pub mod launch;
pub mod mcp;
pub mod readiness;
pub mod session;
pub mod storage;
pub mod transport;

pub use storage::Store;

use std::{fmt, path::Path, time::Duration};

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAX_FRAME: usize = 1024 * 1024;
pub(crate) const MAX_TEXT: usize = 8192;
pub(crate) const MAX_SUBJECT: usize = 256;
pub(crate) const PAGE_DEFAULT: u32 = 50;
pub(crate) const PAGE_MAX: u32 = 200;
pub(crate) const MAX_DEPENDENCIES: usize = 100;
pub(crate) const MAX_ELIGIBLES: usize = 100;
pub(crate) const MAX_PATHS: usize = 100;
pub(crate) const RESERVATION_TTL_DEFAULT: u64 = 600;
pub(crate) const RESERVATION_TTL_MAX: u64 = 3600;
pub(crate) const DEADLINE_TIMEOUT_MAX: u64 = 604_800;
pub(crate) const ARCHIVE_AFTER_DAYS: u64 = 30;
pub(crate) const DATABASE_SOFT_LIMIT: u64 = 1 << 30;
pub(crate) const DATABASE_HARD_LIMIT: u64 = 2 << 30;
/// The trusted local UI acts as a human operator under this reserved program identity.
pub(crate) const OPERATOR_PROGRAM: &str = "warp";
pub(crate) const OPERATOR_NAME: &str = "operator";
/// One activated terminal session is one mutation epoch; restart or expiry fences its requests.
pub(crate) const MUTATION_EPOCH: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Stable machine-readable failure; messages never echo payloads, credentials or environment values.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DomainError {}

impl DomainError {
    /// Domain failures pass through; infrastructure failures become a retryable coordinator error.
    pub(crate) fn from_error(error: anyhow::Error) -> Self {
        match error.downcast::<Self>() {
            Ok(domain) => domain,
            Err(error) => Self {
                code: "coordinator_unavailable".into(),
                message: error.to_string(),
                retryable: true,
                version: None,
            },
        }
    }
}

pub(crate) fn domain(
    code: &str,
    message: &str,
    retryable: bool,
    version: Option<u64>,
) -> anyhow::Error {
    DomainError {
        code: code.into(),
        message: message.into(),
        retryable,
        version,
    }
    .into()
}

pub(crate) fn invalid_input(message: &str) -> anyhow::Error {
    domain("invalid_input", message, false, None)
}

pub(crate) fn invalid_state(message: &str) -> anyhow::Error {
    domain("invalid_state", message, false, None)
}

pub(crate) fn unauthorized(message: &str) -> anyhow::Error {
    domain("unauthorized", message, false, None)
}

pub(crate) fn scope_denied(message: &str) -> anyhow::Error {
    domain("scope_denied", message, false, None)
}

pub(crate) fn request_conflict(message: &str) -> anyhow::Error {
    domain("request_conflict", message, false, None)
}

pub(crate) fn epoch_expired(message: &str) -> anyhow::Error {
    domain("request_epoch_expired", message, false, None)
}

pub(crate) fn capacity_exceeded(message: &str) -> anyhow::Error {
    domain("capacity_exceeded", message, false, None)
}

pub(crate) fn version_conflict(message: &str, version: u64) -> anyhow::Error {
    domain("version_conflict", message, false, Some(version))
}

pub(crate) fn stale_revision(message: &str, version: u64) -> anyhow::Error {
    domain("stale_revision", message, false, Some(version))
}

pub(crate) fn stale_attempt(message: &str, version: u64) -> anyhow::Error {
    domain("stale_attempt", message, false, Some(version))
}

/// The owning execution's fate is unconfirmed; no cancellation, retry or reassignment is implied.
pub(crate) fn execution_unknown(message: &str) -> anyhow::Error {
    domain("execution_unknown", message, false, None)
}

pub(crate) fn coordinator_unavailable(message: &str) -> anyhow::Error {
    domain("coordinator_unavailable", message, true, None)
}

pub(crate) fn dependency_cycle(message: &str) -> anyhow::Error {
    domain("dependency_cycle", message, false, None)
}

pub(crate) fn dependency_blocked(message: &str) -> anyhow::Error {
    domain("dependency_blocked", message, false, None)
}

pub(crate) fn reservation_conflict(message: &str) -> anyhow::Error {
    domain("reservation_conflict", message, false, None)
}

pub(crate) fn storage_full(message: &str) -> anyhow::Error {
    domain("storage_full", message, true, None)
}

/// SQLite reports exhaustion as an infrastructure error; surface the truthful stable code instead.
pub(crate) fn classify_storage(error: anyhow::Error) -> anyhow::Error {
    match error.downcast::<DomainError>() {
        Ok(domain) => domain.into(),
        Err(error) => {
            let message = error.to_string();
            if message.contains("disk is full") || message.contains("database is full") {
                storage_full("Local storage is full; free space or export and purge eligible history")
            } else {
                error
            }
        }
    }
}

/// Internal discovery race only; the native bridge may retry while the UI start event is in flight.
pub(crate) fn binding_inactive(message: &str) -> anyhow::Error {
    domain("binding_inactive", message, true, None)
}

pub fn project_root(path: &Path) -> Result<String> {
    let path = path
        .canonicalize()
        .map_err(|_| invalid_input("Project path is unavailable"))?;
    ensure!(path.is_dir(), invalid_input("Project must be a directory"));
    let root = path
        .ancestors()
        .find(|p| p.join(".git").exists())
        .unwrap_or(&path);
    Ok(root
        .to_str()
        .ok_or_else(|| invalid_input("Project path must be UTF-8"))?
        .to_owned())
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    AgentRegister {
        name: String,
    },
    AgentList,
    AgentSend {
        to: String,
        body: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subject: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thread_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reply_to: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_id: Option<String>,
        request_id: String,
    },
    AgentInbox {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },
    AgentAck {
        message_id: String,
    },
    AgentWait,
    AgentReady,
    TaskAssign {
        to: String,
        description: String,
        acceptance: String,
        reviewer: Option<String>,
        request_id: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        dependencies: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start_deadline: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        execution_timeout_seconds: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        review_timeout_seconds: Option<u64>,
    },
    TaskList {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        assignee: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        include_archived: bool,
    },
    TaskGet {
        task_id: String,
    },
    TaskHistory {
        task_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },
    TaskStart {
        task_id: String,
        revision: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_version: Option<u64>,
        request_id: String,
    },
    TaskSubmit {
        task_id: String,
        revision: u32,
        result: String,
        evidence: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_version: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attempt_id: Option<String>,
        request_id: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        evidence_ids: Vec<String>,
    },
    TaskReview {
        task_id: String,
        revision: u32,
        accepted: bool,
        feedback: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_version: Option<u64>,
        request_id: String,
    },
    TaskCreatePool {
        description: String,
        acceptance: String,
        eligible: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reviewer: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        dependencies: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start_deadline: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        execution_timeout_seconds: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        review_timeout_seconds: Option<u64>,
        request_id: String,
    },
    TaskClaim {
        task_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_version: Option<u64>,
        request_id: String,
    },
    TaskProgress {
        task_id: String,
        revision: u32,
        note: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attempt_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        waiting_reason: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_version: Option<u64>,
        request_id: String,
    },
    TaskCancel {
        task_id: String,
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_version: Option<u64>,
        request_id: String,
    },
    TaskFinishCancel {
        task_id: String,
        revision: u32,
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attempt_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_version: Option<u64>,
        request_id: String,
    },
    TaskFail {
        task_id: String,
        revision: u32,
        reason: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        evidence_ids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attempt_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_version: Option<u64>,
        request_id: String,
    },
    TaskRetry {
        task_id: String,
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start_deadline: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        clear_start_deadline: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        override_uncertain: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_version: Option<u64>,
        request_id: String,
    },
    TaskReassign {
        task_id: String,
        assignee: String,
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reviewer: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start_deadline: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        clear_start_deadline: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        override_uncertain: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_version: Option<u64>,
        request_id: String,
    },
    TaskSetDependencies {
        task_id: String,
        dependencies: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_version: Option<u64>,
        request_id: String,
    },
    ThreadGet {
        thread_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },
    MessageSearch {
        query: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thread_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },
    EvidenceAdd {
        task_id: String,
        kind: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attempt_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hash: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        commit: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        repository: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        head: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        command: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outcome: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
        request_id: String,
    },
    FileReserve {
        paths: Vec<String>,
        mode: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attempt_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ttl_seconds: Option<u64>,
        request_id: String,
    },
    FileRenew {
        reservation_ids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ttl_seconds: Option<u64>,
        request_id: String,
    },
    FileRelease {
        reservation_ids: Vec<String>,
        request_id: String,
    },
    FileReservations {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        include_expired: bool,
    },
}

impl Operation {
    pub(crate) fn request_id(&self) -> Option<&str> {
        match self {
            Self::AgentSend { request_id, .. }
            | Self::TaskAssign { request_id, .. }
            | Self::TaskStart { request_id, .. }
            | Self::TaskSubmit { request_id, .. }
            | Self::TaskReview { request_id, .. }
            | Self::TaskCreatePool { request_id, .. }
            | Self::TaskClaim { request_id, .. }
            | Self::TaskProgress { request_id, .. }
            | Self::TaskCancel { request_id, .. }
            | Self::TaskFinishCancel { request_id, .. }
            | Self::TaskFail { request_id, .. }
            | Self::TaskRetry { request_id, .. }
            | Self::TaskReassign { request_id, .. }
            | Self::TaskSetDependencies { request_id, .. }
            | Self::EvidenceAdd { request_id, .. }
            | Self::FileReserve { request_id, .. }
            | Self::FileRenew { request_id, .. }
            | Self::FileRelease { request_id, .. } => Some(request_id),
            _ => None,
        }
    }
}

/// Private controller operations for the authenticated local UI; never registered as agent MCP tools.
#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum ControllerOperation {
    SpaceList,
    SpaceCreate {
        name: String,
        request_id: String,
    },
    SpaceJoin {
        space_id: String,
        agent: String,
        request_id: String,
    },
    SpaceLeave {
        space_id: String,
        agent: String,
        request_id: String,
    },
    WorkspaceMap {
        space_id: String,
        root: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        repository_id: Option<String>,
        model: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_commit: Option<String>,
        request_id: String,
    },
    EvidenceVerify {
        evidence_id: String,
        verified: bool,
        request_id: String,
    },
    TaskForceCancel {
        task_id: String,
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_version: Option<u64>,
        request_id: String,
    },
    TaskArchive {
        task_id: String,
        request_id: String,
    },
    ArchiveAged {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        older_than_days: Option<u64>,
        request_id: String,
    },
    HistoryExport {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },
    PurgePreview,
    HistoryPurge {
        archived_tasks: bool,
        acknowledged_messages: bool,
        request_id: String,
    },
    InvitationCreate {
        space_ids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ttl_seconds: Option<u64>,
        request_id: String,
    },
    DeviceList,
    DeviceRevoke {
        device_id: String,
        request_id: String,
    },
}

impl ControllerOperation {
    pub(crate) fn request_id(&self) -> Option<&str> {
        match self {
            Self::SpaceCreate { request_id, .. }
            | Self::SpaceJoin { request_id, .. }
            | Self::SpaceLeave { request_id, .. }
            | Self::WorkspaceMap { request_id, .. }
            | Self::EvidenceVerify { request_id, .. }
            | Self::TaskForceCancel { request_id, .. }
            | Self::TaskArchive { request_id, .. }
            | Self::ArchiveAged { request_id, .. }
            | Self::HistoryPurge { request_id, .. }
            | Self::InvitationCreate { request_id, .. }
            | Self::DeviceRevoke { request_id, .. } => Some(request_id),
            _ => None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub terminal: String,
    pub name: String,
    pub program: String,
    pub project: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub from: String,
    pub to: String,
    pub body: String,
    pub subject: Option<String>,
    pub thread_id: Option<String>,
    pub reply_to: Option<String>,
    pub task_id: Option<String>,
    pub revision: Option<u32>,
    pub kind: String,
    pub acknowledged: bool,
    pub sequence: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Attempt {
    pub id: String,
    pub task_id: String,
    pub revision: u32,
    pub owner: String,
    pub run: String,
    pub certainty: String,
    pub outcome: Option<String>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub project: String,
    pub issuer: String,
    pub assignee: String,
    pub reviewer: String,
    pub description: String,
    pub acceptance: String,
    pub state: String,
    pub revision: u32,
    pub version: u64,
    pub result: Option<String>,
    pub evidence: Option<String>,
    pub feedback: Vec<String>,
    #[serde(default)]
    pub history_truncated: bool,
    pub attempts: Vec<Attempt>,
    pub evidence_records: Vec<Evidence>,
    pub created_seq: u64,
    pub archived: bool,
    pub dependencies: Vec<String>,
    /// Nonempty only for unassigned pool tasks; the list of explicitly eligible agents.
    pub eligible: Vec<String>,
    /// Why a blocked or queued task cannot start, resolved from dependencies and assignee state.
    pub wait_reason: Option<String>,
    pub start_deadline: Option<u64>,
    pub execution_timeout_seconds: Option<u64>,
    pub review_timeout_seconds: Option<u64>,
    pub execution_deadline: Option<u64>,
    pub review_deadline: Option<u64>,
    pub review_overdue: bool,
    // Runtime ownership expires at restart; persisted running tasks require explicit recovery.
    #[serde(skip)]
    pub executing_run: Option<String>,
}

/// Result evidence is a bounded descriptor; a path reference is not a file upload.
#[derive(Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub task_id: String,
    pub attempt_id: Option<String>,
    pub kind: String,
    pub path: Option<String>,
    pub hash: Option<String>,
    pub commit: Option<String>,
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub base: Option<String>,
    pub head: Option<String>,
    pub command: Option<String>,
    pub outcome: Option<String>,
    pub exit_code: Option<i64>,
    pub summary: Option<String>,
    pub device: Option<String>,
    pub verified: bool,
    pub created_seq: u64,
}

/// An advisory coordination lease over exact workspace-relative paths; not a filesystem lock.
#[derive(Clone, Serialize, Deserialize)]
pub struct Reservation {
    pub id: String,
    pub workspace: String,
    pub path: String,
    pub mode: String,
    pub owner: String,
    pub task_id: Option<String>,
    pub attempt_id: Option<String>,
    pub created_at: u64,
    pub created_seq: u64,
    pub expires_at: u64,
    pub expired: bool,
    /// An expired reservation whose owning attempt had no confirmed outcome.
    pub abandoned: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub project: String,
    pub sequence: u64,
    pub kind: String,
    pub actor: String,
    pub resource: Option<String>,
    pub attempt: Option<String>,
    pub observed_at: Option<u64>,
    pub imported: bool,
    pub payload: Value,
}

pub(crate) fn text(value: &str) -> Result<()> {
    limited(
        value,
        MAX_TEXT,
        "Text must be nonempty, at most 8192 bytes, and contain no terminal control characters",
    )
}

pub(crate) fn subject(value: &str) -> Result<()> {
    limited(
        value,
        MAX_SUBJECT,
        "Subject must be nonempty, at most 256 bytes, and contain no terminal control characters",
    )
}

pub(crate) fn search_query(value: &str) -> Result<()> {
    limited(
        value,
        MAX_SUBJECT,
        "Search query must be nonempty, at most 256 bytes, and contain no terminal control characters",
    )
}

fn limited(value: &str, max: usize, message: &str) -> Result<()> {
    if value.trim().is_empty()
        || value.len() > max
        || value
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
    {
        return Err(invalid_input(message));
    }
    Ok(())
}

/// Coordinator UTC deadline from an RFC 3339 string; the caller compares against `Store::now`.
pub(crate) fn start_deadline(value: &str) -> Result<u64> {
    let parsed = chrono::DateTime::parse_from_rfc3339(value.trim())
        .map_err(|_| invalid_input("Deadline must be an RFC 3339 UTC timestamp"))?;
    u64::try_from(parsed.timestamp_millis())
        .map_err(|_| invalid_input("Deadline must not predate the Unix epoch"))
}

pub(crate) fn timeout_seconds(value: u64) -> Result<u64> {
    if value == 0 || value > DEADLINE_TIMEOUT_MAX {
        return Err(invalid_input(
            "Timeout must be between 1 second and 604800 seconds",
        ));
    }
    Ok(value)
}

/// Logical workspace-relative path for reservations: forward slashes, no escapes.
pub(crate) fn normalize_relative_path(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_SUBJECT {
        return Err(invalid_input(
            "Path must be nonempty and at most 256 bytes",
        ));
    }
    if value.starts_with('/') || value.starts_with('\\') {
        return Err(invalid_input("Path must be relative to the workspace root"));
    }
    if value.contains(':') {
        return Err(invalid_input(
            "Path must not contain drive or device prefixes",
        ));
    }
    let mut segments = Vec::new();
    for segment in value.split(['/', '\\']) {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return Err(invalid_input("Path must not traverse parent directories"));
        }
        if segment.chars().any(char::is_control) {
            return Err(invalid_input("Path must contain no control characters"));
        }
        segments.push(segment);
    }
    if segments.is_empty() {
        return Err(invalid_input("Path must name a file or subtree"));
    }
    Ok(segments.join("/"))
}

/// Resolve aliases and the nearest existing parent before granting a physical reservation.
pub(crate) fn normalize_workspace_path(workspace: &str, value: &str) -> Result<String> {
    #[cfg(windows)]
    ensure!(!value.ends_with(['.', ' ']), invalid_input("Path is not a normal Windows file or subtree"));
    let relative = normalize_relative_path(value)?;
    #[cfg(windows)]
    for segment in relative.split('/') {
        let stem = segment.split('.').next().unwrap_or_default().to_ascii_uppercase();
        ensure!(
            !segment.ends_with(['.', ' '])
                && !segment.contains(['<', '>', '"', '|', '?', '*'])
                && !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                && !(stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9')),
            invalid_input("Path is not a normal Windows file or subtree")
        );
    }
    let root = Path::new(workspace).canonicalize()
        .map_err(|_| invalid_input("Workspace root is unavailable"))?;
    let mut existing = root.join(&relative);
    let mut suffix = Vec::new();
    loop {
        match std::fs::symlink_metadata(&existing) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = existing.file_name().ok_or_else(|| invalid_input("Path is unavailable"))?;
                suffix.push(name.to_owned());
                ensure!(existing.pop(), invalid_input("Path is unavailable"));
            }
            Err(_) => return Err(invalid_input("Path cannot be inspected")),
        }
    }
    let mut resolved = existing.canonicalize()
        .map_err(|_| invalid_input("Path alias cannot be resolved"))?;
    ensure!(resolved.starts_with(&root), invalid_input("Path escapes the workspace root"));
    ensure!(suffix.is_empty() || resolved.is_dir(), invalid_input("Path parent must be a directory"));
    for segment in suffix.into_iter().rev() {
        resolved.push(segment);
    }
    let path = resolved.strip_prefix(&root)
        .map_err(|_| invalid_input("Path escapes the workspace root"))?
        .to_str().ok_or_else(|| invalid_input("Path must be UTF-8"))?.replace('\\', "/");
    ensure!(!path.is_empty(), invalid_input("Path must name a file or subtree"));
    // Probe an existing directory alias; do not assume every macOS volume ignores case.
    let insensitive = root.ancestors().find_map(|directory| {
        let name = directory.file_name()?.to_str()?;
        let index = name.bytes().position(|byte| byte.is_ascii_alphabetic())?;
        let mut alternate = name.as_bytes().to_vec();
        alternate[index] ^= 0x20;
        let alternate = directory.with_file_name(String::from_utf8(alternate).ok()?);
        let original = directory.metadata().ok()?;
        let other = alternate.metadata();
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::MetadataExt;
            Some(other.is_ok_and(|other| original.dev() == other.dev() && original.ino() == other.ino()))
        }
        #[cfg(windows)]
        {
            let _ = (original, other);
            Some(alternate.canonicalize().ok().as_deref() == Some(directory))
        }
        #[cfg(not(any(target_os = "macos", windows)))]
        {
            let _ = (original, other);
            Some(false)
        }
    }).unwrap_or(false);
    Ok(if insensitive { path.to_lowercase() } else { path })
}

#[cfg(test)]
mod path_tests {
    use super::*;
    #[test]
    fn reservation_paths_resolve_aliases_and_reject_workspace_escapes() {
        let directory = tempfile::tempdir().unwrap();
        let workspace = directory.path().join("Checkout");
        let outside = directory.path().join("Outside");
        std::fs::create_dir_all(workspace.join("src")).unwrap();
        std::fs::create_dir(&outside).unwrap();
        let root = workspace.to_str().unwrap();
        let target = normalize_workspace_path(root, "src/new/file.rs").unwrap();
        assert!(target.eq_ignore_ascii_case("src/new/file.rs"));
        assert!(normalize_workspace_path(root, "../Outside/file.rs").is_err());
        assert!(normalize_workspace_path(root, "C:\\Outside\\file.rs").is_err());
        #[cfg(target_os = "macos")]
        {
            std::os::unix::fs::symlink(workspace.join("src"), workspace.join("alias")).unwrap();
            std::os::unix::fs::symlink(&outside, workspace.join("escape")).unwrap();
            assert_eq!(normalize_workspace_path(root, "alias/new/file.rs").unwrap(), target);
            assert!(normalize_workspace_path(root, "escape/new/file.rs").is_err());
        }
        #[cfg(windows)]
        for path in ["NUL", "con.txt", "src/file.", "src/file ", "LPT1.txt"] {
            assert!(normalize_workspace_path(root, path).is_err());
        }
    }
}
