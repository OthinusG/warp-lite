//! Local coordination for third-party CLI agents; no model or cloud client lives here.
pub mod launch;
pub mod mcp;
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

pub(crate) fn coordinator_unavailable(message: &str) -> anyhow::Error {
    domain("coordinator_unavailable", message, true, None)
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
    },
    TaskGet {
        task_id: String,
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
}

impl Operation {
    pub(crate) fn request_id(&self) -> Option<&str> {
        match self {
            Self::AgentSend { request_id, .. }
            | Self::TaskAssign { request_id, .. }
            | Self::TaskStart { request_id, .. }
            | Self::TaskSubmit { request_id, .. }
            | Self::TaskReview { request_id, .. } => Some(request_id),
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
    pub attempts: Vec<Attempt>,
    pub created_seq: u64,
    // Runtime ownership expires at restart; persisted running tasks require explicit recovery.
    #[serde(skip)]
    pub executing_run: Option<String>,
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

fn limited(value: &str, max: usize, message: &str) -> Result<()> {
    ensure!(
        !value.trim().is_empty()
            && value.len() <= max
            && !value
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\t')),
        "{}",
        message
    );
    Ok(())
}
