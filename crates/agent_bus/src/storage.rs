//! Versioned, normalized SQLite storage: one write owner, short transactions, events with mutations.
use crate::{
    capacity_exceeded, dependency_blocked, dependency_cycle, epoch_expired, execution_unknown,
    invalid_input, invalid_state, normalize_relative_path, request_conflict, reservation_conflict,
    scope_denied, search_query, stale_attempt, stale_revision, start_deadline as parse_deadline,
    subject as validate_subject, text, timeout_seconds, unauthorized, version_conflict, Agent,
    Attempt, ControllerOperation, Event, Evidence, Message, Operation, Reservation, Task,
    ARCHIVE_AFTER_DAYS, DATABASE_SOFT_LIMIT, MAX_DEPENDENCIES, MAX_ELIGIBLES, MAX_PATHS,
    OPERATOR_NAME, OPERATOR_PROGRAM, PAGE_DEFAULT, PAGE_MAX, RESERVATION_TTL_DEFAULT,
    RESERVATION_TTL_MAX,
};
use anyhow::{anyhow, bail, ensure, Result};
use diesel::{
    connection::SimpleConnection,
    prelude::*,
    sql_types::{BigInt, Integer, Nullable, Text},
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

pub(crate) const SCHEMA_VERSION: &str = "7";
/// The v1 loader ignores `user_version`; this payload makes the old deserializer fail instead of silently writing.
pub(crate) const SENTINEL: &str = r#"{"warp_lite_schema_version":7}"#;
pub(crate) const SENTINEL_V6: &str = r#"{"warp_lite_schema_version":6}"#;
pub(crate) const SENTINEL_V5: &str = r#"{"warp_lite_schema_version":5}"#;
pub(crate) const SENTINEL_V4: &str = r#"{"warp_lite_schema_version":4}"#;
pub(crate) const SENTINEL_V3: &str = r#"{"warp_lite_schema_version":3}"#;
/// The marker of the superseded normalized schema; upgrading from it adds columns and tables.
pub(crate) const SENTINEL_V2: &str = r#"{"warp_lite_schema_version":2}"#;
/// Must exceed the maximum mutation epoch so cleanup can never make an old request execute again.
pub(crate) const REQUEST_RETENTION: Duration = Duration::from_secs(8 * 24 * 60 * 60);
const MAX_AGENTS: i64 = 1000;
const MAX_ACTIVE_TASKS_PER_PROJECT: i64 = 1000;
const MAX_PENDING_PER_AGENT: i64 = 1000;
const CONTROL_MESSAGE_RESERVE: i64 = 100;
const MAX_REQUESTS: i64 = 10_000;
const MAX_EVIDENCE_PER_TASK: i64 = 32;
// Overrides preserve uncertainty across revisions; history cleanup cannot imply execution stopped.
const CERTAIN_TASK: &str = "NOT EXISTS (SELECT 1 FROM attempts WHERE task_id = tasks.id AND (finished_at IS NULL OR certainty = 'unknown'))";
const PURGEABLE_TASK: &str = "archived = 1 AND NOT EXISTS (SELECT 1 FROM task_dependencies WHERE prerequisite_id = tasks.id) AND NOT EXISTS (SELECT 1 FROM messages WHERE task_id = tasks.id AND acknowledged = 0) AND NOT EXISTS (SELECT 1 FROM messages AS parent WHERE parent.task_id = tasks.id AND (EXISTS (SELECT 1 FROM messages AS reply WHERE reply.project = parent.project AND reply.reply_to = parent.id AND (reply.task_id IS NULL OR reply.task_id != tasks.id)) OR EXISTS (SELECT 1 FROM messages AS reply WHERE reply.project = parent.project AND reply.thread_id = parent.id AND (reply.task_id IS NULL OR reply.task_id != tasks.id))))";
const PURGEABLE_MESSAGE: &str = "acknowledged = 1 AND (task_id IS NULL OR task_id NOT IN (SELECT id FROM tasks)) AND NOT EXISTS (SELECT 1 FROM messages AS reply WHERE reply.project = messages.project AND reply.reply_to = messages.id) AND NOT EXISTS (SELECT 1 FROM messages AS reply WHERE reply.project = messages.project AND reply.thread_id = messages.id)";

const MAX_RESERVATIONS_PER_WORKSPACE: i64 = 1000;
/// The operator principal's deterministic identity; it is never stored in the agents table.
pub(crate) const OPERATOR_EPOCH: &str = "operator";

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS agent_bus_v1 (id INTEGER PRIMARY KEY CHECK(id=1), payload TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS agents (id TEXT PRIMARY KEY, terminal TEXT NOT NULL, name TEXT NOT NULL, program TEXT NOT NULL, project TEXT NOT NULL, UNIQUE(project, name));
CREATE TABLE IF NOT EXISTS tasks (id TEXT PRIMARY KEY, project TEXT NOT NULL, issuer TEXT NOT NULL, assignee TEXT NOT NULL, reviewer TEXT NOT NULL, description TEXT NOT NULL, acceptance TEXT NOT NULL, state TEXT NOT NULL, revision INTEGER NOT NULL, version INTEGER NOT NULL, result TEXT, evidence TEXT, created_seq INTEGER NOT NULL, archived INTEGER NOT NULL DEFAULT 0, start_deadline INTEGER, execution_timeout INTEGER, review_timeout INTEGER, execution_deadline INTEGER, review_deadline INTEGER, updated_at INTEGER NOT NULL DEFAULT 0);
CREATE INDEX IF NOT EXISTS tasks_project_seq ON tasks(project, created_seq);
CREATE INDEX IF NOT EXISTS tasks_assignee_state ON tasks(assignee, state, id);
CREATE TABLE IF NOT EXISTS attempts (id TEXT PRIMARY KEY, task_id TEXT NOT NULL, revision INTEGER NOT NULL, owner TEXT NOT NULL, run TEXT NOT NULL, certainty TEXT NOT NULL, outcome TEXT, started_at INTEGER, finished_at INTEGER);
CREATE INDEX IF NOT EXISTS attempts_task ON attempts(task_id);
CREATE INDEX IF NOT EXISTS attempts_run_task ON attempts(run, certainty, task_id);
CREATE TABLE IF NOT EXISTS feedback (id INTEGER PRIMARY KEY AUTOINCREMENT, task_id TEXT NOT NULL, revision INTEGER, author TEXT, body TEXT NOT NULL, accepted INTEGER, created_at INTEGER);
CREATE INDEX IF NOT EXISTS feedback_task ON feedback(task_id, id);
CREATE TABLE IF NOT EXISTS messages (id TEXT PRIMARY KEY, project TEXT NOT NULL, sender TEXT NOT NULL, recipient TEXT NOT NULL, body TEXT NOT NULL, subject TEXT, thread_id TEXT, reply_to TEXT, task_id TEXT, revision INTEGER, kind TEXT NOT NULL, acknowledged INTEGER NOT NULL, sequence INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS messages_recipient ON messages(recipient, acknowledged, sequence);
CREATE INDEX IF NOT EXISTS messages_project ON messages(project, sequence);
CREATE INDEX IF NOT EXISTS messages_thread ON messages(project, thread_id, sequence);
CREATE INDEX IF NOT EXISTS messages_task ON messages(task_id);
CREATE INDEX IF NOT EXISTS messages_reply ON messages(project, reply_to);
CREATE TABLE IF NOT EXISTS events (id TEXT PRIMARY KEY, project TEXT NOT NULL, sequence INTEGER NOT NULL, kind TEXT NOT NULL, actor TEXT NOT NULL, resource TEXT, attempt TEXT, observed_at INTEGER, imported INTEGER NOT NULL, payload TEXT NOT NULL);
CREATE UNIQUE INDEX IF NOT EXISTS events_project_seq ON events(project, sequence);
CREATE INDEX IF NOT EXISTS events_resource_seq ON events(project, resource, sequence);
CREATE TABLE IF NOT EXISTS sequences (project TEXT PRIMARY KEY, value INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS requests (actor TEXT NOT NULL, request_id TEXT NOT NULL, epoch TEXT NOT NULL, fingerprint TEXT NOT NULL, response TEXT NOT NULL, created_at INTEGER NOT NULL, PRIMARY KEY(actor, request_id));
CREATE INDEX IF NOT EXISTS requests_created ON requests(created_at);
CREATE TABLE IF NOT EXISTS task_dependencies (task_id TEXT NOT NULL, prerequisite_id TEXT NOT NULL, position INTEGER NOT NULL, PRIMARY KEY(task_id, prerequisite_id));
CREATE INDEX IF NOT EXISTS task_dependencies_prerequisite ON task_dependencies(prerequisite_id);
CREATE TABLE IF NOT EXISTS task_eligibles (task_id TEXT NOT NULL, agent TEXT NOT NULL, position INTEGER NOT NULL, PRIMARY KEY(task_id, agent));
CREATE TABLE IF NOT EXISTS evidence (id TEXT PRIMARY KEY, task_id TEXT NOT NULL, attempt_id TEXT, kind TEXT NOT NULL, path TEXT, hash TEXT, commit_id TEXT, repository TEXT, branch TEXT, base TEXT, head TEXT, command TEXT, outcome TEXT, exit_code INTEGER, summary TEXT, device TEXT, verified INTEGER NOT NULL, created_seq INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS evidence_task ON evidence(task_id);
CREATE TABLE IF NOT EXISTS reservations (id TEXT PRIMARY KEY, workspace TEXT NOT NULL, path TEXT NOT NULL, mode TEXT NOT NULL, owner TEXT NOT NULL, task_id TEXT, attempt_id TEXT, created_at INTEGER NOT NULL, expires_at INTEGER NOT NULL, created_seq INTEGER NOT NULL, space_id TEXT, repository_id TEXT, workspace_id TEXT);
CREATE INDEX IF NOT EXISTS reservations_workspace ON reservations(workspace, created_seq);
CREATE TABLE IF NOT EXISTS spaces (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, device TEXT NOT NULL, created_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS workspaces (id TEXT PRIMARY KEY, space_id TEXT NOT NULL, root TEXT NOT NULL, repository_id TEXT, model TEXT NOT NULL, branch TEXT, base_commit TEXT, created_at INTEGER NOT NULL);
CREATE UNIQUE INDEX IF NOT EXISTS workspaces_root ON workspaces(root);
CREATE INDEX IF NOT EXISTS workspaces_repository ON workspaces(space_id, repository_id);
CREATE TABLE IF NOT EXISTS space_members (space_id TEXT NOT NULL, agent TEXT NOT NULL, position INTEGER NOT NULL, PRIMARY KEY(space_id, agent));
CREATE TABLE IF NOT EXISTS agent_workspace_bindings (agent TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, space_id TEXT NOT NULL, revoked INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS devices (id TEXT PRIMARY KEY, name TEXT NOT NULL, verifier TEXT NOT NULL, generation INTEGER NOT NULL, revoked INTEGER NOT NULL, created_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS device_spaces (device_id TEXT NOT NULL, space_id TEXT NOT NULL, mode TEXT NOT NULL, PRIMARY KEY(device_id, space_id));
CREATE TABLE IF NOT EXISTS invitations (id TEXT PRIMARY KEY, verifier TEXT NOT NULL, space_ids TEXT NOT NULL, expires_at INTEGER NOT NULL, consumed INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS cursors (device_id TEXT NOT NULL, space_id TEXT NOT NULL, sequence INTEGER NOT NULL, PRIMARY KEY(device_id, space_id));
CREATE TABLE IF NOT EXISTS remote_workspaces (id TEXT PRIMARY KEY, device TEXT NOT NULL, space_id TEXT NOT NULL, checkout TEXT NOT NULL, label TEXT NOT NULL, repository_id TEXT, created_at INTEGER NOT NULL, UNIQUE(device, checkout));
CREATE TABLE IF NOT EXISTS remote_actor_bindings (agent TEXT PRIMARY KEY, device TEXT NOT NULL, workspace_id TEXT NOT NULL, space_id TEXT NOT NULL, native_id TEXT NOT NULL, current_epoch TEXT, revoked INTEGER NOT NULL DEFAULT 0, UNIQUE(device, native_id));
CREATE TABLE IF NOT EXISTS remote_runs (epoch TEXT PRIMARY KEY, agent TEXT NOT NULL, native_run TEXT NOT NULL, expires_at INTEGER NOT NULL, closed INTEGER NOT NULL DEFAULT 0, UNIQUE(agent, native_run));
CREATE TABLE IF NOT EXISTS remote_pending_intents (coordinator TEXT NOT NULL, device TEXT NOT NULL, space TEXT NOT NULL, actor TEXT NOT NULL, epoch TEXT NOT NULL, request_id TEXT NOT NULL, operation TEXT NOT NULL, response TEXT, created_at INTEGER NOT NULL, PRIMARY KEY(coordinator, actor, request_id));";

pub(crate) const TASK_COLUMNS: &str = "id, project, issuer, assignee, reviewer, description, acceptance, state, revision, version, result, evidence, created_seq, archived, start_deadline, execution_timeout, review_timeout, execution_deadline, review_deadline";

/// App-selected local workspace; native tool arguments cannot construct this binding.
#[derive(Clone, Debug)]
pub struct WorkspaceBinding {
    pub id: String,
    pub space: String,
    pub root: String,
}
impl WorkspaceBinding {
    pub fn domain(&self) -> String { format!("space:{}", self.space) }
}

/// Single-threaded behind the broker mutex; RefCell keeps short diesel borrows from leaking into APIs.
pub struct Store {
    connection: RefCell<SqliteConnection>,
}

impl Store {
    pub fn open(path: &str) -> Result<Self> {
        #[cfg(unix)]
        if path != ":memory:" {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .mode(0o600)
                .open(path)?;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        let mut connection = SqliteConnection::establish(path)?;
        connection.batch_execute("PRAGMA busy_timeout=5000;")?;
        let store = Self {
            connection: RefCell::new(connection),
        };
        store.prepare_schema(path)?;
        store.cleanup_requests()?;
        Ok(store)
    }

    fn prepare_schema(&self, path: &str) -> Result<()> {
        match self.meta_version()? {
            Some(version) if version == SCHEMA_VERSION => self.create_schema(),
            Some(version) if version == "6" => self.upgrade_additive(path, SENTINEL_V6),
            Some(version) if version == "5" => self.upgrade_additive(path, SENTINEL_V5),
            Some(version) if version == "4" => self.upgrade_additive(path, SENTINEL_V4),
            Some(version) if version == "3" => self.upgrade_v3(path),
            Some(version) if version == "2" => self.upgrade_v2(path),
            Some(version) => bail!("Unsupported agent bus schema version: {version}"),
            None => self.upgrade(path),
        }
    }

    fn create_schema(&self) -> Result<()> {
        self.connection.borrow_mut().batch_execute(SCHEMA)?;
        Ok(())
    }

    fn upgrade_additive(&self, path: &str, sentinel: &str) -> Result<()> {
        self.backup(path, Some(sentinel))?;
        self.transaction(|| {
            self.create_schema()?;
            self.set_legacy_payload(SENTINEL)?;
            self.set_meta_version()
        })
    }

    fn upgrade_v3(&self, path: &str) -> Result<()> {
        self.backup(path, Some(SENTINEL_V3))?;
        self.transaction(|| {
            self.connection.borrow_mut().batch_execute(
                "ALTER TABLE workspaces ADD COLUMN repository_id TEXT;
                 ALTER TABLE reservations ADD COLUMN space_id TEXT;
                 ALTER TABLE reservations ADD COLUMN repository_id TEXT;
                 ALTER TABLE reservations ADD COLUMN workspace_id TEXT;",
            )?;
            self.create_schema()?;
            self.set_legacy_payload(SENTINEL)?;
            self.set_meta_version()
        })
    }

    fn meta_version(&self) -> Result<Option<String>> {
        let tables = diesel::sql_query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='meta'",
        )
        .load::<NameRow>(&mut *self.connection.borrow_mut())?;
        if tables.is_empty() {
            return Ok(None);
        }
        Ok(
            diesel::sql_query("SELECT value FROM meta WHERE key='schema_version'")
                .get_result::<ValueRow>(&mut *self.connection.borrow_mut())
                .optional()?
                .map(|row| row.value),
        )
    }

    /// Additive v2 upgrade: new tables and task columns; existing rows are preserved as-is.
    fn upgrade_v2(&self, path: &str) -> Result<()> {
        self.backup(path, Some(SENTINEL_V2))?;
        self.transaction(|| {
            self.create_schema()?;
            for statement in [
                "ALTER TABLE tasks ADD COLUMN archived INTEGER NOT NULL DEFAULT 0",
                "ALTER TABLE tasks ADD COLUMN start_deadline INTEGER",
                "ALTER TABLE tasks ADD COLUMN execution_timeout INTEGER",
                "ALTER TABLE tasks ADD COLUMN review_timeout INTEGER",
                "ALTER TABLE tasks ADD COLUMN execution_deadline INTEGER",
                "ALTER TABLE tasks ADD COLUMN review_deadline INTEGER",
                "ALTER TABLE tasks ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
            ] {
                self.connection.borrow_mut().batch_execute(statement)?;
            }
            // Historical age is unknown; anchor it at migration time so aged archiving never fires retroactively.
            diesel::sql_query("UPDATE tasks SET updated_at = ? WHERE updated_at = 0")
                .bind::<BigInt, _>(now() as i64)
                .execute(&mut *self.connection.borrow_mut())?;
            self.set_legacy_payload(SENTINEL)?;
            self.set_meta_version()?;
            Ok(())
        })
    }

    /// Crash before commit leaves the old schema usable; crash after commit reopens the new one.
    fn upgrade(&self, path: &str) -> Result<()> {
        match read_legacy_payload(path)? {
            None => self.transaction(|| {
                self.create_schema()?;
                self.set_legacy_payload(SENTINEL)?;
                self.set_meta_version()?;
                Ok(())
            }),
            Some(payload) if payload == SENTINEL => bail!(
                "Agent bus database is marked as migrated but its schema marker is missing; restore the pre-upgrade backup"
            ),
            Some(payload) if payload == SENTINEL_V2 || payload == SENTINEL_V3 || payload == SENTINEL_V4 || payload == SENTINEL_V5 || payload == SENTINEL_V6 => {
                bail!("Agent bus schema marker and meta version disagree; restore the pre-upgrade backup")
            }
            Some(payload) => self.migrate(path, &payload),
        }
    }

    /// One consistent copy before any destructive migration; a crash may leave it in place for retry.
    fn backup(&self, path: &str, expected: Option<&str>) -> Result<()> {
        if path == ":memory:" {
            return Ok(());
        }
        // Preserve the v1 backup when a later normalized store is upgraded again.
        let backup = if expected == Some(SENTINEL_V2) {
            format!("{path}.pre-upgrade-v2")
        } else if expected == Some(SENTINEL_V6) {
            format!("{path}.pre-upgrade-v6")
        } else if expected == Some(SENTINEL_V5) {
            format!("{path}.pre-upgrade-v5")
        } else if expected == Some(SENTINEL_V4) {
            format!("{path}.pre-upgrade-v4")
        } else if expected == Some(SENTINEL_V3) {
            format!("{path}.pre-upgrade-v3")
        } else {
            format!("{path}.pre-upgrade")
        };
        if std::path::Path::new(&backup).exists() {
            if let Some(expected) = expected {
                let stored = read_legacy_payload(&backup)?;
                ensure!(
                    stored.as_deref() == Some(expected),
                    "Existing migration backup does not match this database; refusing to overwrite"
                );
            }
            return Ok(());
        }
        let escaped = backup.replace('\'', "''");
        self.connection
            .borrow_mut()
            .batch_execute(&format!("VACUUM INTO '{escaped}'"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&backup, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    fn migrate(&self, path: &str, payload: &str) -> Result<()> {
        let legacy: LegacySnapshot = serde_json::from_str(payload)
            .map_err(|_| anyhow!("Unrecognized agent bus database content; refusing to migrate"))?;
        self.backup(path, Some(payload))?;
        self.transaction(|| {
            self.create_schema()?;
            for agent in legacy.agents.values() {
                self.insert_agent(&Agent {
                    id: agent.id.clone(),
                    terminal: agent.terminal.clone(),
                    name: agent.name.clone(),
                    program: agent.program.clone(),
                    project: agent.project.clone(),
                })?;
            }
            // Migrated tasks take sequence numbers above historical messages so merged export cursors stay unique.
            let mut created: BTreeMap<&str, u64> = BTreeMap::new();
            for task in legacy.tasks.values() {
                let created_seq = created.entry(&task.project).or_insert(legacy.sequence);
                *created_seq += 1;
                let task = Task {
                    id: task.id.clone(),
                    project: task.project.clone(),
                    issuer: task.issuer.clone(),
                    assignee: task.assignee.clone(),
                    reviewer: task.reviewer.clone(),
                    description: task.description.clone(),
                    acceptance: task.acceptance.clone(),
                    state: task.state.clone(),
                    revision: task.revision,
                    version: 1,
                    result: task.result.clone(),
                    evidence: task.evidence.clone(),
                    feedback: task.feedback.clone(),
                    history_truncated: false,
                    attempts: vec![],
                    evidence_records: vec![],
                    created_seq: *created_seq,
                    archived: false,
                    dependencies: vec![],
                    eligible: vec![],
                    wait_reason: None,
                    start_deadline: None,
                    execution_timeout_seconds: None,
                    review_timeout_seconds: None,
                    execution_deadline: None,
                    review_deadline: None,
                    review_overdue: false,
                    executing_run: None,
                };
                self.insert_task(&task)?;
                for body in &task.feedback {
                    diesel::sql_query("INSERT INTO feedback(task_id, revision, author, body, accepted, created_at) VALUES (?, NULL, NULL, ?, NULL, NULL)")
                        .bind::<Text, _>(&task.id)
                        .bind::<Text, _>(body)
                        .execute(&mut *self.connection.borrow_mut())?;
                }
            }
            for message in &legacy.messages {
                let project = legacy
                    .agents
                    .get(&message.from)
                    .zip(legacy.agents.get(&message.to))
                    .map(|(sender, _)| sender.project.clone())
                    .ok_or_else(|| {
                        anyhow!("Message references an unknown agent; refusing to migrate")
                    })?;
                let message = Message {
                    id: message.id.clone(),
                    from: message.from.clone(),
                    to: message.to.clone(),
                    body: message.body.clone(),
                    subject: None,
                    thread_id: None,
                    reply_to: None,
                    task_id: message.task_id.clone(),
                    revision: message.revision,
                    kind: message.kind.clone(),
                    acknowledged: message.acknowledged,
                    sequence: message.sequence,
                };
                self.insert_message(&project, &message)?;
            }
            for (key, retry) in &legacy.retries {
                let (actor, request_id) = key.split_once(':').ok_or_else(|| {
                    anyhow!("Invalid legacy request key; refusing to migrate")
                })?;
                diesel::sql_query("INSERT INTO requests(actor, request_id, epoch, fingerprint, response, created_at) VALUES (?, ?, 'legacy', ?, ?, ?)")
                    .bind::<Text, _>(actor)
                    .bind::<Text, _>(request_id)
                    .bind::<Text, _>(&retry.operation)
                    .bind::<Text, _>(retry.result.to_string())
                    .bind::<BigInt, _>(now() as i64)
                    .execute(&mut *self.connection.borrow_mut())?;
            }
            ensure!(
                self.agent_count()? as usize == legacy.agents.len()
                    && self.task_count_all()? as usize == legacy.tasks.len()
                    && self.message_count_all()? as usize == legacy.messages.len()
                    && self.feedback_count()? as usize
                        == legacy.tasks.values().map(|t| t.feedback.len()).sum::<usize>(),
                "Migration row counts do not match the v1 snapshot; refusing to commit"
            );
            let mut sequences: BTreeMap<&str, u64> = BTreeMap::new();
            for agent in legacy.agents.values() {
                sequences.insert(&agent.project, legacy.sequence);
            }
            for (project, value) in &created {
                if let Some(sequence) = sequences.get_mut(*project) {
                    *sequence = (*sequence).max(*value);
                }
            }
            for (project, value) in sequences {
                diesel::sql_query("INSERT INTO sequences(project, value) VALUES (?, ?) ON CONFLICT(project) DO UPDATE SET value=excluded.value")
                    .bind::<Text, _>(project)
                    .bind::<BigInt, _>(value as i64)
                    .execute(&mut *self.connection.borrow_mut())?;
            }
            self.set_legacy_payload(SENTINEL)?;
            self.set_meta_version()?;
            Ok(())
        })
    }

    fn set_legacy_payload(&self, payload: &str) -> Result<()> {
        diesel::sql_query("INSERT INTO agent_bus_v1(id, payload) VALUES (1, ?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload")
            .bind::<Text, _>(payload)
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    fn set_meta_version(&self) -> Result<()> {
        // Version 7 is also an authority cutover: older writers must refuse this store.
        // Preserve unknown effects and original intents instead of inventing cancellation.
        let unfinished = diesel::sql_query("SELECT id,task_id,revision,owner,run,certainty,outcome,started_at,finished_at FROM attempts WHERE finished_at IS NULL AND certainty='active' AND owner IN (SELECT agent FROM remote_actor_bindings)")
            .load::<AttemptRow>(&mut *self.connection.borrow_mut())?;
        for attempt in unfinished {
            diesel::sql_query("UPDATE attempts SET certainty='unknown' WHERE id=?")
                .bind::<Text, _>(&attempt.id).execute(&mut *self.connection.borrow_mut())?;
            diesel::sql_query("UPDATE tasks SET version=version+1 WHERE id=?")
                .bind::<Text, _>(&attempt.task_id).execute(&mut *self.connection.borrow_mut())?;
            let project = diesel::sql_query("SELECT project AS value FROM tasks WHERE id=?")
                .bind::<Text, _>(&attempt.task_id)
                .get_result::<ValueRow>(&mut *self.connection.borrow_mut())?.value;
            self.record(&project, "task_execution_unknown", &attempt.owner,
                Some(&attempt.task_id), Some(&attempt.id),
                json!({"revision": attempt.revision, "reason": "legacy_device_retired", "execution_stopped": false}))?;
        }
        self.connection.borrow_mut().batch_execute(
            "UPDATE devices SET revoked=1, generation=generation+1 WHERE revoked=0;
             UPDATE invitations SET consumed=1;
             UPDATE remote_actor_bindings SET revoked=1;
             UPDATE remote_runs SET closed=1;",
        )?;
        diesel::sql_query("INSERT INTO meta(key, value) VALUES ('schema_version', ?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
            .bind::<Text, _>(SCHEMA_VERSION)
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    fn cleanup_requests(&self) -> Result<()> {
        let cutoff = now().saturating_sub(REQUEST_RETENTION.as_millis() as u64);
        diesel::sql_query("DELETE FROM requests WHERE created_at < ?")
            .bind::<BigInt, _>(cutoff as i64)
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    fn transaction<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        self.connection.borrow_mut().batch_execute("BEGIN IMMEDIATE")?;
        match f() {
            Ok(value) => {
                let committed = self.connection.borrow_mut().batch_execute("COMMIT");
                match committed {
                    Ok(()) => Ok(value),
                    Err(error) => {
                        // SQLite can leave a transaction open after a failed commit.
                        let _ = self.connection.borrow_mut().batch_execute("ROLLBACK");
                        Err(error.into())
                    }
                }
            }
            Err(error) => {
                let _ = self.connection.borrow_mut().batch_execute("ROLLBACK");
                Err(error)
            }
        }
    }

    fn insert_agent(&self, agent: &Agent) -> Result<()> {
        diesel::sql_query("INSERT INTO agents(id, terminal, name, program, project) VALUES (?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET terminal=excluded.terminal, name=excluded.name, program=excluded.program, project=excluded.project")
            .bind::<Text, _>(&agent.id)
            .bind::<Text, _>(&agent.terminal)
            .bind::<Text, _>(&agent.name)
            .bind::<Text, _>(&agent.program)
            .bind::<Text, _>(&agent.project)
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    fn insert_task(&self, task: &Task) -> Result<()> {
        diesel::sql_query("INSERT INTO tasks(id, project, issuer, assignee, reviewer, description, acceptance, state, revision, version, result, evidence, created_seq, archived, start_deadline, execution_timeout, review_timeout, execution_deadline, review_deadline, updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind::<Text, _>(&task.id)
            .bind::<Text, _>(&task.project)
            .bind::<Text, _>(&task.issuer)
            .bind::<Text, _>(&task.assignee)
            .bind::<Text, _>(&task.reviewer)
            .bind::<Text, _>(&task.description)
            .bind::<Text, _>(&task.acceptance)
            .bind::<Text, _>(&task.state)
            .bind::<Integer, _>(task.revision as i32)
            .bind::<BigInt, _>(task.version as i64)
            .bind::<Nullable<Text>, _>(&task.result)
            .bind::<Nullable<Text>, _>(&task.evidence)
            .bind::<BigInt, _>(task.created_seq as i64)
            .bind::<Integer, _>(i32::from(task.archived))
            .bind::<Nullable<BigInt>, _>(task.start_deadline.map(|value| value as i64))
            .bind::<Nullable<BigInt>, _>(task.execution_timeout_seconds.map(|value| value as i64))
            .bind::<Nullable<BigInt>, _>(task.review_timeout_seconds.map(|value| value as i64))
            .bind::<Nullable<BigInt>, _>(task.execution_deadline.map(|value| value as i64))
            .bind::<Nullable<BigInt>, _>(task.review_deadline.map(|value| value as i64))
            .bind::<BigInt, _>(now() as i64)
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    fn update_task_state(&self, task: &Task) -> Result<()> {
        diesel::sql_query("UPDATE tasks SET assignee=?, reviewer=?, state=?, revision=?, version=?, result=?, evidence=?, archived=?, start_deadline=?, execution_timeout=?, review_timeout=?, execution_deadline=?, review_deadline=?, updated_at=? WHERE id=?")
            .bind::<Text, _>(&task.assignee)
            .bind::<Text, _>(&task.reviewer)
            .bind::<Text, _>(&task.state)
            .bind::<Integer, _>(task.revision as i32)
            .bind::<BigInt, _>(task.version as i64)
            .bind::<Nullable<Text>, _>(&task.result)
            .bind::<Nullable<Text>, _>(&task.evidence)
            .bind::<Integer, _>(i32::from(task.archived))
            .bind::<Nullable<BigInt>, _>(task.start_deadline.map(|value| value as i64))
            .bind::<Nullable<BigInt>, _>(task.execution_timeout_seconds.map(|value| value as i64))
            .bind::<Nullable<BigInt>, _>(task.review_timeout_seconds.map(|value| value as i64))
            .bind::<Nullable<BigInt>, _>(task.execution_deadline.map(|value| value as i64))
            .bind::<Nullable<BigInt>, _>(task.review_deadline.map(|value| value as i64))
            .bind::<BigInt, _>(now() as i64)
            .bind::<Text, _>(&task.id)
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    fn set_task_dependencies(&self, task_id: &str, dependencies: &[String]) -> Result<()> {
        diesel::sql_query("DELETE FROM task_dependencies WHERE task_id = ?")
            .bind::<Text, _>(task_id)
            .execute(&mut *self.connection.borrow_mut())?;
        for (position, prerequisite) in dependencies.iter().enumerate() {
            diesel::sql_query("INSERT INTO task_dependencies(task_id, prerequisite_id, position) VALUES (?,?,?)")
                .bind::<Text, _>(task_id)
                .bind::<Text, _>(prerequisite)
                .bind::<Integer, _>(position as i32)
                .execute(&mut *self.connection.borrow_mut())?;
        }
        Ok(())
    }

    fn task_dependencies(&self, task_id: &str) -> Result<Vec<String>> {
        Ok(diesel::sql_query("SELECT prerequisite_id AS value FROM task_dependencies WHERE task_id = ? ORDER BY position")
            .bind::<Text, _>(task_id)
            .load::<ValueRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(|row| row.value)
            .collect())
    }

    fn set_task_eligibles(&self, task_id: &str, eligible: &[String]) -> Result<()> {
        diesel::sql_query("DELETE FROM task_eligibles WHERE task_id = ?")
            .bind::<Text, _>(task_id)
            .execute(&mut *self.connection.borrow_mut())?;
        for (position, agent) in eligible.iter().enumerate() {
            diesel::sql_query("INSERT INTO task_eligibles(task_id, agent, position) VALUES (?,?,?)")
                .bind::<Text, _>(task_id)
                .bind::<Text, _>(agent)
                .bind::<Integer, _>(position as i32)
                .execute(&mut *self.connection.borrow_mut())?;
        }
        Ok(())
    }

    fn task_eligibles(&self, task_id: &str) -> Result<Vec<String>> {
        Ok(diesel::sql_query("SELECT agent AS value FROM task_eligibles WHERE task_id = ? ORDER BY position")
            .bind::<Text, _>(task_id)
            .load::<ValueRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(|row| row.value)
            .collect())
    }

    fn insert_message(&self, project: &str, message: &Message) -> Result<()> {
        diesel::sql_query("INSERT INTO messages(id, project, sender, recipient, body, subject, thread_id, reply_to, task_id, revision, kind, acknowledged, sequence) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind::<Text, _>(&message.id)
            .bind::<Text, _>(project)
            .bind::<Text, _>(&message.from)
            .bind::<Text, _>(&message.to)
            .bind::<Text, _>(&message.body)
            .bind::<Nullable<Text>, _>(&message.subject)
            .bind::<Nullable<Text>, _>(&message.thread_id)
            .bind::<Nullable<Text>, _>(&message.reply_to)
            .bind::<Nullable<Text>, _>(&message.task_id)
            .bind::<Nullable<Integer>, _>(message.revision.map(|revision| revision as i32))
            .bind::<Text, _>(&message.kind)
            .bind::<Integer, _>(i32::from(message.acknowledged))
            .bind::<BigInt, _>(message.sequence as i64)
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    fn next_sequence(&self, project: &str) -> Result<u64> {
        let mut connection = self.connection.borrow_mut();
        diesel::sql_query("INSERT INTO sequences(project, value) VALUES (?, 0) ON CONFLICT(project) DO NOTHING")
            .bind::<Text, _>(project)
            .execute(&mut *connection)?;
        diesel::sql_query("UPDATE sequences SET value = value + 1 WHERE project = ?")
            .bind::<Text, _>(project)
            .execute(&mut *connection)?;
        let row = diesel::sql_query("SELECT value FROM sequences WHERE project = ?")
            .bind::<Text, _>(project)
            .get_result::<SequenceRow>(&mut *connection)?;
        Ok(row.value as u64)
    }

    /// Events commit atomically with the mutation they describe.
    fn record(
        &self,
        project: &str,
        kind: &str,
        actor: &str,
        resource: Option<&str>,
        attempt: Option<&str>,
        payload: Value,
    ) -> Result<u64> {
        let sequence = self.next_sequence(project)?;
        diesel::sql_query("INSERT INTO events(id, project, sequence, kind, actor, resource, attempt, observed_at, imported, payload) VALUES (?,?,?,?,?,?,?,?,0,?)")
            .bind::<Text, _>(Uuid::new_v4().to_string())
            .bind::<Text, _>(project)
            .bind::<BigInt, _>(sequence as i64)
            .bind::<Text, _>(kind)
            .bind::<Text, _>(actor)
            .bind::<Nullable<Text>, _>(resource)
            .bind::<Nullable<Text>, _>(attempt)
            .bind::<BigInt, _>(now() as i64)
            .bind::<Text, _>(serde_json::to_string(&payload)?)
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(sequence)
    }

    /// A native prompt observation is never an acknowledgement, task transition or execution proof.
    pub(crate) fn observe_native_delivery(&self, actor: &Agent, run: &str, message: &str,
        task: Option<&str>, revision: Option<u32>, phase: &str) -> Result<()> {
        ensure!(matches!(phase, "claimed" | "submitted" | "cancelled"), invalid_input("Invalid native delivery phase"));
        self.transaction(|| {
            self.authorize(actor)?;
            self.budget_available()?;
            self.record(&actor.project, "native_delivery_observed", &actor.id, task.or(Some(message)), None,
                json!({"message_id":message, "revision":revision, "run":run, "phase":phase,
                    "acknowledgement_implied":false, "execution_implied":false}))?;
            Ok(())
        })
    }

    fn queue(&self, project: &str, mut message: Message) -> Result<Message> {
        let control = !matches!(message.kind.as_str(), "message" | "assignment" | "available");
        let limit = MAX_PENDING_PER_AGENT + if control { CONTROL_MESSAGE_RESERVE } else { 0 };
        ensure!(
            self.inbox_count(&message.to)? < limit,
            capacity_exceeded("Recipient inbox is full; acknowledge pending messages before sending more work")
        );
        message.id = Uuid::new_v4().to_string();
        message.sequence = self.next_sequence(project)?;
        message.acknowledged = false;
        self.insert_message(project, &message)?;
        self.record(
            project,
            "message_queued",
            &message.from,
            Some(&message.id),
            None,
            json!({"kind": message.kind, "to": message.to, "task_id": message.task_id}),
        )?;
        Ok(message)
    }

    fn count(&self, sql: &str, binds: &[&str]) -> Result<i64> {
        let mut connection = self.connection.borrow_mut();
        let query = diesel::sql_query(sql.to_owned());
        let count = match binds {
            [] => query.get_result::<CountRow>(&mut *connection)?,
            [a] => query
                .bind::<Text, _>(*a)
                .get_result::<CountRow>(&mut *connection)?,
            [a, b] => query
                .bind::<Text, _>(*a)
                .bind::<Text, _>(*b)
                .get_result::<CountRow>(&mut *connection)?,
            [a, b, c] => query
                .bind::<Text, _>(*a)
                .bind::<Text, _>(*b)
                .bind::<Text, _>(*c)
                .get_result::<CountRow>(&mut *connection)?,
            [a, b, c, d] => query
                .bind::<Text, _>(*a)
                .bind::<Text, _>(*b)
                .bind::<Text, _>(*c)
                .bind::<Text, _>(*d)
                .get_result::<CountRow>(&mut *connection)?,
            _ => bail!("Too many bind parameters"),
        };
        Ok(count.count)
    }

    fn agent_count(&self) -> Result<i64> {
        self.count("SELECT COUNT(*) AS count FROM agents", &[])
    }

    fn task_count_all(&self) -> Result<i64> {
        self.count("SELECT COUNT(*) AS count FROM tasks", &[])
    }

    fn message_count_all(&self) -> Result<i64> {
        self.count("SELECT COUNT(*) AS count FROM messages", &[])
    }

    fn feedback_count(&self) -> Result<i64> {
        self.count("SELECT COUNT(*) AS count FROM feedback", &[])
    }

    fn task_count(&self, project: &str) -> Result<i64> {
        self.count(
            "SELECT COUNT(*) AS count FROM tasks WHERE project = ?",
            &[project],
        )
    }

    fn active_task_count(&self, project: &str) -> Result<i64> {
        self.count("SELECT COUNT(*) AS count FROM tasks WHERE project = ? AND archived = 0", &[project])
    }

    fn pending_count(&self, project: &str) -> Result<i64> {
        self.count(
            "SELECT COUNT(*) AS count FROM messages WHERE project = ? AND acknowledged = 0",
            &[project],
        )
    }

    fn message_count(&self, project: &str) -> Result<i64> {
        self.count(
            "SELECT COUNT(*) AS count FROM messages WHERE project = ?",
            &[project],
        )
    }

    fn request_count(&self) -> Result<i64> {
        self.count("SELECT COUNT(*) AS count FROM requests", &[])
    }

    pub fn capacity(&self, project: &str) -> Result<Value> {
        let bytes = self.database_size()?;
        Ok(json!({
            "agents": {"used": self.agent_count()?, "limit": MAX_AGENTS},
            "tasks": {"used": self.active_task_count(project)?, "total": self.task_count(project)?, "limit": MAX_ACTIVE_TASKS_PER_PROJECT},
            "pending_messages": {"used": self.pending_count(project)?, "per_agent_limit": MAX_PENDING_PER_AGENT, "control_reserve": CONTROL_MESSAGE_RESERVE},
            "messages": {"used": self.message_count(project)?},
            "database": {"used_bytes": bytes, "soft_limit": DATABASE_SOFT_LIMIT, "hard_limit": crate::DATABASE_HARD_LIMIT},
        }))
    }

    fn database_size(&self) -> Result<u64> {
        let row = diesel::sql_query("SELECT ((SELECT page_count FROM pragma_page_count()) - (SELECT freelist_count FROM pragma_freelist_count())) * (SELECT page_size FROM pragma_page_size()) AS value")
            .get_result::<BytesRow>(&mut *self.connection.borrow_mut())?;
        Ok(row.value.max(0) as u64)
    }

    /// Near the configured budget only new work is refused; acknowledgements and control records still commit.
    fn budget_available(&self) -> Result<()> {
        ensure!(
            self.database_size()? < crate::DATABASE_HARD_LIMIT,
            capacity_exceeded("Database budget reached; export and purge eligible history")
        );
        Ok(())
    }

    fn agent_by_name(&self, project: &str, name: &str) -> Result<Option<Agent>> {
        Ok(diesel::sql_query("SELECT id, terminal, name, program, project FROM agents WHERE project = ? AND name = ?")
            .bind::<Text, _>(project)
            .bind::<Text, _>(name)
            .get_result::<AgentRow>(&mut *self.connection.borrow_mut())
            .optional()?
            .map(AgentRow::agent))
    }

    fn agent_by_terminal(&self, terminal: &str, program: &str, project: &str) -> Result<Option<Agent>> {
        Ok(diesel::sql_query("SELECT id, terminal, name, program, project FROM agents WHERE terminal = ? AND program = ? AND project = ?")
            .bind::<Text, _>(terminal)
            .bind::<Text, _>(program)
            .bind::<Text, _>(project)
            .get_result::<AgentRow>(&mut *self.connection.borrow_mut())
            .optional()?
            .map(AgentRow::agent))
    }

    pub(crate) fn workspace_binding(&self, id: &str) -> Result<WorkspaceBinding> {
        let row = diesel::sql_query("SELECT id, space_id, root, repository_id, model, branch, base_commit FROM workspaces WHERE id = ?")
            .bind::<Text, _>(id)
            .get_result::<WorkspaceRow>(&mut *self.connection.borrow_mut()).optional()?
            .ok_or_else(|| scope_denied("Mapped workspace not found"))?;
        self.space(&row.space_id)?
            .ok_or_else(|| scope_denied("Space not found"))?;
        Ok(WorkspaceBinding {
            id: row.id,
            space: row.space_id,
            root: row.root,
        })
    }

    pub(crate) fn authorize_workspace(&self, binding: &WorkspaceBinding) -> Result<()> {
        let current = self.workspace_binding(&binding.id)?;
        ensure!(
            current.space == binding.space && current.root == binding.root,
            scope_denied("Workspace mapping changed; open a new shared pane")
        );
        Ok(())
    }

    /// Membership is checked before reads, dedup replay and delivery, rather than only at registration.
    pub(crate) fn authorize(&self, actor: &Agent) -> Result<()> {
        if !actor.project.starts_with("space:") || actor.program == OPERATOR_PROGRAM {
            return Ok(());
        }
        ensure!(self.remote_physical_root(actor)?.is_none(),
            scope_denied("Legacy remote participation was revoked"));
        ensure!(self.count("SELECT COUNT(*) AS count FROM agent_workspace_bindings AS b JOIN workspaces AS w ON w.id=b.workspace_id AND w.space_id=b.space_id JOIN space_members AS m ON m.space_id=b.space_id AND m.agent=b.agent WHERE b.agent=? AND b.revoked=0 AND ?='space:' || b.space_id", &[&actor.id, &actor.project])? == 1,
            scope_denied("Shared participation was revoked; open a new shared pane"));
        Ok(())
    }

    /// Retired device provenance is read-only; it never identifies a local checkout.
    fn remote_physical_root(&self, actor: &Agent) -> Result<Option<String>> {
        Ok(diesel::sql_query("SELECT 'remote:' || b.device || ':' || w.checkout AS value FROM remote_actor_bindings AS b JOIN remote_workspaces AS w ON w.id=b.workspace_id WHERE b.agent=?")
            .bind::<Text, _>(&actor.id).get_result::<ValueRow>(&mut *self.connection.borrow_mut()).optional()?
            .map(|row| row.value))
    }

    fn reservation_path(&self, actor: &Agent, workspace: &str, path: &str) -> Result<String> {
        if self.remote_physical_root(actor)?.is_some() {
            normalize_relative_path(path)
        } else {
            crate::normalize_workspace_path(workspace, path)
        }
    }

    /// Historical provenance remains readable by the operator after membership is revoked.
    pub(crate) fn physical_root(&self, actor: &Agent) -> Result<String> {
        if !actor.project.starts_with("space:") {
            return Ok(actor.project.clone());
        }
        if let Some(root) = self.remote_physical_root(actor)? { return Ok(root); }
        diesel::sql_query("SELECT w.root AS value FROM agent_workspace_bindings AS b JOIN workspaces AS w ON w.id=b.workspace_id WHERE b.agent=? AND ?='space:' || b.space_id")
            .bind::<Text, _>(&actor.id).bind::<Text, _>(&actor.project)
            .get_result::<ValueRow>(&mut *self.connection.borrow_mut()).optional()?
            .map(|row| row.value).ok_or_else(|| scope_denied("Producing workspace unavailable"))
    }

    pub(crate) fn register_in_workspace(
        &self,
        terminal: &str,
        program: &str,
        binding: &WorkspaceBinding,
        name: &str,
    ) -> Result<Agent> {
        self.transaction(|| {
            self.authorize_workspace(binding)?;
            let domain = binding.domain();
            if let Some(existing) = self.agent_by_name(&domain, name)? {
                ensure!(self.count("SELECT COUNT(*) AS count FROM agent_workspace_bindings WHERE agent=? AND workspace_id=? AND space_id=?", &[&existing.id, &binding.id, &binding.space])? == 1,
                    scope_denied("Identity belongs to another workspace"));
            }
            let agent = self.register_inner(terminal, program, &domain, name)?;
            let prior = self.count("SELECT COUNT(*) AS count FROM agent_workspace_bindings WHERE agent=?", &[&agent.id])?;
            ensure!(prior == 0 || self.count("SELECT COUNT(*) AS count FROM agent_workspace_bindings WHERE agent=? AND workspace_id=? AND space_id=?", &[&agent.id, &binding.id, &binding.space])? == 1,
                scope_denied("Identity belongs to another workspace"));
            diesel::sql_query("INSERT INTO agent_workspace_bindings(agent, workspace_id, space_id, revoked) VALUES (?,?,?,0) ON CONFLICT(agent) DO UPDATE SET revoked=0")
                .bind::<Text, _>(&agent.id).bind::<Text, _>(&binding.id).bind::<Text, _>(&binding.space)
                .execute(&mut *self.connection.borrow_mut())?;
            self.space_join(&domain, &Self::operator(&domain), &binding.space, &agent.id)?;
            Ok(agent)
        })
    }

    pub fn register(
        &self,
        terminal: &str,
        program: &str,
        project: &str,
        name: &str,
    ) -> Result<Agent> {
        ensure!(
            !project.starts_with("space:"),
            scope_denied("Shared registration requires an app-selected workspace")
        );
        self.register_inner(terminal, program, project, name)
    }

    fn register_inner(&self, terminal: &str, program: &str, project: &str, name: &str) -> Result<Agent> {
        ensure!(
            !program.is_empty() && program.len() <= 64 && program != OPERATOR_PROGRAM,
            invalid_input("Invalid managed program")
        );
        ensure!(
            !name.is_empty()
                && name.len() <= 64
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')),
            invalid_input("Name must contain 1-64 ASCII letters, numbers, hyphens, or underscores")
        );
        ensure!(
            name != OPERATOR_NAME,
            invalid_input("Name is reserved for the local operator")
        );
        let id = match self.agent_by_name(project, name)? {
            Some(agent) => agent.id,
            None => match self.agent_by_terminal(terminal, program, project)? {
                Some(agent) => agent.id,
                None => Uuid::new_v4().to_string(),
            },
        };
        let exists = diesel::sql_query("SELECT id AS name FROM agents WHERE id = ?")
            .bind::<Text, _>(&id)
            .get_result::<NameRow>(&mut *self.connection.borrow_mut())
            .optional()?
            .is_some();
        ensure!(
            exists || self.agent_count()? < MAX_AGENTS,
            capacity_exceeded("Agent capacity reached")
        );
        let agent = Agent {
            id,
            terminal: terminal.to_owned(),
            name: name.to_owned(),
            program: program.to_owned(),
            project: project.to_owned(),
        };
        self.insert_agent(&agent)?;
        Ok(agent)
    }

    /// Losing a run proves disconnection, not process exit or stopped side effects.
    pub fn recover(&self, actor: &Agent, run: &str) -> Result<()> {
        self.transaction(|| self.recover_in_transaction(actor, run))
    }

    fn recover_in_transaction(&self, actor: &Agent, run: &str) -> Result<()> {
        self.authorize(actor)?;
        let tasks = diesel::sql_query(format!(
            "SELECT {TASK_COLUMNS} FROM tasks WHERE assignee = ? AND state IN ('running','cancel_requested')"
        ))
        .bind::<Text, _>(&actor.id)
        .load::<TaskRow>(&mut *self.connection.borrow_mut())?;
        for task in tasks.into_iter().map(TaskRow::task) {
            let active = self.active_attempts(&task.id)?;
            if active.iter().any(|attempt| attempt.run == run) {
                continue;
            }
            if self.pending_interrupt_notification(actor, &task)? {
                continue;
            }
            for attempt in &active {
                diesel::sql_query("UPDATE attempts SET certainty='interrupted', finished_at=NULL WHERE id=?")
                    .bind::<Text, _>(&attempt.id)
                    .execute(&mut *self.connection.borrow_mut())?;
                self.record(
                    &task.project,
                    "task_interrupted",
                    &actor.id,
                    Some(&task.id),
                    Some(&attempt.id),
                    json!({"revision": task.revision}),
                )?;
            }
            let mut task = task;
            task.version += 1;
            self.update_task_state(&task)?;
            if active.is_empty() {
                self.record(
                    &task.project,
                    "task_interrupted",
                    &actor.id,
                    Some(&task.id),
                    None,
                    json!({"revision": task.revision}),
                )?;
            }
            let message = task_message(
                &task.issuer,
                &actor.id,
                "interrupted",
                "Interrupted task. Previous execution is unknown; inspect it and request operator recovery before starting new work."
                    .into(),
                &task.id,
                task.revision,
            );
            self.queue(&task.project, message)?;
        }
        Ok(())
    }

    fn pending_interrupt_notification(&self, actor: &Agent, task: &Task) -> Result<bool> {
        Ok(self.pending(actor)?.iter().any(|message| {
            matches!(message.kind.as_str(), "assignment" | "interrupted")
                && message.task_id.as_deref() == Some(task.id.as_str())
                && message.revision == Some(task.revision)
        }))
    }

    fn active_attempts(&self, task_id: &str) -> Result<Vec<Attempt>> {
        Ok(diesel::sql_query("SELECT id, task_id, revision, owner, run, certainty, outcome, started_at, finished_at FROM attempts WHERE task_id = ? AND certainty = 'active' ORDER BY rowid")
            .bind::<Text, _>(task_id)
            .load::<AttemptRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(AttemptRow::attempt)
            .collect())
    }

    /// The original owner may confirm an outcome, but uncertainty grants no new execution rights.
    fn confirmable_attempts(&self, task: &Task, actor: &Agent, run: &str) -> Result<Vec<Attempt>> {
        Ok(diesel::sql_query("SELECT id,task_id,revision,owner,run,certainty,outcome,started_at,finished_at FROM attempts WHERE task_id=? AND owner=? AND run=? AND revision=? AND certainty IN ('active','unknown') AND outcome IS NULL AND finished_at IS NULL ORDER BY rowid")
            .bind::<Text,_>(&task.id).bind::<Text,_>(&actor.id).bind::<Text,_>(run)
            .bind::<Integer,_>(task.revision as i32)
            .load::<AttemptRow>(&mut *self.connection.borrow_mut())?
            .into_iter().map(AttemptRow::attempt).collect())
    }

    fn unresolved_attempts(&self, task_id: &str, revision: Option<u32>) -> Result<Vec<Attempt>> {
        Ok(diesel::sql_query("SELECT id, task_id, revision, owner, run, certainty, outcome, started_at, finished_at FROM attempts WHERE task_id = ? AND certainty != 'finished' AND (outcome IS NULL OR (certainty = 'unknown' AND revision = COALESCE(?, -1))) ORDER BY rowid")
            .bind::<Text, _>(task_id)
            .bind::<Nullable<Integer>, _>(revision.map(|revision| revision as i32))
            .load::<AttemptRow>(&mut *self.connection.borrow_mut())?
            .into_iter().map(AttemptRow::attempt).collect())
    }

    pub fn pending(&self, actor: &Agent) -> Result<Vec<Message>> {
        self.authorize(actor)?;
        Ok(diesel::sql_query("SELECT id, sender, recipient, body, subject, thread_id, reply_to, task_id, revision, kind, acknowledged, sequence FROM messages WHERE recipient = ? AND acknowledged = 0 ORDER BY sequence")
            .bind::<Text, _>(&actor.id)
            .load::<MessageRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(MessageRow::message)
            .collect())
    }

    pub fn next_work(&self, actor: &Agent, run: &str) -> Result<Option<Message>> {
        let executing = self.execution_in_run(&actor.id, run)?;
        self.first_pending(actor, executing)
    }

    pub(crate) fn first_pending(&self, actor: &Agent, skip_delegation: bool) -> Result<Option<Message>> {
        self.authorize(actor)?;
        Ok(diesel::sql_query("SELECT id, sender, recipient, body, subject, thread_id, reply_to, task_id, revision, kind, acknowledged, sequence FROM messages WHERE recipient = ? AND acknowledged = 0 AND (? = 0 OR kind NOT IN ('assignment','available')) ORDER BY sequence LIMIT 1")
            .bind::<Text, _>(&actor.id).bind::<Integer, _>(i32::from(skip_delegation))
            .get_result::<MessageRow>(&mut *self.connection.borrow_mut()).optional()?.map(MessageRow::message))
    }

    pub(crate) fn inbox_count(&self, agent: &str) -> Result<i64> {
        self.count("SELECT COUNT(*) AS count FROM messages WHERE recipient = ? AND acknowledged = 0", &[agent])
    }

    pub(crate) fn running_task(&self, agent: &str) -> Result<bool> {
        Ok(self.count("SELECT COUNT(*) AS count FROM tasks WHERE assignee = ? AND state IN ('running','cancel_requested')", &[agent])? > 0)
    }

    pub(crate) fn execution_in_run(&self, agent: &str, run: &str) -> Result<bool> {
        Ok(self.count(
            "SELECT COUNT(*) AS count FROM tasks WHERE assignee = ? AND state IN ('running','cancel_requested') AND id IN (SELECT task_id FROM attempts WHERE run = ? AND certainty = 'active')",
            &[agent, run],
        )? > 0)
    }

    pub fn has_work(&self, actor: &Agent) -> Result<bool> {
        if self.count(
            "SELECT COUNT(*) AS count FROM tasks WHERE issuer = ? OR assignee = ? OR reviewer = ?",
            &[actor.id.as_str(), actor.id.as_str(), actor.id.as_str()],
        )? > 0
        {
            return Ok(true);
        }
        Ok(self.count(
            "SELECT COUNT(*) AS count FROM messages WHERE acknowledged = 0 AND (sender = ? OR recipient = ?)",
            &[actor.id.as_str(), actor.id.as_str()],
        )? > 0)
    }

    pub fn task_states(&self, actor: &Agent, assignee: &str) -> Result<Vec<Value>> {
        Ok(diesel::sql_query("SELECT id, state, revision, version FROM tasks WHERE project = ? AND assignee = ? AND archived = 0 AND (issuer = ? OR reviewer = ? OR assignee = ?) ORDER BY created_seq")
            .bind::<Text, _>(&actor.project)
            .bind::<Text, _>(assignee)
            .bind::<Text, _>(&actor.id)
            .bind::<Text, _>(&actor.id)
            .bind::<Text, _>(&actor.id)
            .load::<SummaryRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(|row| {
                json!({"id": row.id, "state": row.state, "revision": row.revision as u32, "version": row.version as u64})
            })
            .collect())
    }

    fn task_row(&self, selector: &str, binds: &[&str; 6]) -> Result<Option<Task>> {
        Ok(diesel::sql_query(selector.to_owned())
            .bind::<Text, _>(binds[0])
            .bind::<Text, _>(binds[1])
            .bind::<Text, _>(binds[2])
            .bind::<Text, _>(binds[3])
            .bind::<Text, _>(binds[4])
            .bind::<Text, _>(binds[5])
            .get_result::<TaskRow>(&mut *self.connection.borrow_mut())
            .optional()?
            .map(TaskRow::task))
    }

    /// Shared queue visibility ends for nonparticipants after a successful claim.
    pub fn task(&self, actor: &Agent, id: &str) -> Result<Task> {
        self.authorize(actor)?;
        let task = self
            .task_row(
                &format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id = ? AND project = ? AND (issuer = ? OR assignee = ? OR reviewer = ? OR (assignee = '' AND EXISTS (SELECT 1 FROM task_eligibles WHERE task_eligibles.task_id = tasks.id AND task_eligibles.agent = ?)))"),
                &[id, actor.project.as_str(), actor.id.as_str(), actor.id.as_str(), actor.id.as_str(), actor.id.as_str()],
            )?
            .ok_or_else(|| scope_denied("Task not found"))?;
        self.assemble(task)
    }

    /// The trusted local UI reads and writes any task in its own project without an agent scope.
    pub(crate) fn operator_task(&self, project: &str, id: &str) -> Result<Task> {
        let task = diesel::sql_query(format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id = ? AND project = ?"))
            .bind::<Text, _>(id)
            .bind::<Text, _>(project)
            .get_result::<TaskRow>(&mut *self.connection.borrow_mut())
            .optional()?
            .map(TaskRow::task)
            .ok_or_else(|| scope_denied("Task not found"))?;
        self.assemble(task)
    }

    /// Mutation visibility: agents stay scoped; the operator principal reaches every project task.
    fn actor_task(&self, actor: &Agent, id: &str) -> Result<Task> {
        if actor.program == OPERATOR_PROGRAM {
            self.operator_task(&actor.project, id)
        } else {
            self.task(actor, id)
        }
    }

    pub(crate) fn operator(project: &str) -> Agent {
        Agent {
            id: format!("{OPERATOR_PROGRAM}:{project}"),
            terminal: String::new(),
            name: OPERATOR_NAME.into(),
            program: OPERATOR_PROGRAM.into(),
            project: project.into(),
        }
    }

    fn assemble(&self, mut task: Task) -> Result<Task> {
        task.attempts = diesel::sql_query("SELECT id, task_id, revision, owner, run, certainty, outcome, started_at, finished_at FROM attempts WHERE task_id = ? ORDER BY rowid DESC LIMIT 16")
            .bind::<Text, _>(&task.id)
            .load::<AttemptRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(AttemptRow::attempt)
            .collect();
        task.attempts.reverse();
        task.feedback = diesel::sql_query("SELECT body FROM feedback WHERE task_id = ? ORDER BY id DESC LIMIT 8")
            .bind::<Text, _>(&task.id)
            .load::<FeedbackRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(|row| row.body)
            .collect();
        task.feedback.reverse();
        task.history_truncated = self.count("SELECT COUNT(*) AS count FROM attempts WHERE task_id = ?", &[task.id.as_str()])? > 16
            || self.count("SELECT COUNT(*) AS count FROM feedback WHERE task_id = ?", &[task.id.as_str()])? > 8;
        task.dependencies = self.task_dependencies(&task.id)?;
        task.eligible = self.task_eligibles(&task.id)?;
        task.evidence_records = self.task_evidence(&task.id)?;
        task.executing_run = task
            .attempts
            .iter()
            .find(|attempt| attempt.certainty == "active" && attempt.revision == task.revision)
            .map(|attempt| attempt.run.clone());
        let blocking = self.blocking_dependencies(&task.id)?;
        task.wait_reason = match task.state.as_str() {
            "running" if task.executing_run.is_none() => Some("Execution is interrupted; the previous process is not confirmed stopped. Request operator recovery.".into()),
            "cancel_requested" => Some("Cancellation requested; the owning execution must confirm it stopped, or an operator must explicitly override unknown execution.".into()),
            "blocked" if !blocking.is_empty() => Some(format!(
                "Waiting for prerequisites: {}",
                blocking.join(", ")
            )),
            "blocked" => Some("Waiting for prerequisites".into()),
            "queued" if task.assignee.is_empty() => Some("Unclaimed pool task".into()),
            _ => None,
        };
        task.review_overdue = task.state == "submitted"
            && task
                .review_deadline
                .is_some_and(|deadline| deadline <= now());
        Ok(task)
    }

    fn blocking_dependencies(&self, task_id: &str) -> Result<Vec<String>> {
        Ok(diesel::sql_query("SELECT dependency.prerequisite_id AS value FROM task_dependencies AS dependency JOIN tasks AS prerequisite ON prerequisite.id = dependency.prerequisite_id WHERE dependency.task_id = ? AND prerequisite.state != 'accepted' ORDER BY dependency.position")
            .bind::<Text, _>(task_id)
            .load::<ValueRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(|row| row.value)
            .collect())
    }

    fn task_evidence(&self, task_id: &str) -> Result<Vec<Evidence>> {
        Ok(diesel::sql_query("SELECT id, task_id, attempt_id, kind, path, hash, commit_id, repository, branch, base, head, command, outcome, exit_code, summary, device, verified, created_seq FROM evidence WHERE task_id = ? ORDER BY created_seq")
            .bind::<Text, _>(task_id)
            .load::<EvidenceRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(EvidenceRow::evidence)
            .collect())
    }

    fn visible_message(&self, actor: &Agent, id: &str) -> Result<Message> {
        diesel::sql_query("SELECT id, sender, recipient, body, subject, thread_id, reply_to, task_id, revision, kind, acknowledged, sequence FROM messages WHERE id = ? AND project = ? AND (sender = ? OR recipient = ?)")
            .bind::<Text, _>(id)
            .bind::<Text, _>(&actor.project)
            .bind::<Text, _>(&actor.id)
            .bind::<Text, _>(&actor.id)
            .get_result::<MessageRow>(&mut *self.connection.borrow_mut())
            .optional()?
            .map(MessageRow::message)
            .ok_or_else(|| scope_denied("Message not found"))
    }

    fn resolve(&self, actor: &Agent, name: &str) -> Result<Agent> {
        let recipient = self.agent(&actor.project, name)?;
        self.authorize(&recipient)?;
        Ok(recipient)
    }

    /// Persisted identity lookup by ID or display name; the transport uses it to admit offline recipients.
    pub(crate) fn agent(&self, project: &str, id_or_name: &str) -> Result<Agent> {
        Ok(diesel::sql_query("SELECT id, terminal, name, program, project FROM agents WHERE project = ? AND (id = ? OR name = ?)")
            .bind::<Text, _>(project)
            .bind::<Text, _>(id_or_name)
            .bind::<Text, _>(id_or_name)
            .get_result::<AgentRow>(&mut *self.connection.borrow_mut())
            .optional()?
            .map(AgentRow::agent)
            .ok_or_else(|| scope_denied("Agent not found in this project"))?)
    }

    fn visible_thread(&self, actor: &Agent, thread_id: &str) -> Result<()> {
        let any = self.count(
            "SELECT COUNT(*) AS count FROM messages WHERE project = ? AND thread_id = ?",
            &[actor.project.as_str(), thread_id],
        )?;
        if any > 0 {
            let participant = self.count(
                "SELECT COUNT(*) AS count FROM messages WHERE project = ? AND thread_id = ? AND (sender = ? OR recipient = ?)",
                &[actor.project.as_str(), thread_id, actor.id.as_str(), actor.id.as_str()],
            )?;
            ensure!(participant > 0, scope_denied("Thread not found"));
        }
        Ok(())
    }

    fn page_limit(limit: Option<u32>) -> Result<u32> {
        match limit {
            None => Ok(PAGE_DEFAULT),
            Some(limit) if (1..=PAGE_MAX).contains(&limit) => Ok(limit),
            Some(_) => Err(invalid_input("limit must be between 1 and 200")),
        }
    }

    pub fn inbox(&self, actor: &Agent, cursor: Option<u64>, limit: Option<u32>) -> Result<Value> {
        self.authorize(actor)?;
        let limit = Self::page_limit(limit)?;
        let rows = diesel::sql_query("SELECT id, sender, recipient, body, subject, thread_id, reply_to, task_id, revision, kind, acknowledged, sequence FROM messages WHERE recipient = ? AND acknowledged = 0 AND sequence > ? ORDER BY sequence LIMIT ?")
            .bind::<Text, _>(&actor.id)
            .bind::<BigInt, _>(cursor.unwrap_or(0) as i64)
            .bind::<BigInt, _>(limit as i64 + 1)
            .load::<MessageRow>(&mut *self.connection.borrow_mut())?;
        let (messages, cursor) = bounded_items(rows.into_iter().map(MessageRow::message)
            .map(|message| (message.sequence, message)), limit)?;
        Ok(json!({"messages": messages, "cursor": cursor}))
    }

    pub fn task_list(
        &self,
        actor: &Agent,
        state: Option<&str>,
        assignee: Option<&str>,
        cursor: Option<u64>,
        limit: Option<u32>,
        include_archived: bool,
    ) -> Result<Value> {
        let limit = Self::page_limit(limit)?;
        let assignee = match assignee {
            Some(name) => match self.resolve(actor, name) {
                Ok(agent) => Some(agent.id),
                Err(_) => return Ok(json!({"tasks": [], "cursor": Value::Null})),
            },
            None => None,
        };
        let rows = diesel::sql_query("SELECT id, state, revision, version, assignee, reviewer, created_seq FROM tasks WHERE project = ? AND (issuer = ? OR assignee = ? OR reviewer = ? OR (assignee = '' AND EXISTS (SELECT 1 FROM task_eligibles WHERE task_eligibles.task_id = tasks.id AND task_eligibles.agent = ?))) AND state = COALESCE(?, state) AND assignee = COALESCE(?, assignee) AND (archived = 0 OR ?) AND created_seq > ? ORDER BY created_seq LIMIT ?")
            .bind::<Text, _>(&actor.project)
            .bind::<Text, _>(&actor.id)
            .bind::<Text, _>(&actor.id)
            .bind::<Text, _>(&actor.id)
            .bind::<Text, _>(&actor.id)
            .bind::<Nullable<Text>, _>(state)
            .bind::<Nullable<Text>, _>(assignee)
            .bind::<Integer, _>(i32::from(include_archived))
            .bind::<BigInt, _>(cursor.unwrap_or(0) as i64)
            .bind::<BigInt, _>(limit as i64 + 1)
            .load::<TaskSummaryRow>(&mut *self.connection.borrow_mut())?;
        Ok(task_summaries(rows, limit))
    }

    /// Panel projection: every task in the project, independent of agent scope.
    pub(crate) fn operator_tasks(
        &self,
        project: &str,
        state: Option<&str>,
        assignee: Option<&str>,
        cursor: Option<u64>,
        limit: Option<u32>,
        include_archived: bool,
    ) -> Result<Value> {
        let limit = Self::page_limit(limit)?;
        let rows = diesel::sql_query("SELECT id, state, revision, version, assignee, reviewer, created_seq FROM tasks WHERE project = ? AND state = COALESCE(?, state) AND (assignee = COALESCE(?, assignee) OR ? = '') AND (archived = 0 OR ?) AND created_seq > ? ORDER BY created_seq LIMIT ?")
            .bind::<Text, _>(project)
            .bind::<Nullable<Text>, _>(state)
            .bind::<Nullable<Text>, _>(assignee)
            .bind::<Text, _>(assignee.unwrap_or(""))
            .bind::<Integer, _>(i32::from(include_archived))
            .bind::<BigInt, _>(cursor.unwrap_or(0) as i64)
            .bind::<BigInt, _>(limit as i64 + 1)
            .load::<TaskSummaryRow>(&mut *self.connection.borrow_mut())?;
        Ok(task_summaries(rows, limit))
    }

    pub fn events(&self, project: &str, after: Option<u64>, limit: Option<u32>) -> Result<Value> {
        let limit = Self::page_limit(limit)?;
        let rows = diesel::sql_query("SELECT id, project, sequence, kind, actor, resource, attempt, observed_at, imported, payload FROM events WHERE project = ? AND sequence > ? ORDER BY sequence LIMIT ?")
            .bind::<Text, _>(project)
            .bind::<BigInt, _>(after.unwrap_or(0) as i64)
            .bind::<BigInt, _>(limit as i64 + 1)
            .load::<EventRow>(&mut *self.connection.borrow_mut())?;
        let (events, cursor) = bounded_items(rows.into_iter().map(EventRow::event)
            .map(|event| (event.sequence, event)), limit)?;
        Ok(json!({"events": events, "cursor": cursor}))
    }

    fn task_history(&self, actor: &Agent, task_id: &str, cursor: Option<u64>, limit: Option<u32>) -> Result<Value> {
        self.actor_task(actor, task_id)?;
        let limit = Self::page_limit(limit)?;
        let rows = diesel::sql_query("SELECT id, project, sequence, kind, actor, resource, attempt, observed_at, imported, payload FROM events WHERE project = ? AND resource = ? AND sequence > ? ORDER BY sequence LIMIT ?")
            .bind::<Text, _>(&actor.project).bind::<Text, _>(task_id)
            .bind::<BigInt, _>(cursor.unwrap_or(0) as i64).bind::<BigInt, _>(limit as i64 + 1)
            .load::<EventRow>(&mut *self.connection.borrow_mut())?;
        let (events, cursor) = bounded_items(rows.into_iter().map(EventRow::event)
            .map(|event| (event.sequence, event)), limit)?;
        Ok(json!({"events": events, "cursor": cursor}))
    }

    fn request_row(&self, actor: &str, request_id: &str) -> Result<Option<RequestRow>> {
        Ok(diesel::sql_query("SELECT epoch, fingerprint, response FROM requests WHERE actor = ? AND request_id = ?")
            .bind::<Text, _>(actor)
            .bind::<Text, _>(request_id)
            .get_result::<RequestRow>(&mut *self.connection.borrow_mut())
            .optional()?)
    }

    /// The broker holds its mutation mutex while checking this and executing the request.
    pub(crate) fn request_seen(&self, actor: &str, request_id: &str) -> Result<bool> {
        Ok(self.count("SELECT COUNT(*) AS count FROM requests WHERE actor = ? AND request_id = ?",
            &[actor, request_id])? != 0)
    }

    fn store_request(
        &self,
        actor: &str,
        request_id: &str,
        epoch: &str,
        fingerprint: &str,
        response: &Value,
    ) -> Result<()> {
        diesel::sql_query("INSERT INTO requests(actor, request_id, epoch, fingerprint, response, created_at) VALUES (?,?,?,?,?,?)")
            .bind::<Text, _>(actor)
            .bind::<Text, _>(request_id)
            .bind::<Text, _>(epoch)
            .bind::<Text, _>(fingerprint)
            .bind::<Text, _>(response.to_string())
            .bind::<BigInt, _>(now() as i64)
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    pub fn execute(&self, actor: &Agent, run: &str, operation: &Operation) -> Result<Value> {
        self.execute_inner(actor, run, operation)
            .map_err(crate::classify_storage)
    }

    fn execute_inner(&self, actor: &Agent, run: &str, operation: &Operation) -> Result<Value> {
        self.authorize(actor)?;
        self.sweep(&actor.project)?;
        match operation {
            Operation::AgentList => return Ok(json!(self.agents(&actor.project)?)),
            Operation::AgentInbox { cursor, limit } => return self.inbox(actor, *cursor, *limit),
            Operation::TaskList {
                state,
                assignee,
                cursor,
                limit,
                include_archived,
            } => {
                if actor.program == OPERATOR_PROGRAM {
                    return self.operator_tasks(
                        &actor.project,
                        state.as_deref(),
                        assignee.as_deref(),
                        *cursor,
                        *limit,
                        *include_archived,
                    );
                }
                return self.task_list(
                    actor,
                    state.as_deref(),
                    assignee.as_deref(),
                    *cursor,
                    *limit,
                    *include_archived,
                );
            }
            Operation::TaskGet { task_id } => return Ok(json!(self.actor_task(actor, task_id)?)),
            Operation::TaskHistory { task_id, cursor, limit } => return self.task_history(actor, task_id, *cursor, *limit),
            Operation::ThreadGet {
                thread_id,
                cursor,
                limit,
            } => return self.thread_get(actor, thread_id, *cursor, *limit),
            Operation::MessageSearch {
                query,
                task_id,
                thread_id,
                cursor,
                limit,
            } => {
                return self.message_search(
                    actor,
                    query,
                    task_id.as_deref(),
                    thread_id.as_deref(),
                    *cursor,
                    *limit,
                )
            }
            Operation::FileReservations {
                path,
                cursor,
                limit,
                include_expired,
            } => {
                return self.file_reservations(
                    actor,
                    path.as_deref(),
                    *cursor,
                    *limit,
                    *include_expired,
                )
            }
            Operation::AgentRegister { .. } | Operation::AgentWait | Operation::AgentReady => {
                return Err(invalid_state("Operation requires live session handling"))
            }
            _ => {}
        }
        let serialized = serde_json::to_string(operation)?;
        let Some(request_id) = operation.request_id() else {
            // AgentAck is idempotent by nature and carries no request ID.
            return self.transaction(|| self.mutate(actor, run, operation));
        };
        if Uuid::parse_str(request_id).is_err() {
            return Err(invalid_input("request_id must be a UUID"));
        }
        let control = !matches!(operation, Operation::AgentSend { .. } | Operation::TaskAssign { .. } | Operation::TaskCreatePool { .. } | Operation::FileReserve { .. } | Operation::EvidenceAdd { .. });
        self.execute_mutation(&actor.id, run, request_id, &serialized, control, || {
            self.mutate(actor, run, operation)
        })
    }

    /// The trusted local UI writes through the same transitions under a deterministic operator principal.
    pub(crate) fn execute_controller(
        &self,
        project: &str,
        operation: &ControllerOperation,
    ) -> Result<Value> {
        self.execute_controller_inner(project, operation)
            .map_err(crate::classify_storage)
    }

    fn execute_controller_inner(&self, project: &str, operation: &ControllerOperation) -> Result<Value> {
        self.sweep(project)?;
        let actor = Self::operator(project);
        match operation {
            ControllerOperation::SpaceList { cursor, limit } => return self.space_list(project, cursor.as_deref(), *limit),
            ControllerOperation::DeviceList | ControllerOperation::InvitationCreate { .. }
            | ControllerOperation::DeviceGrantUpdate { .. } | ControllerOperation::DeviceRevoke { .. }
            | ControllerOperation::RemoteWorkspaceMap { .. } => return Err(crate::domain(
                "feature_unavailable", "Device collaboration was retired", false, None)),
            ControllerOperation::PurgePreview => return self.purge_preview(project),
            ControllerOperation::HistoryExport { after, limit } => {
                return self.history_export(project, *after, *limit)
            }
            _ => {}
        }
        let serialized = serde_json::to_string(operation)?;
        let Some(request_id) = operation.request_id() else {
            return Err(invalid_input("Controller mutation requires a request_id"));
        };
        if Uuid::parse_str(request_id).is_err() {
            return Err(invalid_input("request_id must be a UUID"));
        }
        self.execute_mutation(&actor.id, OPERATOR_EPOCH, request_id, &serialized, true, || {
            self.mutate_controller(project, &actor, operation)
        })
    }

    /// Expired epochs are rejected before a replay can masquerade as the original commit.
    fn execute_mutation(
        &self,
        actor_id: &str,
        run: &str,
        request_id: &str,
        serialized: &str,
        control: bool,
        mutate: impl FnOnce() -> Result<Value>,
    ) -> Result<Value> {
        let mut result = None;
        self.transaction(|| {
            if let Some(row) = self.request_row(actor_id, request_id)? {
                ensure!(
                    row.epoch == run,
                    epoch_expired("Request belongs to an expired mutation epoch; use a new request_id")
                );
                ensure!(
                    row.fingerprint == serialized,
                    request_conflict("request_id was reused for a different operation")
                );
                result = Some(serde_json::from_str(&row.response)?);
                return Ok(());
            }
            ensure!(
                self.request_count()? < MAX_REQUESTS + if control { 1000 } else { 0 },
                capacity_exceeded("Request capacity reached")
            );
            let value = mutate()?;
            self.store_request(actor_id, request_id, run, serialized, &value)?;
            result = Some(value);
            Ok(())
        })?;
        Ok(result.expect("committed mutation returns its stored response"))
    }

    /// Deadline expiry is lazy: every mutation checks unstarted past-due work and overdue execution.
    pub(crate) fn sweep(&self, project: &str) -> Result<()> {
        self.sweep_at(project, now())
    }

    fn sweep_at(&self, project: &str, now: u64) -> Result<()> {
        // Expired leases whose execution already resolved are dropped; unknown owners keep their warning.
        diesel::sql_query("DELETE FROM reservations WHERE owner IN (SELECT id FROM agents WHERE project = ?) AND expires_at <= ? AND (attempt_id IS NULL OR NOT EXISTS (SELECT 1 FROM attempts WHERE attempts.id = reservations.attempt_id AND attempts.certainty = 'active'))")
            .bind::<Text, _>(project)
            .bind::<BigInt, _>(now as i64)
            .execute(&mut *self.connection.borrow_mut())?;
        let expired = diesel::sql_query(format!(
            "SELECT {TASK_COLUMNS} FROM tasks WHERE project = ? AND state IN ('queued','blocked') AND start_deadline IS NOT NULL AND start_deadline <= ? AND NOT EXISTS (SELECT 1 FROM attempts WHERE attempts.task_id = tasks.id AND attempts.revision = tasks.revision)"
        ))
        .bind::<Text, _>(project)
        .bind::<BigInt, _>(now as i64)
        .load::<TaskRow>(&mut *self.connection.borrow_mut())?;
        let overdue = diesel::sql_query(format!(
            "SELECT {TASK_COLUMNS} FROM tasks WHERE project = ? AND state = 'running' AND execution_deadline IS NOT NULL AND execution_deadline <= ?"
        ))
        .bind::<Text, _>(project)
        .bind::<BigInt, _>(now as i64)
        .load::<TaskRow>(&mut *self.connection.borrow_mut())?;
        if expired.is_empty() && overdue.is_empty() {
            return Ok(());
        }
        self.transaction(|| {
            for task in expired.into_iter().map(TaskRow::task) {
                let mut task = task;
                let deadline = task.start_deadline;
                task.state = "expired".into();
                task.start_deadline = None;
                task.version += 1;
                self.update_task_state(&task)?;
                self.retire_available(&task.id)?;
                self.record(
                    &task.project,
                    "task_expired",
                    OPERATOR_EPOCH,
                    Some(&task.id),
                    None,
                    json!({"revision": task.revision, "start_deadline": deadline}),
                )?;
                if !task.assignee.is_empty() {
                    self.queue(
                        &task.project,
                        task_message(
                            &task.issuer,
                            &task.assignee,
                            "expired",
                            "Task start deadline passed before it started. The issuer can retry it with a new deadline."
                                .into(),
                            &task.id,
                            task.revision,
                        ),
                    )?;
                }
            }
            for task in overdue.into_iter().map(TaskRow::task) {
                let mut task = task;
                task.state = "cancel_requested".into();
                task.version += 1;
                self.update_task_state(&task)?;
                self.record(
                    &task.project,
                    "task_cancel_requested",
                    OPERATOR_EPOCH,
                    Some(&task.id),
                    None,
                    json!({"revision": task.revision, "reason": "execution deadline passed"}),
                )?;
                if !task.assignee.is_empty() {
                    self.queue(
                        &task.project,
                        task_message(
                            &task.issuer,
                            &task.assignee,
                            "cancel_requested",
                            "Execution deadline passed. Stop the current work, report what happened, then confirm the cancellation with warp_task_finish_cancel."
                                .into(),
                            &task.id,
                            task.revision,
                        ),
                    )?;
                }
            }
            Ok(())
        })
    }

    pub(crate) fn agents(&self, project: &str) -> Result<Vec<Agent>> {
        // Release the SQLite borrow before membership checks issue their own queries.
        let rows = diesel::sql_query("SELECT id, terminal, name, program, project FROM agents WHERE project = ? ORDER BY name")
            .bind::<Text, _>(project)
            .load::<AgentRow>(&mut *self.connection.borrow_mut())?;
        Ok(rows.into_iter().map(AgentRow::agent)
            .filter(|agent| self.authorize(agent).is_ok()).collect())
    }

    fn mutate(&self, actor: &Agent, run: &str, operation: &Operation) -> Result<Value> {
        match operation {
            Operation::AgentSend {
                to,
                body,
                subject,
                thread_id,
                reply_to,
                task_id,
                ..
            } => {
                self.budget_available()?;
                text(body)?;
                if let Some(subject) = subject {
                    validate_subject(subject)?;
                }
                let recipient = self.resolve(actor, to)?;
                if let Some(task_id) = task_id {
                    self.actor_task(actor, task_id)?;
                }
                let (thread, reply) = match reply_to {
                    Some(reply_id) => {
                        let parent = self.visible_message(actor, reply_id)?;
                        let inherited = parent.thread_id.clone().or_else(|| Some(parent.id.clone()));
                        if let Some(explicit) = thread_id {
                            ensure!(
                                Some(explicit.clone()) == inherited,
                                invalid_input("reply_to does not belong to thread_id")
                            );
                        }
                        (inherited, Some(parent.id))
                    }
                    None => {
                        if let Some(explicit) = thread_id {
                            self.visible_thread(actor, explicit)?;
                        }
                        (thread_id.clone(), None)
                    }
                };
                let message = Message {
                    id: String::new(),
                    from: actor.id.clone(),
                    to: recipient.id,
                    body: body.clone(),
                    subject: subject.clone(),
                    thread_id: thread,
                    reply_to: reply,
                    task_id: task_id.clone(),
                    revision: None,
                    kind: "message".into(),
                    acknowledged: false,
                    sequence: 0,
                };
                Ok(json!(self.queue(&actor.project, message)?))
            }
            Operation::AgentAck { message_id } => {
                let message = self.visible_message(actor, message_id)?;
                ensure!(
                    !matches!(
                        message.kind.as_str(),
                        "assignment" | "review" | "available" | "cancel_requested" | "interrupted"
                    ),
                    invalid_state(
                        "Task notifications require their task transition, not message acknowledgement"
                    )
                );
                diesel::sql_query("UPDATE messages SET acknowledged = 1 WHERE id = ?")
                    .bind::<Text, _>(message_id)
                    .execute(&mut *self.connection.borrow_mut())?;
                self.record(
                    &actor.project,
                    "message_acknowledged",
                    &actor.id,
                    Some(message_id),
                    None,
                    json!({}),
                )?;
                Ok(json!({"acknowledged": message_id}))
            }
            Operation::TaskAssign {
                to,
                description,
                acceptance,
                reviewer,
                dependencies,
                start_deadline,
                execution_timeout_seconds,
                review_timeout_seconds,
                ..
            } => {
                text(description)?;
                text(acceptance)?;
                self.budget_available()?;
                ensure!(
                    self.active_task_count(&actor.project)? < MAX_ACTIVE_TASKS_PER_PROJECT,
                    capacity_exceeded("Active task queue is full; archive eligible completed work")
                );
                let assignee = self.resolve(actor, to)?;
                let reviewer = match reviewer {
                    Some(name) => self.resolve(actor, name)?,
                    None => actor.clone(),
                };
                ensure!(
                    assignee.id != reviewer.id,
                    invalid_state("Assignee cannot review its own task")
                );
                let dependencies =
                    self.validated_dependencies(&actor.project, dependencies, None)?;
                let start_deadline = pending_deadline(start_deadline.as_deref())?;
                let execution_timeout =
                    execution_timeout_seconds.map(timeout_seconds).transpose()?;
                let review_timeout = review_timeout_seconds.map(timeout_seconds).transpose()?;
                let task = Task {
                    id: Uuid::new_v4().to_string(),
                    project: actor.project.clone(),
                    issuer: actor.id.clone(),
                    assignee: assignee.id,
                    reviewer: reviewer.id,
                    description: description.clone(),
                    acceptance: acceptance.clone(),
                    state: "queued".into(),
                    revision: 1,
                    version: 1,
                    result: None,
                    evidence: None,
                    feedback: vec![],
                    history_truncated: false,
                    attempts: vec![],
                    evidence_records: vec![],
                    created_seq: self.next_sequence(&actor.project)?,
                    archived: false,
                    dependencies: dependencies.clone(),
                    eligible: vec![],
                    wait_reason: None,
                    start_deadline,
                    execution_timeout_seconds: execution_timeout,
                    review_timeout_seconds: review_timeout,
                    execution_deadline: None,
                    review_deadline: None,
                    review_overdue: false,
                    executing_run: None,
                };
                self.insert_task(&task)?;
                self.set_task_dependencies(&task.id, &dependencies)?;
                let mut task = task;
                if !self.blocking_dependencies(&task.id)?.is_empty() {
                    task.state = "blocked".into();
                    self.update_task_state(&task)?;
                }
                self.record(
                    &task.project,
                    "task_assigned",
                    &actor.id,
                    Some(&task.id),
                    None,
                    json!({"assignee": task.assignee, "reviewer": task.reviewer, "revision": task.revision, "state": task.state, "dependencies": task.dependencies}),
                )?;
                if task.state == "queued" {
                    self.queue(
                        &task.project,
                        task_message(
                            &actor.id,
                            &task.assignee,
                            "assignment",
                            "Task assigned. Read the task and explicitly start its revision.".into(),
                            &task.id,
                            task.revision,
                        ),
                    )?;
                }
                Ok(json!(self.assemble(task)?))
            }
            Operation::TaskStart {
                task_id,
                revision,
                expected_version,
                ..
            } => {
                let other_running = self.count(
                    "SELECT COUNT(*) AS count FROM tasks WHERE assignee = ? AND state IN ('running','cancel_requested') AND id != ?",
                    &[actor.id.as_str(), task_id],
                )? > 0;
                ensure!(
                    !other_running,
                    invalid_state("An assigned task is already running")
                );
                let task = self.task(actor, task_id)?;
                ensure!(
                    task.assignee == actor.id,
                    unauthorized("Task is not assigned to this agent")
                );
                ensure!(
                    task.revision == *revision,
                    stale_revision("Task revision is stale", task.version)
                );
                if let Some(expected) = expected_version {
                    ensure!(
                        *expected == task.version,
                        version_conflict("Task version does not match", task.version)
                    );
                }
                ensure!(
                    self.blocking_dependencies(&task.id)?.is_empty(),
                    dependency_blocked("Task prerequisites are not complete")
                );
                let mut task = task;
                let active = self.active_attempts(&task.id)?;
                match task.state.as_str() {
                    "queued" => {}
                    "running" if active.iter().any(|attempt| attempt.run == run) => {
                        return Err(invalid_state("Task is already running in this session"))
                    }
                    "running" => {
                        return Err(execution_unknown("Previous execution may still be running; require observed stop or an explicit operator override before retry"));
                    }
                    _ => return Err(invalid_state("Task is not queued or interrupted")),
                }
                let attempt = Attempt {
                    id: Uuid::new_v4().to_string(),
                    task_id: task.id.clone(),
                    revision: task.revision,
                    owner: actor.id.clone(),
                    run: run.to_owned(),
                    certainty: "active".into(),
                    outcome: None,
                    started_at: Some(now()),
                    finished_at: None,
                };
                diesel::sql_query("INSERT INTO attempts(id, task_id, revision, owner, run, certainty, outcome, started_at, finished_at) VALUES (?,?,?,?,?,?,NULL,?,NULL)")
                    .bind::<Text, _>(&attempt.id)
                    .bind::<Text, _>(&attempt.task_id)
                    .bind::<Integer, _>(attempt.revision as i32)
                    .bind::<Text, _>(&attempt.owner)
                    .bind::<Text, _>(&attempt.run)
                    .bind::<Text, _>(&attempt.certainty)
                    .bind::<BigInt, _>(attempt.started_at.unwrap() as i64)
                    .execute(&mut *self.connection.borrow_mut())?;
                let execution_deadline = task
                    .execution_timeout_seconds
                    .map(|seconds| now() + seconds * 1000);
                task.state = "running".into();
                task.start_deadline = None;
                task.execution_deadline = execution_deadline;
                task.version += 1;
                self.update_task_state(&task)?;
                self.ack_task_notification(&task.id, &actor.id)?;
                self.retire_available(&task.id)?;
                self.record(
                    &task.project,
                    "task_started",
                    &actor.id,
                    Some(&task.id),
                    Some(&attempt.id),
                    json!({"revision": task.revision, "owner": actor.id, "run": run}),
                )?;
                let task = self.assemble(task)?;
                let mut value = serde_json::to_value(&task)?;
                value["attempt_id"] = json!(attempt.id);
                Ok(value)
            }
            Operation::TaskSubmit {
                task_id,
                revision,
                result,
                evidence,
                expected_version,
                attempt_id,
                evidence_ids,
                ..
            } => {
                text(result)?;
                text(evidence)?;
                let task = self.task(actor, task_id)?;
                ensure!(
                    task.assignee == actor.id,
                    unauthorized("Task is not assigned to this agent")
                );
                ensure!(
                    task.revision == *revision,
                    stale_revision("Task revision is stale", task.version)
                );
                if let Some(expected) = expected_version {
                    ensure!(
                        *expected == task.version,
                        version_conflict("Task version does not match", task.version)
                    );
                }
                ensure!(task.state == "running", invalid_state("Task is not running"));
                let attempt = self
                    .confirmable_attempts(&task, actor, run)?
                    .into_iter()
                    .find(|attempt| {
                        attempt.owner == actor.id
                            && attempt.run == run
                            && attempt.revision == task.revision
                    })
                    .ok_or_else(|| {
                        stale_attempt("No active attempt owned by this session", task.version)
                    })?;
                if let Some(expected) = attempt_id {
                    ensure!(
                        *expected == attempt.id,
                        stale_attempt("Execution attempt was replaced", task.version)
                    );
                }
                self.validate_evidence_ids(&task, evidence_ids)?;
                diesel::sql_query("UPDATE attempts SET certainty='finished', outcome='submitted', finished_at=? WHERE id=?")
                    .bind::<BigInt, _>(now() as i64)
                    .bind::<Text, _>(&attempt.id)
                    .execute(&mut *self.connection.borrow_mut())?;
                let review_deadline = task
                    .review_timeout_seconds
                    .map(|seconds| now() + seconds * 1000);
                let mut task = task;
                task.state = "submitted".into();
                task.result = Some(result.clone());
                task.evidence = Some(evidence.clone());
                task.execution_deadline = None;
                task.review_deadline = review_deadline;
                task.version += 1;
                self.update_task_state(&task)?;
                self.record(
                    &task.project,
                    "task_submitted",
                    &actor.id,
                    Some(&task.id),
                    Some(&attempt.id),
                    json!({"revision": task.revision, "result": result, "evidence": evidence, "evidence_ids": evidence_ids}),
                )?;
                self.queue(
                    &task.project,
                    task_message(
                        &actor.id,
                        &task.reviewer,
                        "review",
                        "Result submitted. Inspect the task's result and evidence, then accept or request changes.".into(),
                        &task.id,
                        task.revision,
                    ),
                )?;
                let task = self.assemble(task)?;
                let mut value = serde_json::to_value(&task)?;
                value["attempt_id"] = json!(attempt.id);
                Ok(value)
            }
            Operation::TaskReview {
                task_id,
                revision,
                accepted,
                feedback,
                expected_version,
                ..
            } => {
                text(feedback)?;
                let mut task = self.actor_task(actor, task_id)?;
                ensure!(
                    task.reviewer == actor.id || actor.program == OPERATOR_PROGRAM,
                    unauthorized("Task reviewer does not match")
                );
                ensure!(
                    task.revision == *revision,
                    stale_revision("Task revision is stale", task.version)
                );
                if let Some(expected) = expected_version {
                    ensure!(
                        *expected == task.version,
                        version_conflict("Task version does not match", task.version)
                    );
                }
                ensure!(
                    task.state == "submitted",
                    invalid_state("Task is not submitted")
                );
                self.ack_task_notification(&task.id, &actor.id)?;
                diesel::sql_query("INSERT INTO feedback(task_id, revision, author, body, accepted, created_at) VALUES (?,?,?,?,?,?)")
                    .bind::<Text, _>(&task.id)
                    .bind::<Integer, _>(task.revision as i32)
                    .bind::<Text, _>(&actor.id)
                    .bind::<Text, _>(feedback)
                    .bind::<Integer, _>(i32::from(*accepted))
                    .bind::<BigInt, _>(now() as i64)
                    .execute(&mut *self.connection.borrow_mut())?;
                task.state = if *accepted { "accepted" } else { "queued" }.into();
                task.review_deadline = None;
                if !accepted {
                    task.revision = task
                        .revision
                        .checked_add(1)
                        .ok_or_else(|| invalid_state("Revision overflow"))?;
                    task.result = None;
                    task.evidence = None;
                }
                task.version += 1;
                self.update_task_state(&task)?;
                self.record(
                    &task.project,
                    "task_reviewed",
                    &actor.id,
                    Some(&task.id),
                    None,
                    json!({"accepted": accepted, "revision": task.revision, "reviewed_revision": revision, "feedback": feedback}),
                )?;
                if *accepted {
                    self.unblock_dependents(&task.project, &task.id, &actor.id)?;
                }
                let kind = if *accepted { "accepted" } else { "assignment" };
                self.queue(
                    &task.project,
                    task_message(
                        &actor.id,
                        &task.assignee,
                        kind,
                        feedback.clone(),
                        &task.id,
                        task.revision,
                    ),
                )?;
                Ok(json!(self.assemble(task)?))
            }
            Operation::TaskCreatePool {
                description,
                acceptance,
                eligible,
                reviewer,
                dependencies,
                start_deadline,
                execution_timeout_seconds,
                review_timeout_seconds,
                ..
            } => {
                text(description)?;
                text(acceptance)?;
                self.budget_available()?;
                ensure!(
                    self.active_task_count(&actor.project)? < MAX_ACTIVE_TASKS_PER_PROJECT,
                    capacity_exceeded("Active task queue is full; archive eligible completed work")
                );
                ensure!(
                    !eligible.is_empty() && eligible.len() <= MAX_ELIGIBLES,
                    invalid_input("Eligible list must name between 1 and 100 agents")
                );
                let mut eligible_ids = Vec::new();
                let mut seen = HashSet::new();
                for name in eligible {
                    let member = self.resolve(actor, name)?;
                    if seen.insert(member.id.clone()) {
                        eligible_ids.push(member.id);
                    }
                }
                let reviewer = match reviewer {
                    Some(name) => self.resolve(actor, name)?,
                    None => actor.clone(),
                };
                ensure!(
                    !eligible_ids.contains(&reviewer.id),
                    invalid_state("Reviewer cannot be an eligible claimant")
                );
                let dependencies =
                    self.validated_dependencies(&actor.project, dependencies, None)?;
                let start_deadline = pending_deadline(start_deadline.as_deref())?;
                let execution_timeout =
                    execution_timeout_seconds.map(timeout_seconds).transpose()?;
                let review_timeout = review_timeout_seconds.map(timeout_seconds).transpose()?;
                let task = Task {
                    id: Uuid::new_v4().to_string(),
                    project: actor.project.clone(),
                    issuer: actor.id.clone(),
                    assignee: String::new(),
                    reviewer: reviewer.id,
                    description: description.clone(),
                    acceptance: acceptance.clone(),
                    state: "queued".into(),
                    revision: 1,
                    version: 1,
                    result: None,
                    evidence: None,
                    feedback: vec![],
                    history_truncated: false,
                    attempts: vec![],
                    evidence_records: vec![],
                    created_seq: self.next_sequence(&actor.project)?,
                    archived: false,
                    dependencies: dependencies.clone(),
                    eligible: eligible_ids.clone(),
                    wait_reason: None,
                    start_deadline,
                    execution_timeout_seconds: execution_timeout,
                    review_timeout_seconds: review_timeout,
                    execution_deadline: None,
                    review_deadline: None,
                    review_overdue: false,
                    executing_run: None,
                };
                self.insert_task(&task)?;
                self.set_task_eligibles(&task.id, &eligible_ids)?;
                self.set_task_dependencies(&task.id, &dependencies)?;
                let mut task = task;
                if !self.blocking_dependencies(&task.id)?.is_empty() {
                    task.state = "blocked".into();
                    self.update_task_state(&task)?;
                }
                self.record(
                    &task.project,
                    "task_pool_created",
                    &actor.id,
                    Some(&task.id),
                    None,
                    json!({"revision": task.revision, "state": task.state, "eligible": task.eligible, "dependencies": task.dependencies}),
                )?;
                if task.state == "queued" {
                    self.notify_available(&task, &actor.id)?;
                }
                Ok(json!(self.assemble(task)?))
            }
            Operation::TaskClaim {
                task_id,
                expected_version,
                ..
            } => {
                self.budget_available()?;
                let mut task = self.actor_task(actor, task_id)?;
                ensure!(
                    task.assignee.is_empty(),
                    invalid_state("Task is already assigned")
                );
                ensure!(
                    task.state == "queued",
                    invalid_state("Task is not open for claiming")
                );
                ensure!(
                    self.task_eligibles(&task.id)?.contains(&actor.id),
                    unauthorized("Agent is not eligible for this task")
                );
                if let Some(expected) = expected_version {
                    ensure!(
                        *expected == task.version,
                        version_conflict("Task version does not match", task.version)
                    );
                }
                ensure!(
                    self.blocking_dependencies(&task.id)?.is_empty(),
                    dependency_blocked("Task prerequisites are not complete")
                );
                let other_running = self.count(
                    "SELECT COUNT(*) AS count FROM tasks WHERE assignee = ? AND state IN ('running','cancel_requested')",
                    &[actor.id.as_str()],
                )? > 0;
                ensure!(
                    !other_running,
                    invalid_state("An assigned task is already running")
                );
                task.assignee = actor.id.clone();
                task.version += 1;
                self.update_task_state(&task)?;
                self.retire_available(&task.id)?;
                self.record(
                    &task.project,
                    "task_claimed",
                    &actor.id,
                    Some(&task.id),
                    None,
                    json!({"revision": task.revision, "assignee": actor.id}),
                )?;
                self.queue(
                    &task.project,
                    task_message(
                        &actor.id,
                        &actor.id,
                        "assignment",
                        "Task claimed. Read the task and explicitly start its revision.".into(),
                        &task.id,
                        task.revision,
                    ),
                )?;
                Ok(json!(self.assemble(task)?))
            }
            Operation::TaskProgress {
                task_id,
                revision,
                note,
                attempt_id,
                waiting_reason,
                expected_version,
                ..
            } => {
                text(note)?;
                if let Some(reason) = waiting_reason {
                    text(reason)?;
                }
                let task = self.actor_task(actor, task_id)?;
                ensure!(
                    task.assignee == actor.id,
                    unauthorized("Task is not assigned to this agent")
                );
                ensure!(
                    task.revision == *revision,
                    stale_revision("Task revision is stale", task.version)
                );
                if let Some(expected) = expected_version {
                    ensure!(
                        *expected == task.version,
                        version_conflict("Task version does not match", task.version)
                    );
                }
                ensure!(task.state == "running", invalid_state("Task is not running"));
                let attempt = self
                    .active_attempts(&task.id)?
                    .into_iter()
                    .find(|attempt| {
                        attempt.owner == actor.id
                            && attempt.run == run
                            && attempt.revision == task.revision
                    })
                    .ok_or_else(|| {
                        stale_attempt("No active attempt owned by this session", task.version)
                    })?;
                if let Some(expected) = attempt_id {
                    ensure!(
                        *expected == attempt.id,
                        stale_attempt("Execution attempt was replaced", task.version)
                    );
                }
                self.record(
                    &task.project,
                    "task_progress",
                    &actor.id,
                    Some(&task.id),
                    Some(&attempt.id),
                    json!({"revision": task.revision, "note": note, "waiting_reason": waiting_reason}),
                )?;
                let mut value = serde_json::to_value(&self.assemble(task)?)?;
                value["attempt_id"] = json!(attempt.id);
                Ok(value)
            }
            Operation::TaskCancel {
                task_id,
                reason,
                expected_version,
                ..
            } => {
                text(reason)?;
                let task = self.actor_task(actor, task_id)?;
                ensure!(
                    task.issuer == actor.id || actor.program == OPERATOR_PROGRAM,
                    unauthorized("Only the issuer or operator can cancel a task")
                );
                if let Some(expected) = expected_version {
                    ensure!(
                        *expected == task.version,
                        version_conflict("Task version does not match", task.version)
                    );
                }
                Ok(json!(self.cancel_task(actor, task, reason)?))
            }
            Operation::TaskFinishCancel {
                task_id,
                revision,
                reason,
                attempt_id,
                expected_version,
                ..
            } => {
                text(reason)?;
                let task = self.actor_task(actor, task_id)?;
                ensure!(
                    task.assignee == actor.id,
                    unauthorized("Task is not assigned to this agent")
                );
                ensure!(
                    task.revision == *revision,
                    stale_revision("Task revision is stale", task.version)
                );
                if let Some(expected) = expected_version {
                    ensure!(
                        *expected == task.version,
                        version_conflict("Task version does not match", task.version)
                    );
                }
                ensure!(
                    task.state == "cancel_requested",
                    invalid_state("Task has no pending cancellation")
                );
                let active = self.confirmable_attempts(&task, actor, run)?;
                if let Some(expected) = attempt_id {
                    let attempt = active
                        .iter()
                        .find(|attempt| attempt.id == *expected)
                        .ok_or_else(|| {
                            stale_attempt("No active execution attempt with that ID", task.version)
                        })?;
                    ensure!(
                        attempt.owner == actor.id && attempt.run == run && attempt.revision == task.revision,
                        stale_attempt("Execution attempt does not belong to this session", task.version)
                    );
                }
                let owned = active.iter().find(|attempt| {
                    attempt.owner == actor.id && attempt.run == run && attempt.revision == task.revision
                });
                match owned {
                    Some(attempt) => {
                        if let Some(expected) = attempt_id {
                            ensure!(
                                *expected == attempt.id,
                                stale_attempt("The reported stop does not match the owning attempt", task.version)
                            );
                        }
                        diesel::sql_query("UPDATE attempts SET certainty='finished', outcome='cancelled', finished_at=? WHERE id=?")
                            .bind::<BigInt, _>(now() as i64)
                            .bind::<Text, _>(&attempt.id)
                            .execute(&mut *self.connection.borrow_mut())?;
                        self.record(
                            &task.project,
                            "attempt_stopped",
                            &actor.id,
                            Some(&task.id),
                            Some(&attempt.id),
                            json!({"revision": task.revision, "reason": reason}),
                        )?;
                    }
                    None => return Err(stale_attempt("Only the owning execution run can confirm stop", task.version)),
                }
                Ok(json!(self.mark_cancelled(task, actor, reason, false)?))
            }
            Operation::TaskFail {
                task_id,
                revision,
                reason,
                evidence_ids,
                attempt_id,
                expected_version,
                ..
            } => {
                text(reason)?;
                let task = self.actor_task(actor, task_id)?;
                ensure!(
                    task.assignee == actor.id,
                    unauthorized("Task is not assigned to this agent")
                );
                ensure!(
                    task.revision == *revision,
                    stale_revision("Task revision is stale", task.version)
                );
                if let Some(expected) = expected_version {
                    ensure!(
                        *expected == task.version,
                        version_conflict("Task version does not match", task.version)
                    );
                }
                ensure!(task.state == "running", invalid_state("Task is not running"));
                let attempt = self
                    .confirmable_attempts(&task, actor, run)?
                    .into_iter()
                    .find(|attempt| {
                        attempt.owner == actor.id
                            && attempt.run == run
                            && attempt.revision == task.revision
                    })
                    .ok_or_else(|| {
                        stale_attempt("No active attempt owned by this session", task.version)
                    })?;
                if let Some(expected) = attempt_id {
                    ensure!(
                        *expected == attempt.id,
                        stale_attempt("Execution attempt was replaced", task.version)
                    );
                }
                self.validate_evidence_ids(&task, evidence_ids)?;
                diesel::sql_query("UPDATE attempts SET certainty='finished', outcome='failed', finished_at=? WHERE id=?")
                    .bind::<BigInt, _>(now() as i64)
                    .bind::<Text, _>(&attempt.id)
                    .execute(&mut *self.connection.borrow_mut())?;
                let mut task = task;
                task.state = "failed".into();
                task.execution_deadline = None;
                task.review_deadline = None;
                task.version += 1;
                self.update_task_state(&task)?;
                self.record(
                    &task.project,
                    "task_failed",
                    &actor.id,
                    Some(&task.id),
                    Some(&attempt.id),
                    json!({"revision": task.revision, "reason": reason, "evidence_ids": evidence_ids}),
                )?;
                if task.issuer != actor.id {
                    self.queue(
                        &task.project,
                        task_message(
                            &actor.id,
                            &task.issuer,
                            "failed",
                            format!("Task failed: {reason}. Check the evidence, then retry or reassign it."),
                            &task.id,
                            task.revision,
                        ),
                    )?;
                }
                let mut value = serde_json::to_value(&self.assemble(task)?)?;
                value["attempt_id"] = json!(attempt.id);
                Ok(value)
            }
            Operation::TaskRetry {
                task_id,
                reason,
                start_deadline,
                clear_start_deadline,
                override_uncertain,
                expected_version,
                ..
            } => {
                text(reason)?;
                self.budget_available()?;
                let mut task = self.actor_task(actor, task_id)?;
                ensure!(
                    task.issuer == actor.id || actor.program == OPERATOR_PROGRAM,
                    unauthorized("Only the issuer or operator can retry a task")
                );
                ensure!(
                    matches!(task.state.as_str(), "failed" | "expired" | "cancelled"),
                    invalid_state("Only failed, expired or cancelled work can be retried")
                );
                if let Some(expected) = expected_version {
                    ensure!(
                        *expected == task.version,
                        version_conflict("Task version does not match", task.version)
                    );
                }
                self.release_attempts(&task, actor, *override_uncertain, reason)?;
                replace_start_deadline(
                    &mut task,
                    start_deadline.as_deref(),
                    *clear_start_deadline,
                )?;
                task.revision = task
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| invalid_state("Revision overflow"))?;
                task.result = None;
                task.evidence = None;
                task.execution_deadline = None;
                task.review_deadline = None;
                task.state = if self.blocking_dependencies(&task.id)?.is_empty() {
                    "queued".into()
                } else {
                    "blocked".into()
                };
                task.version += 1;
                self.update_task_state(&task)?;
                self.record(
                    &task.project,
                    "task_retried",
                    &actor.id,
                    Some(&task.id),
                    None,
                    json!({"revision": task.revision, "state": task.state, "reason": reason}),
                )?;
                if task.assignee.is_empty() {
                    if task.state == "queued" {
                        self.retire_available(&task.id)?;
                        self.notify_available(&task, &actor.id)?;
                    }
                } else {
                    self.ack_task_notification(&task.id, &task.assignee)?;
                    let body = if task.state == "blocked" {
                        "Task retried but waiting for prerequisites; it will notify when ready.".into()
                    } else {
                        "Task retried. Read the new revision and explicitly start it.".into()
                    };
                    self.queue(
                        &task.project,
                        task_message(
                            &actor.id,
                            &task.assignee,
                            "assignment",
                            body,
                            &task.id,
                            task.revision,
                        ),
                    )?;
                }
                Ok(json!(self.assemble(task)?))
            }
            Operation::TaskReassign {
                task_id,
                assignee,
                reason,
                reviewer,
                start_deadline,
                clear_start_deadline,
                override_uncertain,
                expected_version,
                ..
            } => {
                text(reason)?;
                self.budget_available()?;
                let mut task = self.actor_task(actor, task_id)?;
                ensure!(
                    task.issuer == actor.id || actor.program == OPERATOR_PROGRAM,
                    unauthorized("Only the issuer or operator can reassign a task")
                );
                ensure!(
                    matches!(
                        task.state.as_str(),
                        "queued" | "blocked" | "failed" | "expired" | "cancelled"
                    ),
                    invalid_state("Only non-running work can be reassigned")
                );
                if let Some(expected) = expected_version {
                    ensure!(
                        *expected == task.version,
                        version_conflict("Task version does not match", task.version)
                    );
                }
                self.release_attempts(&task, actor, *override_uncertain, reason)?;
                let previous = task.assignee.clone();
                let new_assignee = self.resolve(actor, assignee)?;
                let new_reviewer = match reviewer {
                    Some(name) => self.resolve(actor, name)?,
                    None => self.agent(&task.project, &task.reviewer.clone())?,
                };
                ensure!(
                    new_assignee.id != new_reviewer.id,
                    invalid_state("Assignee cannot review its own task")
                );
                replace_start_deadline(
                    &mut task,
                    start_deadline.as_deref(),
                    *clear_start_deadline,
                )?;
                task.assignee = new_assignee.id;
                task.reviewer = new_reviewer.id;
                task.revision = task
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| invalid_state("Revision overflow"))?;
                task.result = None;
                task.evidence = None;
                task.execution_deadline = None;
                task.review_deadline = None;
                task.state = if self.blocking_dependencies(&task.id)?.is_empty() {
                    "queued".into()
                } else {
                    "blocked".into()
                };
                task.version += 1;
                self.update_task_state(&task)?;
                self.record(
                    &task.project,
                    "task_reassigned",
                    &actor.id,
                    Some(&task.id),
                    None,
                    json!({"revision": task.revision, "state": task.state, "assignee": task.assignee, "reviewer": task.reviewer, "previous": previous, "reason": reason}),
                )?;
                if !previous.is_empty() && previous != task.assignee {
                    self.ack_task_notification(&task.id, &previous)?;
                }
                if previous.is_empty() {
                    self.retire_available(&task.id)?;
                }
                self.ack_task_notification(&task.id, &task.assignee)?;
                let body = if task.state == "blocked" {
                    "Task reassigned but waiting for prerequisites; it will notify when ready.".into()
                } else {
                    "Task reassigned. Read the new revision and explicitly start it.".into()
                };
                self.queue(
                    &task.project,
                    task_message(
                        &actor.id,
                        &task.assignee,
                        "assignment",
                        body,
                        &task.id,
                        task.revision,
                    ),
                )?;
                Ok(json!(self.assemble(task)?))
            }
            Operation::TaskSetDependencies {
                task_id,
                dependencies,
                expected_version,
                ..
            } => {
                let mut task = self.actor_task(actor, task_id)?;
                ensure!(
                    task.issuer == actor.id || actor.program == OPERATOR_PROGRAM,
                    unauthorized("Only the issuer or operator can change prerequisites")
                );
                ensure!(
                    matches!(task.state.as_str(), "queued" | "blocked") && task.attempts.is_empty(),
                    invalid_state("Prerequisites can change only before the first start")
                );
                if let Some(expected) = expected_version {
                    ensure!(
                        *expected == task.version,
                        version_conflict("Task version does not match", task.version)
                    );
                }
                let validated =
                    self.validated_dependencies(&task.project, dependencies, Some(&task.id))?;
                self.check_dependency_cycles(&task.id, &validated)?;
                let previous = self.task_dependencies(&task.id)?;
                self.set_task_dependencies(&task.id, &validated)?;
                task.dependencies = validated.clone();
                let changed = previous != validated;
                task.state = if self.blocking_dependencies(&task.id)?.is_empty() {
                    "queued".into()
                } else {
                    "blocked".into()
                };
                task.version += 1;
                self.update_task_state(&task)?;
                self.record(
                    &task.project,
                    "task_dependencies_set",
                    &actor.id,
                    Some(&task.id),
                    None,
                    json!({"revision": task.revision, "state": task.state, "dependencies": validated}),
                )?;
                if task.state == "queued" && changed {
                    if task.assignee.is_empty() {
                        self.retire_available(&task.id)?;
                        self.notify_available(&task, &actor.id)?;
                    } else {
                        self.ack_task_notification(&task.id, &task.assignee)?;
                        self.queue(
                            &task.project,
                            task_message(
                                &actor.id,
                                &task.assignee,
                                "assignment",
                                "Prerequisites are clear. Read the revision and explicitly start it."
                                    .into(),
                                &task.id,
                                task.revision,
                            ),
                        )?;
                    }
                }
                Ok(json!(self.assemble(task)?))
            }
            Operation::EvidenceAdd {
                task_id,
                kind,
                attempt_id,
                path,
                hash,
                commit,
                repository,
                branch,
                base,
                head,
                command,
                outcome,
                exit_code,
                summary,
                ..
            } => {
                self.budget_available()?;
                let task = self.actor_task(actor, task_id)?;
                ensure!(
                    task.assignee == actor.id,
                    unauthorized("Task is not assigned to this agent")
                );
                ensure!(
                    task.state == "running",
                    invalid_state("Evidence can only be added while the task is running")
                );
                let attempt = self
                    .active_attempts(&task.id)?
                    .into_iter()
                    .find(|attempt| {
                        attempt.owner == actor.id
                            && attempt.run == run
                            && attempt.revision == task.revision
                    })
                    .ok_or_else(|| {
                        stale_attempt("No active attempt owned by this session", task.version)
                    })?;
                if let Some(expected) = attempt_id {
                    ensure!(
                        *expected == attempt.id,
                        stale_attempt("Execution attempt was replaced", task.version)
                    );
                }
                ensure!(
                    task.evidence_records.len() < MAX_EVIDENCE_PER_TASK as usize,
                    capacity_exceeded("Evidence capacity reached for this task")
                );
                for value in [
                    path.as_deref(),
                    hash.as_deref(),
                    commit.as_deref(),
                    repository.as_deref(),
                    branch.as_deref(),
                    base.as_deref(),
                    head.as_deref(),
                    command.as_deref(),
                    outcome.as_deref(),
                    summary.as_deref(),
                ]
                .into_iter()
                .flatten()
                {
                    text(value)?;
                }
                ensure!(serde_json::to_vec(&json!({"kind":kind,"path":path,"hash":hash,"commit":commit,
                    "repository":repository,"branch":branch,"base":base,"head":head,"command":command,
                    "outcome":outcome,"exit_code":exit_code,"summary":summary}))?.len() <= crate::MAX_TEXT,
                    invalid_input("Combined evidence metadata must be at most 8192 bytes"));
                ensure!(
                    matches!(kind.as_str(), "file" | "commit" | "diff" | "test"),
                    invalid_input("Evidence kind must be file, commit, diff or test")
                );
                let path = match kind.as_str() {
                    "file" => {
                        let path = path
                            .as_deref()
                            .ok_or_else(|| invalid_input("File evidence requires a relative path"))?;
                        Some(normalize_relative_path(path)?)
                    }
                    "diff" => match (path.as_deref(), base.as_deref(), head.as_deref()) {
                        (Some(path), _, _) => Some(normalize_relative_path(path)?),
                        (None, Some(_), Some(_)) => None,
                        _ => {
                            return Err(invalid_input(
                                "Diff evidence requires a path or both base and head commits",
                            ))
                        }
                    },
                    _ => path.as_deref().map(normalize_relative_path).transpose()?,
                };
                if kind == "commit" {
                    ensure!(
                        commit.is_some() && repository.is_some(),
                        invalid_input("Commit evidence requires repository and commit IDs")
                    );
                }
                if kind == "test" {
                    ensure!(
                        command.is_some(),
                        invalid_input("Test evidence requires a command label")
                    );
                    ensure!(
                        matches!(outcome.as_deref(), Some("passed" | "failed" | "not_run")),
                        invalid_input("Test outcome must be passed, failed or not_run")
                    );
                }
                ensure!(
                    exit_code.is_none() || kind == "test",
                    invalid_input("Exit codes apply only to test evidence")
                );
                let evidence_id = Uuid::new_v4().to_string();
                diesel::sql_query("INSERT INTO evidence(id, task_id, attempt_id, kind, path, hash, commit_id, repository, branch, base, head, command, outcome, exit_code, summary, device, verified, created_seq) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,NULL,0,?)")
                    .bind::<Text, _>(&evidence_id)
                    .bind::<Text, _>(&task.id)
                    .bind::<Text, _>(&attempt.id)
                    .bind::<Text, _>(kind)
                    .bind::<Nullable<Text>, _>(&path)
                    .bind::<Nullable<Text>, _>(hash)
                    .bind::<Nullable<Text>, _>(commit)
                    .bind::<Nullable<Text>, _>(repository)
                    .bind::<Nullable<Text>, _>(branch)
                    .bind::<Nullable<Text>, _>(base)
                    .bind::<Nullable<Text>, _>(head)
                    .bind::<Nullable<Text>, _>(command)
                    .bind::<Nullable<Text>, _>(outcome)
                    .bind::<Nullable<BigInt>, _>(exit_code)
                    .bind::<Nullable<Text>, _>(summary)
                    .bind::<BigInt, _>(self.next_sequence(&task.project)? as i64)
                    .execute(&mut *self.connection.borrow_mut())?;
                self.record(
                    &task.project,
                    "evidence_added",
                    &actor.id,
                    Some(&task.id),
                    Some(&attempt.id),
                    json!({"revision": task.revision, "evidence_id": evidence_id, "kind": kind}),
                )?;
                Ok(json!({"evidence_id": evidence_id, "task_id": task.id, "verified": false}))
            }
            Operation::FileReserve {
                paths,
                mode,
                task_id,
                attempt_id,
                ttl_seconds,
                ..
            } => {
                self.budget_available()?;
                let workspace = self.physical_root(actor)?;
                ensure!(
                    matches!(mode.as_str(), "exclusive" | "shared"),
                    invalid_input("Reservation mode must be exclusive or shared")
                );
                ensure!(
                    !paths.is_empty() && paths.len() <= MAX_PATHS,
                    invalid_input("Reserve between 1 and 100 paths")
                );
                let mut normalized = Vec::new();
                let mut seen = HashSet::new();
                for path in paths {
                    let path = self.reservation_path(actor, &workspace, path)?;
                    if seen.insert(path.clone()) {
                        normalized.push(path);
                    }
                }
                let ttl = ttl_seconds.unwrap_or(RESERVATION_TTL_DEFAULT);
                ensure!(
                    (1..=RESERVATION_TTL_MAX).contains(&ttl),
                    invalid_input("Reservation TTL must be between 1 and 3600 seconds")
                );
                let mut attempt_link = attempt_id.clone();
                if let Some(linked) = &attempt_link {
                    ensure!(
                        self.active_attempt_owned_by_run(linked, &actor.id, run)?,
                        invalid_state("Reservation attempt is not active")
                    );
                }
                let mut task_link = None;
                if let Some(id) = task_id {
                    let task = self.actor_task(actor, id)?;
                    ensure!(
                        task.assignee == actor.id,
                        unauthorized("Task is not assigned to this agent")
                    );
                    ensure!(task.state == "running", invalid_state("Task is not running"));
                    let attempt = self
                        .active_attempts(&task.id)?
                        .into_iter()
                        .find(|attempt| {
                            attempt.owner == actor.id
                                && attempt.run == run
                                && attempt.revision == task.revision
                        })
                        .ok_or_else(|| {
                            stale_attempt("No active attempt owned by this session", task.version)
                        })?;
                    if let Some(expected) = &attempt_link {
                        ensure!(
                            *expected == attempt.id,
                            stale_attempt("Execution attempt was replaced", task.version)
                        );
                    }
                    attempt_link = Some(attempt.id);
                    task_link = Some(task.id);
                }
                let now_ms = now();
                let rows = diesel::sql_query("SELECT id, workspace, path, mode, owner, task_id, attempt_id, created_at, expires_at, created_seq FROM reservations WHERE workspace = ? ORDER BY created_seq")
                    .bind::<Text, _>(&workspace)
                    .load::<ReservationRow>(&mut *self.connection.borrow_mut())?;
                let live = rows
                    .iter()
                    .filter(|row| row.expires_at as u64 > now_ms)
                    .count();
                ensure!(
                    live + normalized.len() <= MAX_RESERVATIONS_PER_WORKSPACE as usize,
                    capacity_exceeded("Reservation capacity reached for this workspace")
                );
                for row in rows.iter().filter(|row| row.expires_at as u64 > now_ms) {
                    if row.mode == "shared" && mode == "shared" {
                        continue;
                    }
                    for path in &normalized {
                        if paths_overlap(&row.path, path) {
                            return Err(reservation_conflict(&format!(
                                "Path {} has an active reservation until {}",
                                row.path, row.expires_at
                            )));
                        }
                    }
                }
                let expires_at = now_ms + ttl * 1000;
                let scope = diesel::sql_query("SELECT w.id, w.space_id, w.root, w.repository_id, w.model, w.branch, w.base_commit FROM workspaces AS w JOIN space_members AS member ON member.space_id = w.space_id AND member.agent = ? WHERE w.root = ?")
                    .bind::<Text, _>(&actor.id)
                    .bind::<Text, _>(&workspace)
                    .get_result::<WorkspaceRow>(&mut *self.connection.borrow_mut())
                    .optional()?;
                let mut reservation_ids = Vec::new();
                for path in &normalized {
                    let reservation_id = Uuid::new_v4().to_string();
                    diesel::sql_query("INSERT INTO reservations(id, workspace, path, mode, owner, task_id, attempt_id, created_at, expires_at, created_seq, space_id, repository_id, workspace_id) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
                        .bind::<Text, _>(&reservation_id)
                        .bind::<Text, _>(&workspace)
                        .bind::<Text, _>(path)
                        .bind::<Text, _>(mode)
                        .bind::<Text, _>(&actor.id)
                        .bind::<Nullable<Text>, _>(&task_link)
                        .bind::<Nullable<Text>, _>(&attempt_link)
                        .bind::<BigInt, _>(now_ms as i64)
                        .bind::<BigInt, _>(expires_at as i64)
                        .bind::<BigInt, _>(self.next_sequence(&workspace)? as i64)
                        .bind::<Nullable<Text>, _>(scope.as_ref().map(|scope| scope.space_id.as_str()))
                        .bind::<Nullable<Text>, _>(scope.as_ref().and_then(|scope| scope.repository_id.as_deref()))
                        .bind::<Nullable<Text>, _>(scope.as_ref().map(|scope| scope.id.as_str()))
                        .execute(&mut *self.connection.borrow_mut())?;
                    self.record(
                        &actor.project,
                        "reservation_created",
                        &actor.id,
                        Some(&reservation_id),
                        attempt_link.as_deref(),
                        json!({"path": path, "mode": mode, "task_id": task_link, "expires_at": expires_at}),
                    )?;
                    reservation_ids.push(reservation_id);
                }
                let (overlap_warnings, warnings_truncated) =
                    self.reservation_overlaps(actor, &normalized, mode, now_ms)?;
                Ok(json!({"reservation_ids": reservation_ids, "expires_at": expires_at,
                    "overlap_warnings": overlap_warnings, "warnings_truncated": warnings_truncated}))
            }
            Operation::FileRenew {
                reservation_ids,
                ttl_seconds,
                ..
            } => {
                ensure!(
                    !reservation_ids.is_empty() && reservation_ids.len() <= MAX_PATHS,
                    invalid_input("Renew between 1 and 100 reservations")
                );
                let ttl = ttl_seconds.unwrap_or(RESERVATION_TTL_DEFAULT);
                ensure!(
                    (1..=RESERVATION_TTL_MAX).contains(&ttl),
                    invalid_input("Reservation TTL must be between 1 and 3600 seconds")
                );
                let mut renewed = Vec::new();
                for id in reservation_ids {
                    let row = self.reservation_row(&self.physical_root(actor)?, id)?;
                    ensure!(row.owner == actor.id, unauthorized("Reservation is owned by another agent"));
                    ensure!(
                        row.expires_at as u64 > now(),
                        invalid_state("Reservation already expired; create a new one")
                    );
                    if let Some(attempt) = &row.attempt_id {
                        ensure!(
                            self.active_attempt_owned_by_run(attempt, &actor.id, run)?,
                            invalid_state("Reservation attempt is no longer active")
                        );
                    }
                    let expires_at = now() + ttl * 1000;
                    diesel::sql_query("UPDATE reservations SET expires_at = ? WHERE id = ?")
                        .bind::<BigInt, _>(expires_at as i64)
                        .bind::<Text, _>(id)
                        .execute(&mut *self.connection.borrow_mut())?;
                    self.record(
                        &actor.project,
                        "reservation_renewed",
                        &actor.id,
                        Some(id),
                        row.attempt_id.as_deref(),
                        json!({"path": row.path, "expires_at": expires_at}),
                    )?;
                    renewed.push(id.clone());
                }
                Ok(json!({"renewed": renewed}))
            }
            Operation::FileRelease {
                reservation_ids, ..
            } => {
                ensure!(
                    !reservation_ids.is_empty() && reservation_ids.len() <= MAX_PATHS,
                    invalid_input("Release between 1 and 100 reservations")
                );
                let mut released = Vec::new();
                for id in reservation_ids {
                    let row = self
                        .reservation_by_id(id)?
                        .filter(|row| self.physical_root(actor).is_ok_and(|root| row.workspace == root));
                    let Some(row) = row else { continue };
                    ensure!(row.owner == actor.id, unauthorized("Reservation is owned by another agent"));
                    diesel::sql_query("DELETE FROM reservations WHERE id = ?")
                        .bind::<Text, _>(id)
                        .execute(&mut *self.connection.borrow_mut())?;
                    self.record(
                        &actor.project,
                        "reservation_released",
                        &actor.id,
                        Some(id),
                        row.attempt_id.as_deref(),
                        json!({"path": row.path}),
                    )?;
                    released.push(id.clone());
                }
                Ok(json!({"released": released}))
            }
            _ => Err(invalid_state("Operation requires live session handling")),
        }
    }

    /// Validated prerequisite list: bounded, deduplicated, project-scoped and never self-referential.
    fn validated_dependencies(
        &self,
        project: &str,
        dependencies: &[String],
        exclude: Option<&str>,
    ) -> Result<Vec<String>> {
        ensure!(
            dependencies.len() <= MAX_DEPENDENCIES,
            invalid_input("At most 100 dependency edges per task")
        );
        let mut validated = Vec::new();
        let mut seen = HashSet::new();
        for dependency in dependencies {
            ensure!(
                Some(dependency.as_str()) != exclude,
                invalid_input("A task cannot depend on itself")
            );
            if !seen.insert(dependency.clone()) {
                continue;
            }
            self.operator_task(project, dependency)?;
            validated.push(dependency.clone());
        }
        Ok(validated)
    }

    /// A new edge from `task_id` to any prerequisite cycles when the task is reachable from it.
    fn check_dependency_cycles(&self, task_id: &str, prerequisites: &[String]) -> Result<()> {
        let mut visited = HashSet::new();
        let mut pending: Vec<String> = prerequisites.to_vec();
        while let Some(current) = pending.pop() {
            if current == task_id {
                return Err(dependency_cycle("Dependencies must not form a cycle"));
            }
            if !visited.insert(current.clone()) {
                continue;
            }
            pending.extend(self.task_dependencies(&current)?);
        }
        Ok(())
    }

    /// Task notifications end through their transition; this clears every outstanding one.
    fn ack_task_notification(&self, task_id: &str, agent: &str) -> Result<()> {
        diesel::sql_query("UPDATE messages SET acknowledged = 1 WHERE recipient = ? AND task_id = ? AND acknowledged = 0")
            .bind::<Text, _>(agent)
            .bind::<Text, _>(task_id)
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    /// Pool claim notices end when the queue closes or the task starts.
    fn retire_available(&self, task_id: &str) -> Result<()> {
        diesel::sql_query("UPDATE messages SET acknowledged = 1 WHERE task_id = ? AND kind = 'available' AND acknowledged = 0")
            .bind::<Text, _>(task_id)
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    fn notify_available(&self, task: &Task, actor: &str) -> Result<()> {
        for agent in self.task_eligibles(&task.id)? {
            self.queue(
                &task.project,
                task_message(
                    actor,
                    &agent,
                    "available",
                    "An eligible task is open in the shared pool; claim it with warp_task_claim."
                        .into(),
                    &task.id,
                    task.revision,
                ),
            )?;
        }
        Ok(())
    }

    /// Newly satisfied prerequisites notify only the tasks that were waiting on them.
    fn unblock_dependents(&self, project: &str, prerequisite: &str, actor: &str) -> Result<()> {
        let rows = diesel::sql_query(format!(
            "SELECT {TASK_COLUMNS} FROM tasks WHERE project = ? AND state = 'blocked' AND id IN (SELECT task_id FROM task_dependencies WHERE prerequisite_id = ?)"
        ))
        .bind::<Text, _>(project)
        .bind::<Text, _>(prerequisite)
        .load::<TaskRow>(&mut *self.connection.borrow_mut())?;
        for task in rows.into_iter().map(TaskRow::task) {
            if !self.blocking_dependencies(&task.id)?.is_empty() {
                continue;
            }
            let mut task = task;
            task.state = "queued".into();
            task.version += 1;
            self.update_task_state(&task)?;
            self.record(
                project,
                "task_unblocked",
                actor,
                Some(&task.id),
                None,
                json!({"revision": task.revision}),
            )?;
            if task.assignee.is_empty() {
                self.notify_available(&task, actor)?;
            } else {
                self.ack_task_notification(&task.id, &task.assignee)?;
                self.queue(
                    project,
                    task_message(
                        actor,
                        &task.assignee,
                        "assignment",
                        "Prerequisites are complete. Read the revision and explicitly start it."
                            .into(),
                        &task.id,
                        task.revision,
                    ),
                )?;
            }
        }
        Ok(())
    }

    /// Issuer cancellation: immediate while unstarted or submitted; a stop request while running.
    fn cancel_task(&self, actor: &Agent, mut task: Task, reason: &str) -> Result<Task> {
        match task.state.as_str() {
            "queued" | "blocked" | "submitted" => {
                self.mark_cancelled(task, actor, reason, false)
            }
            "running" => {
                let active = self.active_attempts(&task.id)?;
                if actor.program == OPERATOR_PROGRAM && active.is_empty() && self.unresolved_attempts(&task.id, None)?.is_empty() && !task.attempts.is_empty() {
                    return self.mark_cancelled(task, actor, reason, false);
                }
                task.state = "cancel_requested".into();
                task.version += 1;
                self.update_task_state(&task)?;
                self.record(
                    &task.project,
                    "task_cancel_requested",
                    &actor.id,
                    Some(&task.id),
                    None,
                    json!({"revision": task.revision, "reason": reason}),
                )?;
                if !task.assignee.is_empty() {
                    self.queue(
                        &task.project,
                        task_message(
                            &actor.id,
                            &task.assignee,
                            "cancel_requested",
                            "Stop the current work, report what happened, then confirm the cancellation with warp_task_finish_cancel."
                                .into(),
                            &task.id,
                            task.revision,
                        ),
                    )?;
                }
                Ok(task)
            }
            "cancel_requested" => {
                ensure!(
                    actor.program == OPERATOR_PROGRAM,
                    unauthorized(
                        "Cancellation is awaiting the owning session's confirmation"
                    )
                );
                ensure!(
                    self.unresolved_attempts(&task.id, None)?.is_empty() && !task.attempts.is_empty(),
                    execution_unknown("Execution is still owned by a live attempt; confirm the stop or use the explicit override")
                );
                self.mark_cancelled(task, actor, reason, false)
            }
            _ => Err(invalid_state("Task cannot be cancelled from its current state")),
        }
    }

    /// Terminal cancellation: fence every active attempt, then close delivery and notify.
    fn mark_cancelled(
        &self,
        mut task: Task,
        actor: &Agent,
        reason: &str,
        override_uncertain: bool,
    ) -> Result<Task> {
        let unresolved = self.unresolved_attempts(&task.id, None)?;
        ensure!(unresolved.is_empty() || override_uncertain,
            execution_unknown("Confirm stopped execution or use the explicit operator override"));
        for attempt in unresolved {
            diesel::sql_query("UPDATE attempts SET certainty='unknown', outcome='overridden', finished_at=NULL WHERE id=?")
                .bind::<Text, _>(&attempt.id)
                .execute(&mut *self.connection.borrow_mut())?;
            self.record(
                &task.project,
                "task_interrupted",
                &actor.id,
                Some(&task.id),
                Some(&attempt.id),
                json!({"revision": task.revision, "reason": reason, "override": override_uncertain}),
            )?;
        }
        task.state = "cancelled".into();
        task.start_deadline = None;
        task.execution_deadline = None;
        task.review_deadline = None;
        task.version += 1;
        self.update_task_state(&task)?;
        self.retire_available(&task.id)?;
        if !task.assignee.is_empty() {
            self.ack_task_notification(&task.id, &task.assignee)?;
        }
        self.record(
            &task.project,
            "task_cancelled",
            &actor.id,
            Some(&task.id),
            None,
            json!({"revision": task.revision, "reason": reason, "override": override_uncertain}),
        )?;
        if !task.assignee.is_empty() {
            self.queue(
                &task.project,
                task_message(
                    &actor.id,
                    &task.assignee,
                    "cancelled",
                    format!("Task cancelled: {reason}"),
                    &task.id,
                    task.revision,
                ),
            )?;
        }
        self.assemble(task)
    }

    /// Retry and reassignment require the previous execution to be known stopped or explicitly overridden.
    fn release_attempts(
        &self,
        task: &Task,
        actor: &Agent,
        override_uncertain: bool,
        reason: &str,
    ) -> Result<()> {
        // Cancellation overrides do not authorize a later execution grant.
        let active = self.unresolved_attempts(&task.id, Some(task.revision))?;
        if active.is_empty() {
            return Ok(());
        }
        ensure!(
            actor.program == OPERATOR_PROGRAM && override_uncertain,
            execution_unknown("Previous execution outcome is unknown; confirm the stop or use the explicit override")
        );
        for attempt in active {
            diesel::sql_query("UPDATE attempts SET certainty='unknown', outcome='overridden', finished_at=NULL WHERE id=?")
                .bind::<Text, _>(&attempt.id)
                .execute(&mut *self.connection.borrow_mut())?;
            self.record(
                &task.project,
                "task_interrupted",
                &actor.id,
                Some(&task.id),
                Some(&attempt.id),
                json!({"revision": task.revision, "override": override_uncertain, "reason": reason}),
            )?;
        }
        Ok(())
    }

    fn validate_evidence_ids(&self, task: &Task, evidence_ids: &[String]) -> Result<()> {
        for id in evidence_ids {
            let known = self.count(
                "SELECT COUNT(*) AS count FROM evidence WHERE id = ? AND task_id = ?",
                &[id.as_str(), task.id.as_str()],
            )?;
            ensure!(
                known > 0,
                scope_denied("Evidence reference does not belong to this task")
            );
        }
        Ok(())
    }

    fn attempt_certainty(&self, attempt_id: &str, owner: &str) -> Result<Option<String>> {
        Ok(diesel::sql_query("SELECT certainty AS value FROM attempts WHERE id = ? AND owner = ?")
            .bind::<Text, _>(attempt_id)
            .bind::<Text, _>(owner)
            .get_result::<ValueRow>(&mut *self.connection.borrow_mut())
            .optional()?
            .map(|row| row.value))
    }

    fn active_attempt_owned_by_run(&self, attempt_id: &str, owner: &str, run: &str) -> Result<bool> {
        Ok(self.count(
            "SELECT COUNT(*) AS count FROM attempts WHERE id = ? AND owner = ? AND run = ? AND certainty = 'active'",
            &[attempt_id, owner, run],
        )? > 0)
    }
}

/// Panel controller mutations and shared-space projections; agent-scoped reads stay in the first block.
impl Store {
    fn mutate_controller(
        &self,
        project: &str,
        actor: &Agent,
        operation: &ControllerOperation,
    ) -> Result<Value> {
        match operation {
            ControllerOperation::DeviceList | ControllerOperation::InvitationCreate { .. }
            | ControllerOperation::DeviceGrantUpdate { .. } | ControllerOperation::DeviceRevoke { .. }
            | ControllerOperation::RemoteWorkspaceMap { .. } => Err(crate::domain(
                "feature_unavailable", "Device collaboration was retired", false, None)),
            ControllerOperation::ReservationUpdate { reservation_id, workspace, expected_owner, expected_expires_at, ttl_seconds, reason, .. } => {
                text(reason)?;
                let row = self.reservation_row(workspace, reservation_id)?;
                ensure!(row.owner == *expected_owner && row.expires_at as u64 == *expected_expires_at,
                    invalid_state("Reservation changed; refresh and confirm a new intent"));
                let owner = self.agent(project, &row.owner)?;
                ensure!(self.physical_root(&owner)? == *workspace, scope_denied("Reservation belongs to another checkout"));
                if let Some(ttl) = ttl_seconds {
                    ensure!((1..=RESERVATION_TTL_MAX).contains(ttl), invalid_input("Reservation TTL must be between 1 and 3600 seconds"));
                    self.authorize(&owner)?;
                    ensure!(row.expires_at as u64 > now(), invalid_state("Reservation expired; create a new one"));
                    if let Some(attempt) = &row.attempt_id {
                        ensure!(self.count("SELECT COUNT(*) AS count FROM attempts WHERE id = ? AND owner = ? AND certainty = 'active' AND outcome IS NULL AND finished_at IS NULL", &[attempt.as_str(), row.owner.as_str()])? == 1,
                            execution_unknown("Reservation attempt is no longer confirmed active"));
                    }
                    let expires_at = now() + ttl * 1000;
                    diesel::sql_query("UPDATE reservations SET expires_at = ? WHERE id = ?")
                        .bind::<BigInt, _>(expires_at as i64).bind::<Text, _>(reservation_id)
                        .execute(&mut *self.connection.borrow_mut())?;
                    self.record(project, "reservation_renewed", &actor.id, Some(reservation_id), row.attempt_id.as_deref(),
                        json!({"path": row.path, "owner": row.owner, "expires_at": expires_at, "reason": reason, "operator": true}))?;
                    Ok(json!({"renewed": [reservation_id], "expires_at": expires_at}))
                } else {
                    diesel::sql_query("DELETE FROM reservations WHERE id = ?").bind::<Text, _>(reservation_id)
                        .execute(&mut *self.connection.borrow_mut())?;
                    self.record(project, "reservation_released", &actor.id, Some(reservation_id), row.attempt_id.as_deref(),
                        json!({"path": row.path, "owner": row.owner, "reason": reason, "operator": true, "execution_stopped": false}))?;
                    Ok(json!({"released": [reservation_id], "execution_stopped": false}))
                }
            }
            ControllerOperation::SpaceCreate { name, .. } => self.space_create(project, actor, name),
            ControllerOperation::SpaceJoin { space_id, agent, .. } => {
                self.space_join(project, actor, space_id, agent)
            }
            ControllerOperation::SpaceLeave { space_id, agent, .. } => {
                self.space_leave(project, actor, space_id, agent)
            }
            ControllerOperation::WorkspaceMap {
                space_id,
                root,
                repository_id,
                model,
                branch,
                base_commit,
                ..
            } => self.workspace_map(
                project,
                actor,
                space_id,
                root,
                repository_id.as_deref(),
                model,
                branch.as_deref(),
                base_commit.as_deref(),
            ),
            ControllerOperation::EvidenceVerify {
                evidence_id,
                verified,
                ..
            } => self.evidence_verify(project, actor, evidence_id, *verified),
            ControllerOperation::TaskForceCancel {
                task_id,
                reason,
                expected_version,
                ..
            } => self.force_cancel(project, actor, task_id, reason, *expected_version),
            ControllerOperation::TaskArchive { task_id, .. } => {
                self.task_archive(project, actor, task_id)
            }
            ControllerOperation::ArchiveAged { older_than_days, .. } => {
                self.archive_aged(project, actor, *older_than_days)
            }
            ControllerOperation::HistoryPurge {
                expected_sequence,
                archived_tasks,
                acknowledged_messages,
                ..
            } => {
                if let Some(expected) = expected_sequence {
                    let current = self.history_sequence(project)?;
                    ensure!(*expected == current, version_conflict("History changed; refresh the purge preview", current));
                }
                self.history_purge(project, actor, *archived_tasks, *acknowledged_messages)
            },
            _ => Err(invalid_state("Controller operation requires read handling")),
        }
    }

    fn space_create(&self, project: &str, actor: &Agent, name: &str) -> Result<Value> {
        validate_subject(name)?;
        let name = name.trim();
        ensure!(
            !name.eq_ignore_ascii_case("private"),
            invalid_input("The private space is implicit and cannot be created")
        );
        ensure!(
            self.count("SELECT COUNT(*) AS count FROM spaces WHERE name = ?", &[name])? == 0,
            invalid_input("A space with this name already exists")
        );
        let space_id = Uuid::new_v4().to_string();
        diesel::sql_query("INSERT INTO spaces(id, name, device, created_at) VALUES (?,?, 'local', ?)")
            .bind::<Text, _>(&space_id)
            .bind::<Text, _>(name)
            .bind::<BigInt, _>(now() as i64)
            .execute(&mut *self.connection.borrow_mut())?;
        self.record(
            project,
            "space_created",
            &actor.id,
            Some(&space_id),
            None,
            json!({"name": name}),
        )?;
        Ok(json!({
            "space_id": space_id,
            "name": name,
            "device": "local",
            "private": false,
            "members": Vec::<String>::new(),
        }))
    }

    fn space_join(&self, project: &str, actor: &Agent, space_id: &str, agent: &str) -> Result<Value> {
        self.space(space_id)?
            .ok_or_else(|| scope_denied("Space not found"))?;
        let member = self.agent(project, agent)?;
        let position = self.count(
            "SELECT COUNT(*) AS count FROM space_members WHERE space_id = ?",
            &[space_id],
        )?;
        let inserted = diesel::sql_query("INSERT INTO space_members(space_id, agent, position) VALUES (?,?,?) ON CONFLICT(space_id, agent) DO NOTHING")
            .bind::<Text, _>(space_id)
            .bind::<Text, _>(&member.id)
            .bind::<BigInt, _>(position)
            .execute(&mut *self.connection.borrow_mut())?;
        if inserted > 0 {
            self.record(
                project,
                "space_member_joined",
                &actor.id,
                Some(space_id),
                None,
                json!({"agent": member.id}),
            )?;
        }
        Ok(json!({"space_id": space_id, "agent": member.id}))
    }

    fn space_leave(&self, project: &str, actor: &Agent, space_id: &str, agent: &str) -> Result<Value> {
        self.space(space_id)?
            .ok_or_else(|| scope_denied("Space not found"))?;
        let member = self.agent(project, agent)?;
        diesel::sql_query("UPDATE remote_actor_bindings SET revoked=1,current_epoch=NULL WHERE agent=? AND space_id=?")
            .bind::<Text, _>(agent).bind::<Text, _>(space_id).execute(&mut *self.connection.borrow_mut())?;
        diesel::sql_query("UPDATE agent_workspace_bindings SET revoked=1 WHERE agent=? AND space_id=?")
            .bind::<Text, _>(&member.id).bind::<Text, _>(space_id)
            .execute(&mut *self.connection.borrow_mut())?;
        let removed = diesel::sql_query("DELETE FROM space_members WHERE space_id = ? AND agent = ?")
            .bind::<Text, _>(space_id)
            .bind::<Text, _>(&member.id)
            .execute(&mut *self.connection.borrow_mut())?;
        ensure!(
            removed > 0,
            invalid_state("Agent is not a member of this space")
        );
        self.record(
            project,
            "space_member_left",
            &actor.id,
            Some(space_id),
            None,
            json!({"agent": member.id}),
        )?;
        Ok(json!({"space_id": space_id, "agent": member.id}))
    }

    fn space(&self, space_id: &str) -> Result<Option<SpaceRow>> {
        Ok(diesel::sql_query("SELECT id, name, device FROM spaces WHERE id = ?")
            .bind::<Text, _>(space_id)
            .get_result::<SpaceRow>(&mut *self.connection.borrow_mut())
            .optional()?)
    }

    /// The private space is implicit and always listed first; members resolve through live agents.
    fn space_list(&self, _project: &str, cursor: Option<&str>, limit: Option<u32>) -> Result<Value> {
        let limit = Self::page_limit(limit)?;
        if let Some(cursor) = cursor.filter(|cursor| !cursor.is_empty()) { validate_subject(cursor)?; }
        let mut spaces = if cursor.is_none() { vec![json!({
            "id": Value::Null,
            "name": "Private",
            "device": "local",
            "private": true,
            "members": Vec::<String>::new(),
            "workspaces": [],
        })] } else { vec![] };
        let available = limit as usize - spaces.len();
        let rows = diesel::sql_query("SELECT id, name, device FROM spaces WHERE name > ? ORDER BY name LIMIT ?")
            .bind::<Text, _>(cursor.unwrap_or(""))
            .bind::<BigInt, _>(available as i64 + 1)
            .load::<SpaceRow>(&mut *self.connection.borrow_mut())?;
        let more = rows.len() > available;
        let mut next = cursor.unwrap_or("").to_owned();
        for space in rows.into_iter().take(available) {
            next = space.name.clone();
            let members: Vec<String> = diesel::sql_query("SELECT member.name AS name FROM space_members AS membership JOIN agents AS member ON member.id = membership.agent WHERE membership.space_id = ? ORDER BY membership.position")
                .bind::<Text, _>(&space.id)
                .load::<MemberRow>(&mut *self.connection.borrow_mut())?
                .into_iter()
                .filter_map(|row| row.name)
                .collect();
            let workspaces: Vec<_> = diesel::sql_query("SELECT id, space_id, root, repository_id, model, branch, base_commit FROM workspaces WHERE space_id = ? ORDER BY root")
                .bind::<Text, _>(&space.id).load::<WorkspaceRow>(&mut *self.connection.borrow_mut())?
                .into_iter().map(|row| json!({"id": row.id, "space_id": row.space_id, "root": row.root,
                    "repository_id": row.repository_id, "model": row.model, "branch": row.branch, "base_commit": row.base_commit})).collect();
            spaces.push(json!({
                "id": space.id,
                "name": space.name,
                "device": space.device,
                "private": false,
                "members": members,
                "workspaces": workspaces,
            }));
        }
        Ok(json!({"spaces": spaces, "cursor": more.then_some(next)}))
    }

    fn workspace_map(
        &self,
        project: &str,
        actor: &Agent,
        space_id: &str,
        root: &str,
        repository_id: Option<&str>,
        model: &str,
        branch: Option<&str>,
        base_commit: Option<&str>,
    ) -> Result<Value> {
        self.space(space_id)?
            .ok_or_else(|| scope_denied("Space not found"))?;
        let repository_id = repository_id
            .map(|id| Uuid::parse_str(id).map(|id| id.to_string()))
            .transpose()
            .map_err(|_| invalid_input("Repository identity must be an opaque UUID"))?;
        validate_subject(model)?;
        if let Some(branch) = branch {
            validate_subject(branch)?;
        }
        if let Some(base_commit) = base_commit {
            validate_subject(base_commit)?;
        }
        let root = crate::project_root(std::path::Path::new(root.trim()))?;
        diesel::sql_query("UPDATE agent_workspace_bindings SET revoked=1 WHERE workspace_id IN (SELECT id FROM workspaces WHERE root=? AND space_id!=?)")
            .bind::<Text, _>(&root).bind::<Text, _>(space_id)
            .execute(&mut *self.connection.borrow_mut())?;
        diesel::sql_query("INSERT INTO workspaces(id, space_id, root, repository_id, model, branch, base_commit, created_at) VALUES (?,?,?,?,?,?,?,?) ON CONFLICT(root) DO UPDATE SET space_id=excluded.space_id, repository_id=excluded.repository_id, model=excluded.model, branch=excluded.branch, base_commit=excluded.base_commit")
            .bind::<Text, _>(Uuid::new_v4().to_string())
            .bind::<Text, _>(space_id)
            .bind::<Text, _>(&root)
            .bind::<Nullable<Text>, _>(&repository_id)
            .bind::<Text, _>(model)
            .bind::<Nullable<Text>, _>(branch)
            .bind::<Nullable<Text>, _>(base_commit)
            .bind::<BigInt, _>(now() as i64)
            .execute(&mut *self.connection.borrow_mut())?;
        let workspace = diesel::sql_query("SELECT id, space_id, root, repository_id, model, branch, base_commit FROM workspaces WHERE root = ?")
            .bind::<Text, _>(&root)
            .get_result::<WorkspaceRow>(&mut *self.connection.borrow_mut())?;
        self.record(
            project,
            "workspace_mapped",
            &actor.id,
            Some(&workspace.id),
            None,
            json!({"space_id": workspace.space_id, "root": workspace.root, "repository_id": workspace.repository_id, "model": workspace.model}),
        )?;
        Ok(json!({
            "workspace_id": workspace.id,
            "space_id": workspace.space_id,
            "root": workspace.root,
            "repository_id": workspace.repository_id,
            "model": workspace.model,
            "branch": workspace.branch,
            "base_commit": workspace.base_commit,
        }))
    }

    fn evidence_reference(&self, project: &str, evidence_id: &str) -> Result<EvidenceCheckRow> {
        let row = diesel::sql_query("SELECT evidence.task_id AS task_id, evidence.kind AS kind, evidence.path AS path, evidence.hash AS hash, evidence.commit_id AS commit_id, evidence.base AS base, evidence.head AS head, evidence.device AS device, tasks.project AS project, COALESCE(w.root, producer.project) AS producing_root FROM evidence JOIN tasks ON tasks.id = evidence.task_id LEFT JOIN attempts AS attempt ON attempt.id=evidence.attempt_id LEFT JOIN agents AS producer ON producer.id=attempt.owner LEFT JOIN agent_workspace_bindings AS binding ON binding.agent=producer.id LEFT JOIN workspaces AS w ON w.id=binding.workspace_id WHERE evidence.id = ?")
            .bind::<Text, _>(evidence_id)
            .get_result::<EvidenceCheckRow>(&mut *self.connection.borrow_mut())
            .optional()?;
        row
            .filter(|row| row.project == project)
            .ok_or_else(|| scope_denied("Evidence not found in this project"))
    }

    /// Resolve only local file content from the producing checkout, with current path checks.
    pub(crate) fn local_evidence_file(&self, project: &str, evidence_id: &str) -> Result<std::path::PathBuf> {
        let row = self.evidence_reference(project, evidence_id)?;
        ensure!(row.device.is_none() && row.kind == "file", invalid_state("Evidence has no local file content"));
        let root = row.producing_root.as_deref().filter(|root| !root.starts_with("space:") && !root.starts_with("remote:"))
            .ok_or_else(|| invalid_state("Producing workspace unavailable"))?;
        let relative = row.path.as_deref().ok_or_else(|| invalid_state("Evidence path unavailable"))?;
        let normalized = crate::normalize_workspace_path(root, relative)?;
        let root = std::path::Path::new(root).canonicalize().map_err(|_| invalid_state("Producing workspace unavailable"))?;
        let file = root.join(normalized).canonicalize().map_err(|_| invalid_state("Evidence content unavailable"))?;
        ensure!(file.starts_with(&root) && file.is_file(), scope_denied("Evidence is not a file inside its producing checkout"));
        Ok(file)
    }

    /// Verification is an explicit local operator read, never a model's provenance claim.
    fn evidence_verify(
        &self,
        project: &str,
        actor: &Agent,
        evidence_id: &str,
        verified: bool,
    ) -> Result<Value> {
        let row = self.evidence_reference(project, evidence_id)?;
        if verified {
            ensure!(row.device.is_none(), invalid_state("Remote evidence cannot be verified as local content"));
            let root = row.producing_root.as_deref().filter(|root| !root.starts_with("space:"))
                .ok_or_else(|| invalid_state("Producing workspace unavailable"))?;
            match row.kind.as_str() {
                "file" | "diff" if row.path.is_some() => verify_file(root, row.path.as_deref().unwrap(), row.hash.as_deref())?,
                "commit" => verify_commit(root, row.commit_id.as_deref())?,
                "diff" => {
                    verify_commit(root, row.base.as_deref())?;
                    verify_commit(root, row.head.as_deref())?;
                }
                _ => return Err(invalid_state("Reported test outcomes are not independently verified")),
            }
        }
        diesel::sql_query("UPDATE evidence SET verified = ? WHERE id = ?")
            .bind::<Integer, _>(i32::from(verified))
            .bind::<Text, _>(evidence_id)
            .execute(&mut *self.connection.borrow_mut())?;
        self.record(
            project,
            "evidence_verified",
            &actor.id,
            Some(evidence_id),
            None,
            json!({"task_id": row.task_id, "verified": verified}),
        )?;
        Ok(json!({"evidence_id": evidence_id, "task_id": row.task_id, "verified": verified}))
    }

    /// Operator override: the only path that fences a still-active attempt without its owning session.
    fn force_cancel(
        &self,
        project: &str,
        actor: &Agent,
        task_id: &str,
        reason: &str,
        expected_version: Option<u64>,
    ) -> Result<Value> {
        text(reason)?;
        let task = self.operator_task(project, task_id)?;
        if let Some(expected) = expected_version {
            ensure!(
                expected == task.version,
                version_conflict("Task version does not match", task.version)
            );
        }
        match task.state.as_str() {
            "queued" | "blocked" | "submitted" => {
                Ok(json!(self.mark_cancelled(task, actor, reason, false)?))
            }
            "running" | "cancel_requested" => {
                let override_uncertain = self.unresolved_attempts(&task.id, None)?.len() > 0 || task.attempts.is_empty();
                Ok(json!(self.mark_cancelled(task, actor, reason, override_uncertain)?))
            }
            _ => Err(invalid_state("Task cannot be force-cancelled from its current state")),
        }
    }

    fn task_archive(&self, project: &str, actor: &Agent, task_id: &str) -> Result<Value> {
        let mut task = self.operator_task(project, task_id)?;
        ensure!(
            matches!(
                task.state.as_str(),
                "accepted" | "failed" | "expired" | "cancelled"
            ),
            invalid_state("Only terminal tasks can be archived")
        );
        ensure!(
            self.unresolved_dependents(&task.id)?.is_empty(),
            invalid_state("Other tasks still depend on this one")
        );
        ensure!(
            self.count(&format!("SELECT COUNT(*) AS count FROM tasks WHERE id = ? AND {CERTAIN_TASK}"), &[task.id.as_str()])? == 1,
            execution_unknown("Unfinished or uncertain attempts must remain in active history")
        );
        task.archived = true;
        task.version += 1;
        self.update_task_state(&task)?;
        self.record(
            project,
            "task_archived",
            &actor.id,
            Some(&task.id),
            None,
            json!({"revision": task.revision, "aged": false}),
        )?;
        Ok(json!(self.assemble(task)?))
    }

    /// Dependent tasks that are not themselves terminal keep the prerequisite unresolved.
    fn unresolved_dependents(&self, task_id: &str) -> Result<Vec<String>> {
        Ok(diesel::sql_query("SELECT dependent.id AS value FROM task_dependencies AS dependency JOIN tasks AS dependent ON dependent.id = dependency.task_id WHERE dependency.prerequisite_id = ? AND dependent.state NOT IN ('accepted','failed','expired','cancelled') ORDER BY dependent.created_seq")
            .bind::<Text, _>(task_id)
            .load::<ValueRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(|row| row.value)
            .collect())
    }

    fn archive_aged(
        &self,
        project: &str,
        actor: &Agent,
        older_than_days: Option<u64>,
    ) -> Result<Value> {
        let days = older_than_days.unwrap_or(ARCHIVE_AFTER_DAYS);
        ensure!(
            (1..=3650).contains(&days),
            invalid_input("older_than_days must be between 1 and 3650")
        );
        let cutoff = now().saturating_sub(days * 24 * 60 * 60 * 1000);
        let rows = diesel::sql_query(format!(
            "SELECT {TASK_COLUMNS} FROM tasks WHERE project = ? AND archived = 0 AND state IN ('accepted','failed','expired','cancelled') AND updated_at < ? AND {CERTAIN_TASK} ORDER BY created_seq"
        ))
        .bind::<Text, _>(project)
        .bind::<BigInt, _>(cutoff as i64)
        .load::<TaskRow>(&mut *self.connection.borrow_mut())?;
        let mut archived = Vec::new();
        for task in rows.into_iter().map(TaskRow::task) {
            if !self.unresolved_dependents(&task.id)?.is_empty() {
                continue;
            }
            let mut task = task;
            task.archived = true;
            task.version += 1;
            self.update_task_state(&task)?;
            self.record(
                project,
                "task_archived",
                &actor.id,
                Some(&task.id),
                None,
                json!({"revision": task.revision, "aged": true}),
            )?;
            archived.push(task.id);
        }
        Ok(json!({"archived": archived}))
    }

    fn history_sequence(&self, project: &str) -> Result<u64> {
        Ok(self.count("SELECT COALESCE(MAX(value), 0) AS count FROM sequences WHERE project = ?", &[project])? as u64)
    }

    fn purge_preview(&self, project: &str) -> Result<Value> {
        let tasks = self.count(
            &format!("SELECT COUNT(*) AS count FROM tasks WHERE project = ? AND {PURGEABLE_TASK} AND {CERTAIN_TASK}"),
            &[project],
        )?;
        let messages = self.count(
            &format!("SELECT COUNT(*) AS count FROM messages WHERE project = ? AND {PURGEABLE_MESSAGE}"),
            &[project],
        )?;
        Ok(json!({"tasks": tasks, "messages": messages, "sequence": self.history_sequence(project)?}))
    }

    /// Only archived, unreferenced work and read messages; active or uncertain records are never deleted.
    fn history_purge(
        &self,
        project: &str,
        actor: &Agent,
        archived_tasks: bool,
        acknowledged_messages: bool,
    ) -> Result<Value> {
        ensure!(
            archived_tasks || acknowledged_messages,
            invalid_input("Select archived tasks, acknowledged messages, or both")
        );
        // Select both sets before either deletion can make additional rows eligible.
        let rows = if archived_tasks {
            diesel::sql_query(format!(
                "SELECT {TASK_COLUMNS} FROM tasks WHERE project = ? AND {PURGEABLE_TASK} AND {CERTAIN_TASK} ORDER BY created_seq"
            ))
            .bind::<Text, _>(project)
            .load::<TaskRow>(&mut *self.connection.borrow_mut())?
        } else {
            Vec::new()
        };
        let purged_messages = if acknowledged_messages {
            diesel::sql_query(format!("DELETE FROM messages WHERE id IN (SELECT id FROM messages WHERE project = ? AND {PURGEABLE_MESSAGE})"))
                .bind::<Text, _>(project)
                .execute(&mut *self.connection.borrow_mut())? as u64
        } else {
            0
        };
        let mut purged_tasks = Vec::new();
        for task in rows.into_iter().map(TaskRow::task) {
            for statement in [
                "DELETE FROM messages WHERE task_id = ?",
                "DELETE FROM task_dependencies WHERE task_id = ?",
                "DELETE FROM task_eligibles WHERE task_id = ?",
                "DELETE FROM attempts WHERE task_id = ?",
                "DELETE FROM feedback WHERE task_id = ?",
                "DELETE FROM evidence WHERE task_id = ?",
            ] {
                diesel::sql_query(statement)
                    .bind::<Text, _>(&task.id)
                    .execute(&mut *self.connection.borrow_mut())?;
            }
            diesel::sql_query("DELETE FROM tasks WHERE id = ?")
                .bind::<Text, _>(&task.id)
                .execute(&mut *self.connection.borrow_mut())?;
            self.record(
                project,
                "task_purged",
                &actor.id,
                Some(&task.id),
                None,
                json!({"revision": task.revision, "state": task.state}),
            )?;
            purged_tasks.push(task.id);
        }
        self.record(
            project,
            "history_purged",
            &actor.id,
            None,
            None,
            json!({"tasks": purged_tasks.len(), "messages": purged_messages}),
        )?;
        Ok(json!({"purged_tasks": purged_tasks, "purged_messages": purged_messages}))
    }
    /// One merged ordered stream: tasks, messages and events share the per-project sequence counter.
    fn history_export(&self, project: &str, after: Option<u64>, limit: Option<u32>) -> Result<Value> {
        let limit = Self::page_limit(limit)?;
        let after = after.unwrap_or(0) as i64;
        let fetch = limit as i64 + 1;
        let mut records: Vec<(u64, &'static str, Value)> = Vec::new();
        let tasks = diesel::sql_query(format!(
            "SELECT {TASK_COLUMNS} FROM tasks WHERE project = ? AND created_seq > ? ORDER BY created_seq LIMIT ?"
        ))
        .bind::<Text, _>(project)
        .bind::<BigInt, _>(after)
        .bind::<BigInt, _>(fetch)
        .load::<TaskRow>(&mut *self.connection.borrow_mut())?;
        for task in tasks.into_iter().map(TaskRow::task) {
            let sequence = task.created_seq;
            records.push((sequence, "task", serde_json::to_value(self.assemble(task)?)?));
        }
        for message in diesel::sql_query("SELECT id, sender, recipient, body, subject, thread_id, reply_to, task_id, revision, kind, acknowledged, sequence FROM messages WHERE project = ? AND sequence > ? ORDER BY sequence LIMIT ?")
            .bind::<Text, _>(project)
            .bind::<BigInt, _>(after)
            .bind::<BigInt, _>(fetch)
            .load::<MessageRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(MessageRow::message)
        {
            let sequence = message.sequence;
            records.push((sequence, "message", serde_json::to_value(message)?));
        }
        for event in diesel::sql_query("SELECT id, project, sequence, kind, actor, resource, attempt, observed_at, imported, payload FROM events WHERE project = ? AND sequence > ? ORDER BY sequence LIMIT ?")
            .bind::<Text, _>(project)
            .bind::<BigInt, _>(after)
            .bind::<BigInt, _>(fetch)
            .load::<EventRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(EventRow::event)
        {
            let sequence = event.sequence;
            records.push((sequence, "event", serde_json::to_value(event)?));
        }
        records.sort_by_key(|(sequence, kind, _)| (*sequence, *kind));
        let (records, cursor) = bounded_items(records.into_iter()
            .map(|(sequence, kind, data)| (sequence, json!({"type": kind, "sequence": sequence, "data": data}))), limit)?;
        Ok(json!({"records": records, "cursor": cursor}))
    }

    /// Thread history: the root message plus replies, visible only to thread participants.
    fn thread_get(
        &self,
        actor: &Agent,
        thread_id: &str,
        cursor: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Value> {
        let limit = Self::page_limit(limit)?;
        let total = self.count(
            "SELECT COUNT(*) AS count FROM messages WHERE project = ? AND (thread_id = ? OR id = ?)",
            &[actor.project.as_str(), thread_id, thread_id],
        )?;
        if total > 0 && actor.program != OPERATOR_PROGRAM {
            let participant = diesel::sql_query("SELECT COUNT(*) AS count FROM messages WHERE project = ? AND (thread_id = ? OR id = ?) AND (sender = ? OR recipient = ?)")
                .bind::<Text, _>(&actor.project)
                .bind::<Text, _>(thread_id)
                .bind::<Text, _>(thread_id)
                .bind::<Text, _>(&actor.id)
                .bind::<Text, _>(&actor.id)
                .get_result::<CountRow>(&mut *self.connection.borrow_mut())?
                .count;
            ensure!(participant > 0, scope_denied("Thread not found"));
        }
        let rows = diesel::sql_query("SELECT id, sender, recipient, body, subject, thread_id, reply_to, task_id, revision, kind, acknowledged, sequence FROM messages WHERE project = ? AND (thread_id = ? OR id = ?) AND sequence > ? ORDER BY sequence LIMIT ?")
            .bind::<Text, _>(&actor.project)
            .bind::<Text, _>(thread_id)
            .bind::<Text, _>(thread_id)
            .bind::<BigInt, _>(cursor.unwrap_or(0) as i64)
            .bind::<BigInt, _>(limit as i64 + 1)
            .load::<MessageRow>(&mut *self.connection.borrow_mut())?;
        let (messages, cursor) = bounded_items(rows.into_iter().map(MessageRow::message)
            .map(|message| (message.sequence, message)), limit)?;
        Ok(json!({"messages": messages, "cursor": cursor}))
    }

    /// Literal substring search over caller-visible records; LIKE metacharacters are escaped.
    fn message_search(
        &self,
        actor: &Agent,
        query: &str,
        task_id: Option<&str>,
        thread_id: Option<&str>,
        cursor: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Value> {
        let limit = Self::page_limit(limit)?;
        let query = query.trim();
        search_query(query)?;
        let escaped = query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        let rows = diesel::sql_query("SELECT id, sender, recipient, body, subject, thread_id, reply_to, task_id, revision, kind, acknowledged, sequence FROM messages WHERE project = ? AND (sender = ? OR recipient = ? OR ?) AND (body LIKE ? ESCAPE '\\' OR subject LIKE ? ESCAPE '\\') AND (COALESCE(?, '') = '' OR task_id = ?) AND (COALESCE(?, '') = '' OR thread_id = ?) AND sequence > ? ORDER BY sequence LIMIT ?")
            .bind::<Text, _>(&actor.project)
            .bind::<Text, _>(&actor.id)
            .bind::<Text, _>(&actor.id)
            .bind::<Integer, _>(i32::from(actor.program == OPERATOR_PROGRAM))
            .bind::<Text, _>(&pattern)
            .bind::<Text, _>(&pattern)
            .bind::<Text, _>(task_id.unwrap_or(""))
            .bind::<Nullable<Text>, _>(task_id)
            .bind::<Text, _>(thread_id.unwrap_or(""))
            .bind::<Nullable<Text>, _>(thread_id)
            .bind::<BigInt, _>(cursor.unwrap_or(0) as i64)
            .bind::<BigInt, _>(limit as i64 + 1)
            .load::<MessageRow>(&mut *self.connection.borrow_mut())?;
        let (messages, cursor) = bounded_items(rows.into_iter().map(MessageRow::message)
            .map(|message| (message.sequence, message)), limit)?;
        Ok(json!({"messages": messages, "cursor": cursor}))
    }

    /// Cross-checkout overlap is advisory and visible only through explicit shared membership.
    fn reservation_overlaps(
        &self,
        actor: &Agent,
        paths: &[String],
        mode: &str,
        now_ms: u64,
    ) -> Result<(Vec<Value>, bool)> {
        let mut warnings = Vec::new();
        let mut seen = HashSet::new();
        for path in paths {
            let rows = diesel::sql_query(
                "SELECT r.id AS reservation_id, other.id AS workspace_id, r.path, r.owner, r.task_id, r.expires_at
                 FROM workspaces AS source
                 JOIN space_members AS self_member ON self_member.space_id = source.space_id AND self_member.agent = ?
                 JOIN workspaces AS other ON other.space_id = source.space_id AND other.repository_id = source.repository_id AND other.root != source.root
                 JOIN reservations AS r ON r.workspace = other.root AND r.workspace_id = other.id AND r.space_id = source.space_id AND r.repository_id = source.repository_id
                 JOIN space_members AS owner_member ON owner_member.space_id = source.space_id AND owner_member.agent = r.owner
                 WHERE source.root = ? AND r.expires_at > ? AND (? = 'exclusive' OR r.mode = 'exclusive')
                   AND (r.path = ? OR substr(r.path, 1, length(?) + 1) = ? || '/' OR substr(?, 1, length(r.path) + 1) = r.path || '/')
                 ORDER BY r.workspace, r.created_seq, r.id LIMIT ?",
            )
            .bind::<Text, _>(&actor.id)
            .bind::<Text, _>(self.physical_root(actor)?)
            .bind::<BigInt, _>(now_ms as i64)
            .bind::<Text, _>(mode)
            .bind::<Text, _>(path)
            .bind::<Text, _>(path)
            .bind::<Text, _>(path)
            .bind::<Text, _>(path)
            .bind::<BigInt, _>((PAGE_DEFAULT + 1) as i64)
            .load::<ReservationOverlapRow>(&mut *self.connection.borrow_mut())?;
            for row in rows {
                if !seen.insert(row.reservation_id.clone()) {
                    continue;
                }
                if warnings.len() == PAGE_DEFAULT as usize {
                    return Ok((warnings, true));
                }
                warnings.push(
                    json!({"kind": "merge_overlap", "reservation_id": row.reservation_id,
                    "workspace_id": row.workspace_id, "path": row.path, "owner": self.agent(&actor.project, &row.owner).ok().map(|owner| owner.id),
                    "task_id": row.task_id.filter(|id| self.operator_task(&actor.project, id).is_ok()), "expires_at": row.expires_at}),
                );
            }
        }
        Ok((warnings, false))
    }

    fn file_reservations(
        &self,
        actor: &Agent,
        path: Option<&str>,
        cursor: Option<u64>,
        limit: Option<u32>,
        include_expired: bool,
    ) -> Result<Value> {
        let workspace = self.physical_root(actor)?;
        let filter = path.map(|path| self.reservation_path(actor, &workspace, path)).transpose()?;
        self.operator_reservations(&actor.project, &workspace, filter.as_deref(), cursor, limit, include_expired)
    }

    /// Trusted panel reads use the same checkout and scope filters as native leases.
    pub(crate) fn operator_reservations(
        &self, project: &str, workspace: &str, filter: Option<&str>,
        cursor: Option<u64>, limit: Option<u32>, include_expired: bool,
    ) -> Result<Value> {
        let limit = Self::page_limit(limit)?;
        let now_ms = now();
        let rows = diesel::sql_query("SELECT id, workspace, path, mode, owner, task_id, attempt_id, created_at, expires_at, created_seq FROM reservations WHERE workspace = ? AND owner IN (SELECT id FROM agents WHERE project = ?) AND created_seq > ? AND (expires_at > ? OR ?) ORDER BY created_seq LIMIT ?")
            .bind::<Text, _>(&workspace)
            .bind::<Text, _>(project)
            .bind::<BigInt, _>(i64::try_from(cursor.unwrap_or(0)).unwrap_or(i64::MAX))
            .bind::<BigInt, _>(now_ms as i64)
            .bind::<Integer, _>(i32::from(include_expired))
            // Filtered native reads retain complete overlap matching; the common
            // unfiltered panel page is bounded directly by the indexed SQL query.
            .bind::<BigInt, _>(if filter.is_some() { i64::MAX } else { limit as i64 + 1 })
            .load::<ReservationRow>(&mut *self.connection.borrow_mut())?;
        let mut reservations = Vec::new();
        for row in rows {
            if cursor.is_some_and(|cursor| row.created_seq as u64 <= cursor) {
                continue;
            }
            let reservation = self.reservation(row, now_ms)?;
            if !include_expired && reservation.expired {
                continue;
            }
            if filter.is_some_and(|filter| !paths_overlap(&reservation.path, filter))
            {
                continue;
            }
            reservations.push(reservation);
            if reservations.len() as u32 > limit {
                break;
            }
        }
        reservations.truncate(limit as usize);
        let cursor = reservations.last().map(|reservation| reservation.created_seq);
        Ok(json!({"reservations": reservations, "cursor": cursor}))
    }

    /// Lease expiry and the abandoned-owner warning resolve at read time.
    fn reservation(&self, row: ReservationRow, now: u64) -> Result<Reservation> {
        let expired = row.expires_at as u64 <= now;
        let abandoned = if !expired {
            false
        } else {
            match &row.attempt_id {
                Some(attempt_id) => self.attempt_certainty(attempt_id, &row.owner)?.map_or(
                    true,
                    |certainty| certainty != "finished" && certainty != "interrupted",
                ),
                None => false,
            }
        };
        Ok(Reservation {
            id: row.id,
            workspace: row.workspace,
            path: row.path,
            mode: row.mode,
            owner: row.owner,
            task_id: row.task_id,
            attempt_id: row.attempt_id,
            created_at: row.created_at as u64,
            created_seq: row.created_seq as u64,
            expires_at: row.expires_at as u64,
            expired,
            abandoned,
        })
    }

    fn reservation_row(&self, workspace: &str, id: &str) -> Result<ReservationRow> {
        diesel::sql_query("SELECT id, workspace, path, mode, owner, task_id, attempt_id, created_at, expires_at, created_seq FROM reservations WHERE id = ? AND workspace = ?")
            .bind::<Text, _>(id)
            .bind::<Text, _>(workspace)
            .get_result::<ReservationRow>(&mut *self.connection.borrow_mut())
            .optional()?
            .ok_or_else(|| scope_denied("Reservation not found in this workspace"))
    }

    fn reservation_by_id(&self, id: &str) -> Result<Option<ReservationRow>> {
        Ok(diesel::sql_query("SELECT id, workspace, path, mode, owner, task_id, attempt_id, created_at, expires_at, created_seq FROM reservations WHERE id = ?")
            .bind::<Text, _>(id)
            .get_result::<ReservationRow>(&mut *self.connection.borrow_mut())
            .optional()?)
    }
}

/// Inspect the opened handle before reading, so a renamed parent cannot redirect verification.
fn opened_file_path(file: &std::fs::File) -> Result<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        use std::os::{fd::AsRawFd, unix::ffi::OsStringExt};
        let mut path = vec![0u8; libc::PATH_MAX as usize];
        // F_GETPATH writes at most PATH_MAX bytes into this owned, initialized buffer.
        ensure!(unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, path.as_mut_ptr()) } != -1,
            invalid_state("Cannot inspect the opened evidence file"));
        let length = path.iter().position(|byte| *byte == 0)
            .ok_or_else(|| invalid_state("Evidence path is too long"))?;
        path.truncate(length);
        Ok(std::ffi::OsString::from_vec(path).into())
    }
    #[cfg(windows)]
    {
        use std::os::windows::{ffi::OsStringExt, io::AsRawHandle};
        use windows::Win32::{Foundation::HANDLE, Storage::FileSystem::{GetFinalPathNameByHandleW, FILE_NAME_NORMALIZED}};
        let mut path = vec![0u16; 32768];
        // The borrowed handle stays open and the API receives the full slice length.
        let length = unsafe { GetFinalPathNameByHandleW(HANDLE(file.as_raw_handle()), &mut path, FILE_NAME_NORMALIZED) } as usize;
        ensure!(length > 0 && length < path.len(), invalid_state("Cannot inspect the opened evidence file"));
        Ok(std::ffi::OsString::from_wide(&path[..length]).into())
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = file;
        Err(invalid_state("Local evidence verification supports macOS and Windows"))
    }
}

fn evidence_path_allowed(path: &std::path::Path) -> bool {
    path.components().all(|component| {
        let value = component.as_os_str().to_string_lossy().to_ascii_lowercase();
        !matches!(value.as_str(), ".env" | ".git" | ".ssh" | ".aws" | ".npmrc" | ".netrc" | "credentials" | "id_rsa" | "id_ed25519")
            && !value.starts_with(".env.")
            && ![".pem", ".p12", ".pfx", ".key"].iter().any(|suffix| value.ends_with(*suffix))
    })
}

fn verify_file(project: &str, path: &str, hash: Option<&str>) -> Result<()> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let hash = hash.map(|hash| hash.strip_prefix("sha256:").unwrap_or(hash))
        .ok_or_else(|| invalid_state("File verification requires a SHA-256 hash"))?;
    ensure!(hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
        invalid_input("File verification requires a 64-digit SHA-256 hash"));
    ensure!(evidence_path_allowed(std::path::Path::new(path)), invalid_input("Credential paths cannot be verified as evidence"));
    let relative = crate::normalize_workspace_path(project, path)?;
    let root = std::path::Path::new(project).canonicalize()
        .map_err(|_| invalid_state("Producing workspace is unavailable"))?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(root.join(relative)).map_err(|_| invalid_state("Evidence file is unavailable"))?;
    let opened = opened_file_path(&file)?;
    let relative = opened.strip_prefix(&root).map_err(|_| invalid_input("Opened evidence escapes the workspace"))?;
    ensure!(evidence_path_allowed(relative), invalid_input("Credential paths cannot be verified as evidence"));
    let metadata = file.metadata().map_err(|_| invalid_state("Evidence file cannot be inspected"))?;
    const LIMIT: u64 = 64 * 1024 * 1024;
    ensure!(metadata.is_file() && metadata.len() <= LIMIT, invalid_input("Evidence must be a regular file at most 64 MiB"));
    let mut file = file.take(LIMIT + 1);
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 8192];
    let mut bytes = 0u64;
    loop {
        let count = file.read(&mut buffer).map_err(|_| invalid_state("Evidence file cannot be read"))?;
        if count == 0 { break; }
        bytes += count as u64;
        ensure!(bytes <= LIMIT, invalid_input("Evidence file exceeds 64 MiB"));
        digest.update(&buffer[..count]);
    }
    ensure!(format!("{:x}", digest.finalize()).eq_ignore_ascii_case(hash), invalid_state("Evidence content hash does not match"));
    Ok(())
}

fn verify_commit(project: &str, commit: Option<&str>) -> Result<()> {
    use std::{process::{Command, Stdio}, time::Instant};
    let commit = commit.ok_or_else(|| invalid_state("Commit verification requires a Git object ID"))?;
    ensure!(matches!(commit.len(), 40 | 64) && commit.bytes().all(|byte| byte.is_ascii_hexdigit()),
        invalid_input("Commit verification requires a full hexadecimal Git object ID"));
    let mut child = Command::new("git").args(["--no-lazy-fetch", "--no-optional-locks", "-C", project, "cat-file", "-e"])
        .arg(format!("{commit}^{{commit}}"))
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map_err(|_| invalid_state("Git is unavailable for local object verification"))?;
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                ensure!(status.success(), invalid_state("Commit is unavailable locally or Git lacks no-fetch support"));
                return Ok(());
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(invalid_state("Local Git object verification did not finish"));
            }
        }
    }
}

/// Reserve space for the outer MCP string envelope as well as the local IPC envelope.
fn bounded_items<T: serde::Serialize>(items: impl Iterator<Item = (u64, T)>, limit: u32) -> Result<(Vec<T>, Option<u64>)> {
    let mut page = Vec::new();
    let mut cursor = None;
    let mut bytes = 0;
    let budget = crate::MAX_FRAME / 2 - 4096;
    for (sequence, item) in items.take(limit as usize) {
        let size = serde_json::to_vec(&item)?.len() + 1;
        ensure!(size <= budget, crate::domain("frame_too_large", "A history record exceeds the readable frame budget; use a local export/restore procedure", false, None));
        if bytes + size > budget { break; }
        bytes += size;
        cursor = Some(sequence);
        page.push(item);
    }
    Ok((page, cursor))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn task_summaries(rows: Vec<TaskSummaryRow>, limit: u32) -> Value {
    let tasks: Vec<Value> = rows
        .into_iter()
        .take(limit as usize)
        .map(|row| {
            json!({
                "id": row.id,
                "state": row.state,
                "revision": row.revision as u32,
                "version": row.version as u64,
                "assignee": row.assignee,
                "reviewer": row.reviewer,
                "created_seq": row.created_seq as u64,
            })
        })
        .collect();
    let cursor = tasks
        .last()
        .and_then(|task| task["created_seq"].as_u64())
        .map(|sequence| json!(sequence))
        .unwrap_or(Value::Null);
    json!({"tasks": tasks, "cursor": cursor})
}

fn task_message(
    from: &str,
    to: &str,
    kind: &str,
    body: String,
    task_id: &str,
    revision: u32,
) -> Message {
    Message {
        id: String::new(),
        from: from.into(),
        to: to.into(),
        body,
        subject: None,
        thread_id: None,
        reply_to: None,
        task_id: Some(task_id.into()),
        revision: Some(revision),
        kind: kind.into(),
        acknowledged: false,
        sequence: 0,
    }
}

/// Two normalized workspace-relative paths overlap when equal or one is an ancestor of the other.
fn paths_overlap(a: &str, b: &str) -> bool {
    a == b
        || a.strip_prefix(b).is_some_and(|rest| rest.starts_with('/'))
        || b.strip_prefix(a).is_some_and(|rest| rest.starts_with('/'))
}

fn pending_deadline(value: Option<&str>) -> Result<Option<u64>> {
    value.map(future_deadline).transpose()
}

fn future_deadline(value: &str) -> Result<u64> {
    let deadline = parse_deadline(value)?;
    ensure!(
        deadline > now(),
        invalid_input("Start deadline must be in the future")
    );
    Ok(deadline)
}

/// Retry/reassign deadline policy: explicit replacement, explicit clear, or only an unmatured deadline is reused.
fn replace_start_deadline(task: &mut Task, replacement: Option<&str>, clear: bool) -> Result<()> {
    ensure!(
        !(clear && replacement.is_some()),
        invalid_input("Choose either a new start deadline or an explicit clear")
    );
    if let Some(value) = replacement {
        task.start_deadline = Some(future_deadline(value)?);
        return Ok(());
    }
    if clear {
        task.start_deadline = None;
        return Ok(());
    }
    ensure!(
        !task
            .start_deadline
            .is_some_and(|deadline| deadline <= now()),
        invalid_input("The start deadline has passed; replace or clear it to retry")
    );
    Ok(())
}

fn read_legacy_payload(path: &str) -> Result<Option<String>> {
    if !std::path::Path::new(path).exists() {
        return Ok(None);
    }
    let mut connection = SqliteConnection::establish(path)?;
    let tables = diesel::sql_query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='agent_bus_v1'",
    )
    .load::<NameRow>(&mut connection)?;
    if tables.is_empty() {
        return Ok(None);
    }
    Ok(
        diesel::sql_query("SELECT payload AS value FROM agent_bus_v1 WHERE id=1")
            .get_result::<ValueRow>(&mut connection)
            .optional()?
            .map(|row| row.value),
    )
}

#[derive(QueryableByName)]
struct NameRow {
    #[diesel(sql_type = Text)]
    #[allow(dead_code)]
    name: String,
}

#[derive(QueryableByName)]
struct ValueRow {
    #[diesel(sql_type = Text)]
    value: String,
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct BytesRow {
    #[diesel(sql_type = BigInt)]
    value: i64,
}

#[derive(QueryableByName)]
struct SequenceRow {
    #[diesel(sql_type = BigInt)]
    value: i64,
}

#[derive(QueryableByName)]
struct AgentRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    terminal: String,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    program: String,
    #[diesel(sql_type = Text)]
    project: String,
}

impl AgentRow {
    fn agent(self) -> Agent {
        Agent {
            id: self.id,
            terminal: self.terminal,
            name: self.name,
            program: self.program,
            project: self.project,
        }
    }
}

#[derive(QueryableByName)]
struct TaskRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    project: String,
    #[diesel(sql_type = Text)]
    issuer: String,
    #[diesel(sql_type = Text)]
    assignee: String,
    #[diesel(sql_type = Text)]
    reviewer: String,
    #[diesel(sql_type = Text)]
    description: String,
    #[diesel(sql_type = Text)]
    acceptance: String,
    #[diesel(sql_type = Text)]
    state: String,
    #[diesel(sql_type = Integer)]
    revision: i32,
    #[diesel(sql_type = BigInt)]
    version: i64,
    #[diesel(sql_type = Nullable<Text>)]
    result: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    evidence: Option<String>,
    #[diesel(sql_type = BigInt)]
    created_seq: i64,
    #[diesel(sql_type = Integer)]
    archived: i32,
    #[diesel(sql_type = Nullable<BigInt>)]
    start_deadline: Option<i64>,
    #[diesel(sql_type = Nullable<BigInt>)]
    execution_timeout: Option<i64>,
    #[diesel(sql_type = Nullable<BigInt>)]
    review_timeout: Option<i64>,
    #[diesel(sql_type = Nullable<BigInt>)]
    execution_deadline: Option<i64>,
    #[diesel(sql_type = Nullable<BigInt>)]
    review_deadline: Option<i64>,
}

impl TaskRow {
    fn task(self) -> Task {
        Task {
            id: self.id,
            project: self.project,
            issuer: self.issuer,
            assignee: self.assignee,
            reviewer: self.reviewer,
            description: self.description,
            acceptance: self.acceptance,
            state: self.state,
            revision: self.revision as u32,
            version: self.version as u64,
            result: self.result,
            evidence: self.evidence,
            feedback: vec![],
            history_truncated: false,
            attempts: vec![],
            evidence_records: vec![],
            created_seq: self.created_seq as u64,
            archived: self.archived != 0,
            dependencies: vec![],
            eligible: vec![],
            wait_reason: None,
            start_deadline: self.start_deadline.map(|value| value as u64),
            execution_timeout_seconds: self.execution_timeout.map(|value| value as u64),
            review_timeout_seconds: self.review_timeout.map(|value| value as u64),
            execution_deadline: self.execution_deadline.map(|value| value as u64),
            review_deadline: self.review_deadline.map(|value| value as u64),
            review_overdue: false,
            executing_run: None,
        }
    }
}

#[derive(QueryableByName)]
struct SummaryRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    state: String,
    #[diesel(sql_type = Integer)]
    revision: i32,
    #[diesel(sql_type = BigInt)]
    version: i64,
}

#[derive(QueryableByName)]
struct TaskSummaryRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    state: String,
    #[diesel(sql_type = Integer)]
    revision: i32,
    #[diesel(sql_type = BigInt)]
    version: i64,
    #[diesel(sql_type = Text)]
    assignee: String,
    #[diesel(sql_type = Text)]
    reviewer: String,
    #[diesel(sql_type = BigInt)]
    created_seq: i64,
}

#[derive(QueryableByName)]
struct MessageRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    sender: String,
    #[diesel(sql_type = Text)]
    recipient: String,
    #[diesel(sql_type = Text)]
    body: String,
    #[diesel(sql_type = Nullable<Text>)]
    subject: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    thread_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    reply_to: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    task_id: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    revision: Option<i32>,
    #[diesel(sql_type = Text)]
    kind: String,
    #[diesel(sql_type = Integer)]
    acknowledged: i32,
    #[diesel(sql_type = BigInt)]
    sequence: i64,
}

impl MessageRow {
    fn message(self) -> Message {
        Message {
            id: self.id,
            from: self.sender,
            to: self.recipient,
            body: self.body,
            subject: self.subject,
            thread_id: self.thread_id,
            reply_to: self.reply_to,
            task_id: self.task_id,
            revision: self.revision.map(|revision| revision as u32),
            kind: self.kind,
            acknowledged: self.acknowledged != 0,
            sequence: self.sequence as u64,
        }
    }
}

#[derive(QueryableByName)]
struct AttemptRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    task_id: String,
    #[diesel(sql_type = Integer)]
    revision: i32,
    #[diesel(sql_type = Text)]
    owner: String,
    #[diesel(sql_type = Text)]
    run: String,
    #[diesel(sql_type = Text)]
    certainty: String,
    #[diesel(sql_type = Nullable<Text>)]
    outcome: Option<String>,
    #[diesel(sql_type = Nullable<BigInt>)]
    started_at: Option<i64>,
    #[diesel(sql_type = Nullable<BigInt>)]
    finished_at: Option<i64>,
}

impl AttemptRow {
    fn attempt(self) -> Attempt {
        Attempt {
            id: self.id,
            task_id: self.task_id,
            revision: self.revision as u32,
            owner: self.owner,
            run: self.run,
            certainty: self.certainty,
            outcome: self.outcome,
            started_at: self.started_at.map(|at| at as u64),
            finished_at: self.finished_at.map(|at| at as u64),
        }
    }
}

#[derive(QueryableByName)]
struct EvidenceRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    task_id: String,
    #[diesel(sql_type = Nullable<Text>)]
    attempt_id: Option<String>,
    #[diesel(sql_type = Text)]
    kind: String,
    #[diesel(sql_type = Nullable<Text>)]
    path: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    hash: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    commit_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    repository: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    branch: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    base: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    head: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    command: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    outcome: Option<String>,
    #[diesel(sql_type = Nullable<BigInt>)]
    exit_code: Option<i64>,
    #[diesel(sql_type = Nullable<Text>)]
    summary: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    device: Option<String>,
    #[diesel(sql_type = Integer)]
    verified: i32,
    #[diesel(sql_type = BigInt)]
    created_seq: i64,
}

impl EvidenceRow {
    fn evidence(self) -> Evidence {
        Evidence {
            id: self.id,
            task_id: self.task_id,
            attempt_id: self.attempt_id,
            kind: self.kind,
            path: self.path,
            hash: self.hash,
            commit: self.commit_id,
            repository: self.repository,
            branch: self.branch,
            base: self.base,
            head: self.head,
            command: self.command,
            outcome: self.outcome,
            exit_code: self.exit_code,
            summary: self.summary,
            device: self.device,
            verified: self.verified != 0,
            created_seq: self.created_seq as u64,
        }
    }
}

#[derive(QueryableByName)]
struct ReservationRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    workspace: String,
    #[diesel(sql_type = Text)]
    path: String,
    #[diesel(sql_type = Text)]
    mode: String,
    #[diesel(sql_type = Text)]
    owner: String,
    #[diesel(sql_type = Nullable<Text>)]
    task_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    attempt_id: Option<String>,
    #[diesel(sql_type = BigInt)]
    created_at: i64,
    #[diesel(sql_type = BigInt)]
    expires_at: i64,
    #[diesel(sql_type = BigInt)]
    created_seq: i64,
}

#[derive(QueryableByName)]
struct ReservationOverlapRow {
    #[diesel(sql_type = Text)]
    reservation_id: String,
    #[diesel(sql_type = Text)]
    workspace_id: String,
    #[diesel(sql_type = Text)]
    path: String,
    #[diesel(sql_type = Text)]
    owner: String,
    #[diesel(sql_type = Nullable<Text>)]
    task_id: Option<String>,
    #[diesel(sql_type = BigInt)]
    expires_at: i64,
}

#[derive(QueryableByName)]
struct EvidenceCheckRow {
    #[diesel(sql_type = Nullable<Text>)]
    producing_root: Option<String>,
    #[diesel(sql_type = Text)]
    task_id: String,
    #[diesel(sql_type = Text)]
    kind: String,
    #[diesel(sql_type = Nullable<Text>)]
    path: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    hash: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    commit_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    base: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    head: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    device: Option<String>,
    #[diesel(sql_type = Text)]
    project: String,
}

#[derive(QueryableByName)]
struct SpaceRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    device: String,
}

#[derive(QueryableByName)]
struct MemberRow {
    #[diesel(sql_type = Nullable<Text>)]
    name: Option<String>,
}

#[derive(QueryableByName)]
struct WorkspaceRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    space_id: String,
    #[diesel(sql_type = Text)]
    root: String,
    #[diesel(sql_type = Nullable<Text>)]
    repository_id: Option<String>,
    #[diesel(sql_type = Text)]
    model: String,
    #[diesel(sql_type = Nullable<Text>)]
    branch: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    base_commit: Option<String>,
}

#[derive(QueryableByName)]
struct FeedbackRow {
    #[diesel(sql_type = Text)]
    body: String,
}

#[derive(QueryableByName)]
struct EventRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    project: String,
    #[diesel(sql_type = BigInt)]
    sequence: i64,
    #[diesel(sql_type = Text)]
    kind: String,
    #[diesel(sql_type = Text)]
    actor: String,
    #[diesel(sql_type = Nullable<Text>)]
    resource: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    attempt: Option<String>,
    #[diesel(sql_type = Nullable<BigInt>)]
    observed_at: Option<i64>,
    #[diesel(sql_type = Integer)]
    imported: i32,
    #[diesel(sql_type = Text)]
    payload: String,
}

impl EventRow {
    fn event(self) -> Event {
        Event {
            id: self.id,
            project: self.project,
            sequence: self.sequence as u64,
            kind: self.kind,
            actor: self.actor,
            resource: self.resource,
            attempt: self.attempt,
            observed_at: self.observed_at.map(|at| at as u64),
            imported: self.imported != 0,
            payload: serde_json::from_str(&self.payload).unwrap_or(Value::Null),
        }
    }
}

#[derive(QueryableByName)]
struct RequestRow {
    #[diesel(sql_type = Text)]
    epoch: String,
    #[diesel(sql_type = Text)]
    fingerprint: String,
    #[diesel(sql_type = Text)]
    response: String,
}

#[derive(Deserialize)]
struct LegacySnapshot {
    agents: BTreeMap<String, LegacyAgent>,
    messages: Vec<LegacyMessage>,
    tasks: BTreeMap<String, LegacyTask>,
    retries: BTreeMap<String, LegacyRetry>,
    #[allow(dead_code)]
    sequence: u64,
}

#[derive(Deserialize)]
struct LegacyAgent {
    id: String,
    terminal: String,
    name: String,
    program: String,
    project: String,
}

#[derive(Deserialize)]
struct LegacyMessage {
    id: String,
    from: String,
    to: String,
    body: String,
    task_id: Option<String>,
    revision: Option<u32>,
    kind: String,
    acknowledged: bool,
    sequence: u64,
}

#[derive(Deserialize)]
struct LegacyTask {
    id: String,
    project: String,
    issuer: String,
    assignee: String,
    reviewer: String,
    description: String,
    acceptance: String,
    state: String,
    revision: u32,
    result: Option<String>,
    evidence: Option<String>,
    feedback: Vec<String>,
}

#[derive(Deserialize)]
struct LegacyRetry {
    operation: String,
    result: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DomainError, MUTATION_EPOCH};

    fn code(error: &anyhow::Error) -> &str {
        error
            .downcast_ref::<DomainError>()
            .unwrap_or_else(|| panic!("expected a domain error: {error}"))
            .code
            .as_str()
    }

    fn actor(store: &Store, name: &str) -> Agent {
        store
            .register(&format!("terminal-{name}"), "claude", "/project", name)
            .unwrap()
    }

    fn send(_from: &Agent, to: &str, body: &str, request_id: &str) -> Operation {
        Operation::AgentSend {
            to: to.into(),
            body: body.into(),
            subject: None,
            thread_id: None,
            reply_to: None,
            task_id: None,
            request_id: request_id.into(),
        }
    }

    fn assign(_from: &Agent, to: &str, reviewer: Option<&str>, request_id: &str) -> Operation {
        Operation::TaskAssign {
            to: to.into(),
            description: "work".into(),
            acceptance: "evidence".into(),
            reviewer: reviewer.map(str::to_owned),
            dependencies: vec![],
            start_deadline: None,
            execution_timeout_seconds: None,
            review_timeout_seconds: None,
            request_id: request_id.into(),
        }
    }

    fn transition(task_id: &str, revision: u32, request_id: &str) -> Operation {
        Operation::TaskStart {
            task_id: task_id.into(),
            revision,
            expected_version: None,
            request_id: request_id.into(),
        }
    }

    fn submit(task_id: &str, revision: u32, attempt_id: Option<&str>, request_id: &str) -> Operation {
        Operation::TaskSubmit {
            task_id: task_id.into(),
            revision,
            result: "done".into(),
            evidence: "tests pass".into(),
            expected_version: None,
            attempt_id: attempt_id.map(str::to_owned),
            request_id: request_id.into(),
            evidence_ids: vec![],
        }
    }

    fn review(task_id: &str, revision: u32, accepted: bool, request_id: &str) -> Operation {
        Operation::TaskReview {
            task_id: task_id.into(),
            revision,
            accepted,
            feedback: if accepted { "looks good".into() } else { "redo".into() },
            expected_version: None,
            request_id: request_id.into(),
        }
    }

    #[test]
    fn attempts_versions_and_events_commit_with_transitions() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let reviewer = actor(&store, "reviewer");
        let assigned = store
            .execute(
                &issuer,
                "run-issuer",
                &assign(&issuer, "worker", Some("reviewer"), &Uuid::new_v4().to_string()),
            )
            .unwrap();
        assert_eq!(assigned["state"], "queued");
        assert_eq!(assigned["version"], 1);
        let task_id = assigned["id"].as_str().unwrap().to_owned();
        let started = store
            .execute(&worker, "run-worker", &transition(&task_id, 1, &Uuid::new_v4().to_string()))
            .unwrap();
        assert_eq!(started["state"], "running");
        assert_eq!(started["version"], 2);
        let attempt_id = started["attempt_id"].as_str().unwrap().to_owned();
        let submitted = store
            .execute(
                &worker,
                "run-worker",
                &submit(&task_id, 1, Some(&attempt_id), &Uuid::new_v4().to_string()),
            )
            .unwrap();
        assert_eq!(submitted["state"], "submitted");
        assert_eq!(submitted["version"], 3);
        let task = store.task(&reviewer, &task_id).unwrap();
        assert_eq!(task.attempts.len(), 1);
        assert_eq!(task.attempts[0].certainty, "finished");
        assert_eq!(task.attempts[0].outcome.as_deref(), Some("submitted"));
        assert!(task.attempts[0].finished_at.is_some());
        assert_eq!(task.revision, 1);
        let events = store.events("/project", None, None).unwrap();
        let kinds: Vec<&str> = events["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|event| event["kind"].as_str().unwrap())
            .collect();
        assert!(kinds.contains(&"task_assigned"));
        assert!(kinds.contains(&"task_started"));
        assert!(kinds.contains(&"task_submitted"));
        let started_event = events["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["kind"] == "task_started")
            .unwrap();
        assert_eq!(started_event["attempt"], attempt_id.as_str());
        assert_eq!(started_event["resource"], task_id.as_str());
        let paged = store.events("/project", None, Some(2)).unwrap();
        assert_eq!(paged["events"].as_array().unwrap().len(), 2);
        let cursor = paged["cursor"].as_u64().unwrap();
        let rest = store.events("/project", Some(cursor), Some(2)).unwrap();
        assert!(!rest["events"].as_array().unwrap().is_empty());
        assert!(rest["events"][0]["sequence"].as_u64().unwrap() > cursor);
        assert_eq!(store.pending(&worker).unwrap().len(), 0);
        assert_eq!(store.pending(&reviewer).unwrap().len(), 1);
        assert_eq!(store.pending(&reviewer).unwrap()[0].kind, "review");
    }

    #[test]
    fn stale_work_and_wrong_callers_get_stable_codes() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let task_id = store
            .execute(
                &issuer,
                "run-issuer",
                &assign(&issuer, "worker", None, &Uuid::new_v4().to_string()),
            )
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let wrong_version = store
            .execute(
                &worker,
                "run-worker",
                &Operation::TaskStart {
                    task_id: task_id.clone(),
                    revision: 1,
                    expected_version: Some(99),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&wrong_version), "version_conflict");
        assert_eq!(
            wrong_version.downcast_ref::<DomainError>().unwrap().version,
            Some(1)
        );
        let wrong_revision = store
            .execute(&worker, "run-worker", &transition(&task_id, 7, &Uuid::new_v4().to_string()))
            .unwrap_err();
        assert_eq!(code(&wrong_revision), "stale_revision");
        let unauthorized_start = store
            .execute(
                &issuer,
                "run-issuer",
                &transition(&task_id, 1, &Uuid::new_v4().to_string()),
            )
            .unwrap_err();
        assert_eq!(code(&unauthorized_start), "unauthorized");
        let started = store
            .execute(&worker, "run-worker", &transition(&task_id, 1, &Uuid::new_v4().to_string()))
            .unwrap();
        let attempt_id = started["attempt_id"].as_str().unwrap().to_owned();
        let foreign_run = store
            .execute(
                &worker,
                "run-foreign",
                &submit(&task_id, 1, None, &Uuid::new_v4().to_string()),
            )
            .unwrap_err();
        assert_eq!(code(&foreign_run), "stale_attempt");
        let wrong_attempt = store
            .execute(
                &worker,
                "run-worker",
                &submit(
                    &task_id,
                    1,
                    Some(&Uuid::new_v4().to_string()),
                    &Uuid::new_v4().to_string(),
                ),
            )
            .unwrap_err();
        assert_eq!(code(&wrong_attempt), "stale_attempt");
        assert!(store
            .execute(
                &worker,
                "run-worker",
                &review(&task_id, 1, true, &Uuid::new_v4().to_string()),
            )
            .is_err());
        let outsider = actor(&store, "reviewer");
        let foreign_task = store
            .execute(
                &outsider,
                "run-other",
                &Operation::TaskGet {
                    task_id: task_id.clone(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&foreign_task), "scope_denied");
        store
            .execute(
                &worker,
                "run-worker",
                &submit(&task_id, 1, Some(&attempt_id), &Uuid::new_v4().to_string()),
            )
            .unwrap();
        assert_eq!(store.pending(&issuer).unwrap().len(), 1);
    }

    #[test]
    fn request_replay_conflicts_and_epoch_expiry() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        actor(&store, "worker");
        let request_id = Uuid::new_v4().to_string();
        let operation = send(&issuer, "worker", "hello", &request_id);
        let first = store.execute(&issuer, "run-1", &operation).unwrap();
        let replay = store.execute(&issuer, "run-1", &operation).unwrap();
        assert_eq!(first, replay);
        assert_eq!(store.pending(&actor(&store, "worker")).unwrap().len(), 1);
        let conflicted = store
            .execute(&issuer, "run-1", &send(&issuer, "worker", "changed", &request_id))
            .unwrap_err();
        assert_eq!(code(&conflicted), "request_conflict");
        let expired = store.execute(&issuer, "run-2", &operation).unwrap_err();
        assert_eq!(code(&expired), "request_epoch_expired");
        assert_eq!(store.pending(&actor(&store, "worker")).unwrap().len(), 1);
        let invalid = store
            .execute(&issuer, "run-1", &send(&issuer, "worker", "hi", "not-a-uuid"))
            .unwrap_err();
        assert_eq!(code(&invalid), "invalid_input");
    }

    #[test]
    fn retention_exceeds_the_epoch_and_aged_requests_are_pruned() {
        assert!(REQUEST_RETENTION > MUTATION_EPOCH);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bus.sqlite");
        let path = path.to_str().unwrap();
        let request_id = Uuid::new_v4().to_string();
        {
            let store = Store::open(path).unwrap();
            let issuer = actor(&store, "issuer");
            actor(&store, "worker");
            store
                .execute(
                    &issuer,
                    "run-1",
                    &send(&issuer, "worker", "hello", &request_id),
                )
                .unwrap();
            diesel::sql_query("UPDATE requests SET created_at = 0")
                .execute(&mut *store.connection.borrow_mut())
                .unwrap();
            assert_eq!(store.request_count().unwrap(), 1);
        }
        let store = Store::open(path).unwrap();
        assert_eq!(store.request_count().unwrap(), 0);
        let issuer = actor(&store, "issuer");
        store
            .execute(
                &issuer,
                "run-1",
                &send(&issuer, "worker", "hello", &request_id),
            )
            .unwrap();
        assert_eq!(store.pending(&actor(&store, "worker")).unwrap().len(), 2);
    }

    #[test]
    fn unknown_outcomes_require_the_unchanged_original_owner_attempt_and_run() {
        for outcome in ["submitted", "failed", "cancelled"] {
            let store = Store::open(":memory:").unwrap();
            let issuer = actor(&store, "issuer");
            let worker = actor(&store, "worker");
            let assigned = store.execute(&issuer, "issuer-run", &assign(&issuer,
                "worker", None, &Uuid::new_v4().to_string())).unwrap();
            let task_id = assigned["id"].as_str().unwrap();
            store.execute(&worker, "original-run", &transition(task_id, 1,
                &Uuid::new_v4().to_string())).unwrap();
            if outcome == "cancelled" {
                store.execute(&issuer, "issuer-run", &Operation::TaskCancel {
                    task_id: task_id.into(), reason: "Confirm the original stop".into(),
                    expected_version: None, request_id: Uuid::new_v4().to_string(),
                }).unwrap();
            }
            diesel::sql_query("UPDATE attempts SET certainty='unknown' WHERE task_id=?")
                .bind::<Text,_>(task_id).execute(&mut *store.connection.borrow_mut()).unwrap();
            let task = store.task(&worker, task_id).unwrap();
            let attempt_id = Some(task.attempts[0].id.clone());
            let request_id = Uuid::new_v4().to_string();
            let operation = match outcome {
                "submitted" => Operation::TaskSubmit { task_id: task_id.into(), revision: 1,
                    result: "Original work completed".into(), evidence: "Original checks passed".into(),
                    attempt_id, expected_version: Some(task.version), evidence_ids: vec![], request_id },
                "failed" => Operation::TaskFail { task_id: task_id.into(), revision: 1,
                    reason: "Original work failed".into(), attempt_id,
                    expected_version: Some(task.version), evidence_ids: vec![], request_id },
                _ => Operation::TaskFinishCancel { task_id: task_id.into(), revision: 1,
                    reason: "Original execution stopped".into(), attempt_id,
                    expected_version: Some(task.version), request_id },
            };
            assert!(store.execute(&worker, "replacement-run", &operation).is_err());
            assert!(store.execute(&issuer, "issuer-run", &operation).is_err());
            diesel::sql_query("UPDATE attempts SET outcome='overridden' WHERE task_id=?")
                .bind::<Text,_>(task_id).execute(&mut *store.connection.borrow_mut()).unwrap();
            assert!(store.execute(&worker, "original-run", &operation).is_err());
            diesel::sql_query("UPDATE attempts SET outcome=NULL WHERE task_id=?")
                .bind::<Text,_>(task_id).execute(&mut *store.connection.borrow_mut()).unwrap();
            let result = store.execute(&worker, "original-run", &operation).unwrap();
            assert_eq!(store.execute(&worker, "original-run", &operation).unwrap(), result);
            let finished = store.task(&worker, task_id).unwrap();
            assert_eq!(finished.state, outcome);
            assert_eq!(finished.attempts[0].certainty, "finished");
            assert_eq!(finished.attempts[0].outcome.as_deref(), Some(outcome));
            assert!(finished.attempts[0].finished_at.is_some());
        }
    }

    #[test]
    fn recover_interrupts_only_once_per_revision() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let task_id = store
            .execute(
                &issuer,
                "run-issuer",
                &assign(&issuer, "worker", None, &Uuid::new_v4().to_string()),
            )
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        store
            .execute(&worker, "run-1", &transition(&task_id, 1, &Uuid::new_v4().to_string()))
            .unwrap();
        store.recover(&worker, "run-2").unwrap();
        let task = store.task(&worker, &task_id).unwrap();
        assert_eq!(task.state, "running");
        assert_eq!(task.version, 3);
        assert_eq!(task.attempts[0].certainty, "interrupted");
        assert!(task.attempts[0].finished_at.is_none());
        let pending = store.pending(&worker).unwrap();
        assert_eq!(pending.len(), 1);
        assert!(pending[0].body.contains("Interrupted task"));
        store.recover(&worker, "run-2").unwrap();
        assert_eq!(store.task(&worker, &task_id).unwrap().version, 3);
        assert_eq!(store.pending(&worker).unwrap().len(), 1);
        let unsafe_restart = store
            .execute(&worker, "run-2", &transition(&task_id, 1, &Uuid::new_v4().to_string()))
            .unwrap_err();
        assert_eq!(code(&unsafe_restart), "execution_unknown");
        store.execute_controller("/project", &ControllerOperation::TaskForceCancel {
            task_id: task_id.clone(), reason: "Operator accepts overlap risk".into(),
            expected_version: Some(3), request_id: Uuid::new_v4().to_string()
        }).unwrap();
        let fenced = store.task(&worker, &task_id).unwrap();
        assert_eq!(fenced.attempts[0].certainty, "unknown");
        assert_eq!(fenced.attempts[0].outcome.as_deref(), Some("overridden"));
        assert!(fenced.attempts[0].finished_at.is_none());
        let retry: Operation = serde_json::from_value(json!({"op":"task_retry", "task_id":task_id,
            "reason":"Restart after explicit override", "request_id":Uuid::new_v4().to_string()})).unwrap();
        let unsafe_retry = store.execute(&issuer, "issuer-run", &retry).unwrap_err();
        assert_eq!(code(&unsafe_retry), "execution_unknown");
        let retry: Operation = serde_json::from_value(json!({"op":"task_retry", "task_id":task_id,
            "reason":"Explicitly authorize overlapping replacement", "override_uncertain":true,
            "expected_version":fenced.version, "request_id":Uuid::new_v4().to_string()})).unwrap();
        store.execute(&Store::operator("/project"), OPERATOR_EPOCH, &retry).unwrap();
        let restarted = store.execute(&worker, "run-2", &transition(&task_id, 2, &Uuid::new_v4().to_string())).unwrap();
        assert_eq!(restarted["state"], "running");
        assert_eq!(restarted["version"], 6);
        assert_eq!(restarted["attempts"].as_array().unwrap().len(), 2);
        assert_eq!(store.pending(&worker).unwrap().len(), 0);
    }

    #[test]
    fn threads_links_and_subjects_are_validated() {
        let store = Store::open(":memory:").unwrap();
        let alice = actor(&store, "alice");
        let bob = actor(&store, "bob");
        let carol = actor(&store, "carol");
        let root = store
            .execute(
                &alice,
                "run-alice",
                &send(&alice, "bob", "root", &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let root_id = root["id"].as_str().unwrap().to_owned();
        let reply = store
            .execute(
                &bob,
                "run-bob",
                &Operation::AgentSend {
                    to: "alice".into(),
                    body: "reply".into(),
                    subject: Some("Re: root".into()),
                    thread_id: None,
                    reply_to: Some(root_id.clone()),
                    task_id: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(reply["thread_id"], root_id.as_str());
        assert_eq!(reply["reply_to"], root_id.as_str());
        let outsider = store
            .execute(
                &carol,
                "run-carol",
                &Operation::AgentSend {
                    to: "alice".into(),
                    body: "sneak".into(),
                    subject: None,
                    thread_id: None,
                    reply_to: Some(root_id.clone()),
                    task_id: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&outsider), "scope_denied");
        let hijack = store
            .execute(
                &carol,
                "run-carol",
                &Operation::AgentSend {
                    to: "alice".into(),
                    body: "hijack".into(),
                    subject: None,
                    thread_id: Some(root_id.clone()),
                    reply_to: None,
                    task_id: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&hijack), "scope_denied");
        let fresh = store
            .execute(
                &carol,
                "run-carol",
                &Operation::AgentSend {
                    to: "alice".into(),
                    body: "fresh thread".into(),
                    subject: None,
                    thread_id: Some(Uuid::new_v4().to_string()),
                    reply_to: None,
                    task_id: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert!(fresh["thread_id"].is_string());
        let mismatch = store
            .execute(
                &bob,
                "run-bob",
                &Operation::AgentSend {
                    to: "alice".into(),
                    body: "wrong thread".into(),
                    subject: None,
                    thread_id: Some(Uuid::new_v4().to_string()),
                    reply_to: Some(root_id.clone()),
                    task_id: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&mismatch), "invalid_input");
        let long_subject = store
            .execute(
                &alice,
                "run-alice",
                &Operation::AgentSend {
                    to: "bob".into(),
                    body: "body".into(),
                    subject: Some("s".repeat(crate::MAX_SUBJECT + 1)),
                    thread_id: None,
                    reply_to: None,
                    task_id: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&long_subject), "invalid_input");
        let missing_task = store
            .execute(
                &alice,
                "run-alice",
                &Operation::AgentSend {
                    to: "bob".into(),
                    body: "link".into(),
                    subject: None,
                    thread_id: None,
                    reply_to: None,
                    task_id: Some(Uuid::new_v4().to_string()),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&missing_task), "scope_denied");
    }

    #[test]
    fn inbox_and_task_list_page_by_cursor() {
        let store = Store::open(":memory:").unwrap();
        let alice = actor(&store, "alice");
        let bob = actor(&store, "bob");
        for index in 0..5 {
            store
                .execute(
                    &alice,
                    "run-alice",
                    &send(&alice, "bob", &format!("m{index}"), &Uuid::new_v4().to_string()),
                )
                .unwrap();
        }
        let first = store.inbox(&bob, None, Some(2)).unwrap();
        assert_eq!(first["messages"].as_array().unwrap().len(), 2);
        assert_eq!(first["messages"][0]["body"], "m0");
        let cursor = first["cursor"].as_u64().unwrap();
        let second = store.inbox(&bob, Some(cursor), Some(2)).unwrap();
        assert_eq!(second["messages"][0]["body"], "m2");
        assert!(second["cursor"].as_u64().unwrap() > cursor);
        assert_eq!(
            code(&store.inbox(&bob, None, Some(0)).unwrap_err()),
            "invalid_input"
        );
        assert_eq!(
            code(&store.inbox(&bob, None, Some(201)).unwrap_err()),
            "invalid_input"
        );
        let mut task_ids = Vec::new();
        for index in 0..5 {
            task_ids.push(
                store
                    .execute(
                        &alice,
                        "run-alice",
                        &Operation::TaskAssign {
                            to: "bob".into(),
                            description: format!("task {index}"),
                            acceptance: "done".into(),
                            reviewer: None,
                            dependencies: vec![],
                            start_deadline: None,
                            execution_timeout_seconds: None,
                            review_timeout_seconds: None,
                            request_id: Uuid::new_v4().to_string(),
                        },
                    )
                    .unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let listed = store
            .execute(
                &alice,
                "run-alice",
                &Operation::TaskList {
                    state: None,
                    assignee: Some("bob".into()),
                    cursor: None,
                    limit: Some(3),
                    include_archived: false,
                },
            )
            .unwrap();
        assert_eq!(listed["tasks"].as_array().unwrap().len(), 3);
        let cursor = listed["cursor"].as_u64().unwrap();
        let rest = store
            .execute(
                &alice,
                "run-alice",
                &Operation::TaskList {
                    state: Some("queued".into()),
                    assignee: None,
                    cursor: Some(cursor),
                    limit: None,
                    include_archived: false,
                },
            )
            .unwrap();
        assert_eq!(rest["tasks"].as_array().unwrap().len(), 2);
        assert!(rest["cursor"].is_null() || rest["cursor"].as_u64().unwrap() >= cursor);
    }

    fn legacy_payload() -> String {
        json!({
            "agents": {
                "agent-a": {"id": "agent-a", "terminal": "t1", "name": "a", "program": "claude", "project": "/p"},
                "agent-b": {"id": "agent-b", "terminal": "t2", "name": "b", "program": "claude", "project": "/p"}
            },
            "messages": [
                {"id": "m1", "from": "agent-a", "to": "agent-b", "body": "hello", "task_id": null, "revision": null, "kind": "message", "acknowledged": false, "sequence": 1},
                {"id": "m2", "from": "agent-b", "to": "agent-a", "body": "old", "task_id": null, "revision": null, "kind": "message", "acknowledged": true, "sequence": 2}
            ],
            "tasks": {
                "task-1": {"id": "task-1", "project": "/p", "issuer": "agent-a", "assignee": "agent-b", "reviewer": "agent-a", "description": "work", "acceptance": "done", "state": "running", "revision": 2, "result": null, "evidence": null, "feedback": ["more detail"]},
                "task-2": {"id": "task-2", "project": "/p", "issuer": "agent-a", "assignee": "agent-b", "reviewer": "agent-a", "description": "old work", "acceptance": "done", "state": "accepted", "revision": 1, "result": "ok", "evidence": "log", "feedback": []}
            },
            "retries": {
                "agent-a:11111111-1111-4111-8111-111111111111": {"operation": "legacy-op", "result": {"legacy": true}}
            },
            "sequence": 42
        })
        .to_string()
    }

    fn legacy_database(path: &str, payload: &str) {
        let mut connection = SqliteConnection::establish(path).unwrap();
        connection
            .batch_execute("CREATE TABLE agent_bus_v1 (id INTEGER PRIMARY KEY CHECK(id=1), payload TEXT NOT NULL);")
            .unwrap();
        diesel::sql_query("INSERT INTO agent_bus_v1(id, payload) VALUES (1, ?)")
            .bind::<Text, _>(payload)
            .execute(&mut connection)
            .unwrap();
    }

    #[test]
    fn migration_preserves_v1_data_with_backup_and_restore() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bus.sqlite");
        let path = path.to_str().unwrap();
        let payload = legacy_payload();
        legacy_database(path, &payload);
        let store = Store::open(path).unwrap();
        assert_eq!(read_legacy_payload(path).unwrap().as_deref(), Some(SENTINEL));
        let backup = format!("{path}.pre-upgrade");
        assert!(std::path::Path::new(&backup).exists());
        let a = store.register("t1", "claude", "/p", "a").unwrap();
        let b = store.register("t2", "claude", "/p", "b").unwrap();
        assert_eq!(a.id, "agent-a");
        assert_eq!(b.id, "agent-b");
        let task = store.task(&a, "task-1").unwrap();
        assert_eq!(task.state, "running");
        assert_eq!(task.revision, 2);
        assert_eq!(task.version, 1);
        assert_eq!(task.feedback, vec!["more detail".to_string()]);
        let accepted = store.task(&a, "task-2").unwrap();
        assert_eq!(accepted.state, "accepted");
        assert_eq!(accepted.result.as_deref(), Some("ok"));
        let pending = store.pending(&b).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].body, "hello");
        let expired = store
            .execute(
                &a,
                "run-new",
                &send(&a, "b", "hello", "11111111-1111-4111-8111-111111111111"),
            )
            .unwrap_err();
        assert_eq!(code(&expired), "request_epoch_expired");
        let interrupted = store
            .execute(&b, "run-b", &transition("task-1", 2, &Uuid::new_v4().to_string()))
            .unwrap_err();
        assert_eq!(code(&interrupted), "execution_unknown");
        assert_eq!(store.task(&b, "task-1").unwrap().version, 1);
        drop(store);
        let store = Store::open(path).unwrap();
        assert_eq!(store.task(&b, "task-1").unwrap().state, "running");
        assert_eq!(store.request_count().unwrap(), 1);
        drop(store);
        std::fs::copy(&backup, path).unwrap();
        let store = Store::open(path).unwrap();
        let task = store.task(&a, "task-1").unwrap();
        assert_eq!(task.revision, 2);
        assert_eq!(task.version, 1);
        assert_eq!(store.task(&a, "task-2").unwrap().result.as_deref(), Some("ok"));
        assert_eq!(store.pending(&b).unwrap().len(), 1);
    }

    #[test]
    fn migration_failures_leave_v1_untouched() {
        let directory = tempfile::tempdir().unwrap();
        let malformed = directory.path().join("malformed.sqlite");
        let malformed = malformed.to_str().unwrap();
        legacy_database(malformed, "{not json");
        assert!(Store::open(malformed).is_err());
        assert_eq!(read_legacy_payload(malformed).unwrap().as_deref(), Some("{not json"));
        assert!(!std::path::Path::new(&format!("{malformed}.pre-upgrade")).exists());
        let orphan = directory.path().join("orphan.sqlite");
        let orphan = orphan.to_str().unwrap();
        let payload = json!({
            "agents": {"agent-a": {"id": "agent-a", "terminal": "t1", "name": "a", "program": "claude", "project": "/p"}},
            "messages": [{"id": "m1", "from": "agent-missing", "to": "agent-a", "body": "hello", "task_id": null, "revision": null, "kind": "message", "acknowledged": false, "sequence": 1}],
            "tasks": {},
            "retries": {},
            "sequence": 1
        })
        .to_string();
        legacy_database(orphan, &payload);
        assert!(Store::open(orphan).is_err());
        assert_eq!(read_legacy_payload(orphan).unwrap().as_deref(), Some(payload.as_str()));
        assert!(std::path::Path::new(&format!("{orphan}.pre-upgrade")).exists());
    }

    #[test]
    fn the_v1_loader_rejects_the_sentinel() {
        assert!(serde_json::from_str::<LegacySnapshot>(SENTINEL).is_err());
        assert!(serde_json::from_str::<LegacySnapshot>(&legacy_payload()).is_ok());
    }

    #[test]
    fn pagination_holds_at_capacity_scale() {
        let store = Store::open(":memory:").unwrap();
        let a = actor(&store, "a");
        let b = actor(&store, "b");
        {
            let mut connection = store.connection.borrow_mut();
            connection.batch_execute("BEGIN IMMEDIATE").unwrap();
            for chunk in 0..500 {
                let mut statement = String::from("INSERT INTO messages(id, project, sender, recipient, body, subject, thread_id, reply_to, task_id, revision, kind, acknowledged, sequence) VALUES ");
                for row in 0..200 {
                    if row > 0 {
                        statement.push(',');
                    }
                    let index = chunk * 200 + row + 1;
                    statement.push_str(&format!("('m{index}', '/project', '{}', '{}', '', NULL, NULL, NULL, NULL, NULL, 'message', 0, {index})", a.id, b.id));
                }
                connection.batch_execute(&statement).unwrap();
            }
            for chunk in 0..40 {
                let mut statement = String::from("INSERT INTO tasks(id, project, issuer, assignee, reviewer, description, acceptance, state, revision, version, result, evidence, created_seq) VALUES ");
                for row in 0..250 {
                    if row > 0 {
                        statement.push(',');
                    }
                    let index = chunk * 250 + row + 1;
                    statement.push_str(&format!("('t{index}', '/project', '{}', '{}', '{}', '', '', 'accepted', 1, 1, NULL, NULL, {index})", a.id, b.id, a.id));
                }
                connection.batch_execute(&statement).unwrap();
            }
            connection.batch_execute("COMMIT").unwrap();
        }
        let first = store.inbox(&b, None, None).unwrap();
        assert_eq!(first["messages"].as_array().unwrap().len(), 50);
        let cursor = first["cursor"].as_u64().unwrap();
        let second = store.inbox(&b, Some(cursor), Some(200)).unwrap();
        assert_eq!(second["messages"].as_array().unwrap().len(), 200);
        let mut total = 0usize;
        let mut cursor = None;
        loop {
            let page = store.inbox(&b, cursor, Some(200)).unwrap();
            let items = page["messages"].as_array().unwrap();
            if items.is_empty() {
                break;
            }
            total += items.len();
            cursor = page["cursor"].as_u64();
        }
        assert_eq!(total, 100_000);
        let mut tasks = Vec::new();
        let mut cursor = None;
        loop {
            let page = store
                .execute(
                    &b,
                    "run-b",
                    &Operation::TaskList {
                        state: None,
                        assignee: None,
                        cursor,
                        limit: Some(200),
                        include_archived: false,
                    },
                )
                .unwrap();
            let items = page["tasks"].as_array().unwrap();
            if items.is_empty() {
                break;
            }
            tasks.extend(items.iter().cloned());
            cursor = page["cursor"].as_u64();
        }
        assert_eq!(tasks.len(), 10_000);
        let full = store
            .execute(
                &a,
                "run-a",
                &assign(&a, "b", None, &Uuid::new_v4().to_string()),
            )
            .unwrap_err();
        assert_eq!(code(&full), "capacity_exceeded");
    }

    fn future(seconds: i64) -> String {
        (chrono::Utc::now() + chrono::Duration::seconds(seconds)).to_rfc3339()
    }

    fn export_all(store: &Store) -> Vec<Value> {
        let mut records = Vec::new();
        let mut after = None;
        loop {
            let page = store
                .execute_controller(
                    "/project",
                    &ControllerOperation::HistoryExport {
                        after,
                        limit: Some(50),
                    },
                )
                .unwrap();
            let items = page["records"].as_array().unwrap();
            if items.is_empty() {
                break;
            }
            after = page["cursor"].as_u64();
            records.extend(items.iter().cloned());
        }
        records
    }

    #[test]
    fn archival_recovers_capacity_and_full_inboxes_preserve_stop_requests() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let request_id = || Uuid::new_v4().to_string();
        let task = store.execute(&issuer, "issuer-run", &assign(&issuer, "worker", None, &request_id())).unwrap();
        let id = task["id"].as_str().unwrap();
        store.execute(&worker, "worker-run", &transition(id, 1, &request_id())).unwrap();
        {
            let mut connection = store.connection.borrow_mut();
            for index in 0..MAX_PENDING_PER_AGENT {
                diesel::sql_query("INSERT INTO messages(id, project, sender, recipient, body, kind, acknowledged, sequence) VALUES (?, '/project', ?, ?, 'unread', 'message', 0, ?)")
                    .bind::<Text, _>(format!("full-{index}"))
                    .bind::<Text, _>(&issuer.id).bind::<Text, _>(&worker.id)
                    .bind::<BigInt, _>(index + 100).execute(&mut *connection).unwrap();
            }
        }
        let overflow = store.execute(&issuer, "issuer-run", &send(&issuer, "worker", "more", &request_id())).unwrap_err();
        assert_eq!(code(&overflow), "capacity_exceeded");
        let cancel: Operation = serde_json::from_value(json!({
            "op": "task_cancel", "task_id": id, "reason": "Stop", "request_id": request_id()
        })).unwrap();
        assert_eq!(store.execute(&issuer, "issuer-run", &cancel).unwrap()["state"], "cancel_requested");
        let stopped: Operation = serde_json::from_value(json!({
            "op": "task_finish_cancel", "task_id": id, "revision": 1,
            "reason": "Stopped", "request_id": request_id()
        })).unwrap();
        store.execute(&worker, "worker-run", &stopped).unwrap();
        store.execute_controller("/project", &ControllerOperation::TaskArchive {
            task_id: id.into(), request_id: request_id()
        }).unwrap();
        assert_eq!(store.active_task_count("/project").unwrap(), 0);
        assert_eq!(store.task_count("/project").unwrap(), 1);
        assert!(store.task_states(&issuer, &worker.id).unwrap().is_empty());
        assert_eq!(store.task(&issuer, id).unwrap().state, "cancelled");
        let cancellation_notice = store.pending(&worker).unwrap().into_iter().find(|message| message.kind == "cancelled").unwrap();
        store.execute(&worker, "worker-run", &Operation::AgentAck { message_id: cancellation_notice.id }).unwrap();
        store.execute(&worker, "worker-run", &Operation::AgentAck { message_id: "full-0".into() }).unwrap();
        assert!(store.execute(&issuer, "issuer-run", &send(&issuer, "worker", "more", &request_id())).is_ok());
        assert!(store.execute(&issuer, "issuer-run", &assign(&issuer, "worker", None, &request_id())).is_err(),
            "Full receiver inbox must still refuse fresh assignments");
    }

    #[test]
    #[ignore = "C10-scale history benchmark; run explicitly on both GitHub target platforms"]
    fn representative_history_pages_within_budget_and_preserves_live_work() {
        use std::time::Instant;
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("history.sqlite");
        let store = Store::open(database.to_str().unwrap()).unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        store.transaction(|| {
            diesel::sql_query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<10000) INSERT INTO tasks(id,project,issuer,assignee,reviewer,description,acceptance,state,revision,version,created_seq,archived) SELECT 'completed-'||x,'/project',?,?,?,'historical task','verified','accepted',1,4,x,1 FROM n")
                .bind::<Text, _>(&issuer.id).bind::<Text, _>(&worker.id).bind::<Text, _>(&issuer.id)
                .execute(&mut *store.connection.borrow_mut())?;
            diesel::sql_query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<100000) INSERT INTO messages(id,project,sender,recipient,body,kind,acknowledged,sequence) SELECT 'history-'||x,'/project',?,?,'historical fixture '||x,'message',1,10000+x FROM n")
                .bind::<Text, _>(&issuer.id).bind::<Text, _>(&worker.id)
                .execute(&mut *store.connection.borrow_mut())?;
            diesel::sql_query("INSERT INTO sequences(project,value) VALUES ('/project',110000) ON CONFLICT(project) DO UPDATE SET value=110000")
                .execute(&mut *store.connection.borrow_mut())?;
            Ok(())
        }).unwrap();
        assert_eq!(store.capacity("/project").unwrap()["tasks"]["total"], 10000);
        assert_eq!(store.capacity("/project").unwrap()["messages"]["used"], 100000);
        assert!(store.task_states(&issuer, &worker.id).unwrap().is_empty());
        let mut timings = Vec::new();
        let mut cursor = None;
        let mut count = 0;
        loop {
            let start = Instant::now();
            let page = store.task_list(&issuer, None, None, cursor, Some(200), true).unwrap();
            timings.push(start.elapsed());
            let tasks = page["tasks"].as_array().unwrap();
            if tasks.is_empty() { break; }
            assert!(tasks.iter().all(|task| task["created_seq"].as_u64().unwrap() > cursor.unwrap_or(0)));
            count += tasks.len();
            cursor = page["cursor"].as_u64();
        }
        assert_eq!(count, 10000);
        timings.sort_unstable();
        let p95 = timings[(timings.len() - 1) * 95 / 100];
        println!("C10 OS={} history=100000 completed_tasks=10000 indexed_task_page_p95_ms={:.3}", std::env::consts::OS, p95.as_secs_f64()*1000.);
        assert!(p95 < Duration::from_millis(150), "Indexed task listing exceeded the C10 reference-runner target: {p95:?}");
        let search = store.message_search(&issuer, "historical fixture 100000", None, None, None, Some(50)).unwrap();
        assert_eq!(search["messages"].as_array().unwrap().len(), 1);
        let exported = store.history_export("/project", None, Some(50)).unwrap();
        assert_eq!(exported["records"].as_array().unwrap().len(), 50);
        let next = store.history_export("/project", exported["cursor"].as_u64(), Some(50)).unwrap();
        assert!(next["cursor"].as_u64().unwrap() > exported["cursor"].as_u64().unwrap());
        let live = store.execute(&issuer, "issuer-run", &assign(&issuer, "worker", None, &Uuid::new_v4().to_string())).unwrap();
        let send = send(&issuer, "worker", "Live message", &Uuid::new_v4().to_string());
        let sent = store.execute(&issuer, "issuer-run", &send).unwrap();
        let preview = store.purge_preview("/project").unwrap();
        assert_eq!(preview["tasks"], 10000);
        assert_eq!(preview["messages"], 100000);
        store.execute_controller("/project", &ControllerOperation::HistoryPurge { expected_sequence: None,
            archived_tasks: true, acknowledged_messages: true, request_id: Uuid::new_v4().to_string()
        }).unwrap();
        assert_eq!(store.task(&issuer, live["id"].as_str().unwrap()).unwrap().state, "queued");
        assert_eq!(store.pending(&worker).unwrap().len(), 2);
        assert_eq!(store.execute(&issuer, "issuer-run", &send).unwrap(), sent);
        assert_eq!(store.task_count("/project").unwrap(), 1);
    }

    #[test]
    fn cancellation_preserves_exclusive_execution_until_stop_is_confirmed() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let operation = |mut value: Value| {
            value["request_id"] = json!(Uuid::new_v4().to_string());
            serde_json::from_value::<Operation>(value).unwrap()
        };
        let first = store.execute(&issuer, "issuer-run", &assign(&issuer, "worker", None, &Uuid::new_v4().to_string())).unwrap();
        let second = store.execute(&issuer, "issuer-run", &assign(&issuer, "worker", None, &Uuid::new_v4().to_string())).unwrap();
        let pool = store.execute(&issuer, "issuer-run", &operation(json!({
            "op": "task_create_pool", "description": "pool", "acceptance": "checked",
            "eligible": ["worker"], "request_id": "pool"
        }))).unwrap();
        store.execute(&worker, "worker-run", &transition(first["id"].as_str().unwrap(), 1, &Uuid::new_v4().to_string())).unwrap();
        let cancelled = store.execute(&issuer, "issuer-run", &operation(json!({
            "op": "task_cancel", "task_id": first["id"], "reason": "Stop work", "request_id": "cancel"
        }))).unwrap();
        assert_eq!(cancelled["state"], "cancel_requested");
        let forged_stop = store.execute(&worker, "replacement-run", &operation(json!({
            "op": "task_finish_cancel", "task_id": first["id"], "revision": 1,
            "reason": "Unproven stop", "request_id": "forged-stop"
        }))).unwrap_err();
        assert_eq!(code(&forged_stop), "stale_attempt");
        let blocked_start = store.execute(&worker, "worker-run", &transition(second["id"].as_str().unwrap(), 1, &Uuid::new_v4().to_string())).unwrap_err();
        assert_eq!(code(&blocked_start), "invalid_state");
        let blocked_claim = store.execute(&worker, "worker-run", &operation(json!({
            "op": "task_claim", "task_id": pool["id"], "request_id": "claim"
        }))).unwrap_err();
        assert_eq!(code(&blocked_claim), "invalid_state");
        assert_eq!(store.next_work(&worker, "worker-run").unwrap().unwrap().kind, "cancel_requested");
        store.execute(&worker, "worker-run", &operation(json!({
            "op": "task_finish_cancel", "task_id": first["id"], "revision": 1,
            "reason": "Stopped", "request_id": "stopped"
        }))).unwrap();
        assert!(store.execute(&worker, "worker-run", &transition(second["id"].as_str().unwrap(), 1, &Uuid::new_v4().to_string())).is_ok());
    }

    #[test]
    fn long_rework_history_preserves_results_and_pages_within_wire_bounds() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let stranger = actor(&store, "stranger");
        let task = store.execute(&issuer, "issuer-run", &assign(&issuer, "worker", None, &Uuid::new_v4().to_string())).unwrap();
        let id = task["id"].as_str().unwrap();
        for revision in 1..=20 {
            store.execute(&worker, "worker-run", &transition(id, revision, &Uuid::new_v4().to_string())).unwrap();
            let submitted: Operation = serde_json::from_value(json!({"op":"task_submit", "task_id":id,
                "revision":revision, "result":format!("result for generation {revision}"),
                "evidence":format!("checked {revision}"), "request_id":Uuid::new_v4().to_string()})).unwrap();
            store.execute(&worker, "worker-run", &submitted).unwrap();
            let rework: Operation = serde_json::from_value(json!({"op":"task_review", "task_id":id,
                "revision":revision, "accepted":false, "feedback":format!("Check again {revision}"),
                "request_id":Uuid::new_v4().to_string()})).unwrap();
            store.execute(&issuer, "issuer-run", &rework).unwrap();
        }
        let detail = store.task(&issuer, id).unwrap();
        assert!(detail.result.is_none());
        assert!(detail.history_truncated);
        assert_eq!(detail.attempts.len(), 16);
        assert_eq!(detail.feedback.len(), 8);
        assert_eq!(detail.feedback.last().unwrap(), "Check again 20");
        assert_eq!(code(&store.task_history(&stranger, id, None, None).unwrap_err()), "scope_denied");
        // History must survive expiry of the response cache.
        diesel::sql_query("DELETE FROM requests").execute(&mut *store.connection.borrow_mut()).unwrap();
        let mut cursor = None;
        let mut submissions = Vec::new();
        loop {
            let page = store.task_history(&issuer, id, cursor, Some(3)).unwrap();
            let events = page["events"].as_array().unwrap();
            if events.is_empty() { break; }
            cursor = page["cursor"].as_u64();
            submissions.extend(events.iter().filter(|event| event["kind"] == "task_submitted").cloned());
        }
        assert_eq!(submissions.len(), 20);
        assert_eq!(submissions[0]["payload"]["result"], "result for generation 1");
        assert_eq!(submissions[19]["payload"]["evidence"], "checked 20");
        for _ in 0..100 {
            store.execute(&issuer, "issuer-run", &send(&issuer, "worker", &"\"".repeat(crate::MAX_TEXT), &Uuid::new_v4().to_string())).unwrap();
        }
        let mut cursor = None;
        let mut messages = 0;
        loop {
            let page = store.inbox(&worker, cursor, Some(200)).unwrap();
            let envelope = json!({"content":[{"type":"text","text":page.to_string()}]});
            assert!(serde_json::to_vec(&envelope).unwrap().len() < crate::MAX_FRAME);
            let items = page["messages"].as_array().unwrap();
            if items.is_empty() { break; }
            messages += items.len();
            cursor = page["cursor"].as_u64();
        }
        assert_eq!(messages, 101, "Current assignment plus all ordinary messages remain readable");
    }

    #[test]
    fn sqlite_full_rolls_back_new_work_and_preserves_existing_records() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("full.sqlite");
        let store = Store::open(database.to_str().unwrap()).unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let existing = store.execute(&issuer, "issuer-run", &send(&issuer, "worker", "Existing message", &Uuid::new_v4().to_string())).unwrap();
        store.connection.borrow_mut().batch_execute("PRAGMA max_page_count = 1").unwrap();
        let full = store.execute(&issuer, "issuer-run", &send(&issuer, "worker", &"x".repeat(crate::MAX_TEXT), &Uuid::new_v4().to_string())).unwrap_err();
        assert_eq!(code(&full), "storage_full");
        let pending = store.pending(&worker).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, existing["id"].as_str().unwrap());
        assert_eq!(pending[0].body, "Existing message");
        assert_eq!(store.request_count().unwrap(), 1, "A failed write cannot leave a dedup success row");
    }

    #[test]
    fn failed_commit_releases_transaction_before_the_identical_retry() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("commit.sqlite");
        let store = Store::open(database.to_str().unwrap()).unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        store.connection.borrow_mut().batch_execute("PRAGMA busy_timeout=0").unwrap();
        let mut reader = SqliteConnection::establish(database.to_str().unwrap()).unwrap();
        reader.batch_execute("BEGIN; SELECT COUNT(*) FROM messages;").unwrap();
        let send = send(&issuer, "worker", "Committed once", &Uuid::new_v4().to_string());
        assert!(store.execute(&issuer, "issuer-run", &send).is_err());
        assert!(store.pending(&worker).unwrap().is_empty());
        assert_eq!(store.request_count().unwrap(), 0);
        reader.batch_execute("ROLLBACK").unwrap();
        let sent = store.execute(&issuer, "issuer-run", &send).unwrap();
        assert_eq!(store.execute(&issuer, "issuer-run", &send).unwrap(), sent);
        assert_eq!(store.pending(&worker).unwrap().len(), 1);
    }

    #[test]
    fn dependencies_gate_starts_until_prerequisites_accept() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let prerequisite = store
            .execute(
                &issuer,
                "run-issuer",
                &assign(&issuer, "worker", None, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let prerequisite_id = prerequisite["id"].as_str().unwrap().to_owned();
        let dependent = store
            .execute(
                &issuer,
                "run-issuer",
                &Operation::TaskAssign {
                    to: "worker".into(),
                    description: "dependent".into(),
                    acceptance: "evidence".into(),
                    reviewer: None,
                    dependencies: vec![prerequisite_id.clone()],
                    start_deadline: None,
                    execution_timeout_seconds: None,
                    review_timeout_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(dependent["state"], "blocked");
        assert_eq!(dependent["version"], 1);
        assert_eq!(dependent["dependencies"][0], prerequisite_id.as_str());
        assert!(dependent["wait_reason"].is_string());
        let dependent_id = dependent["id"].as_str().unwrap().to_owned();
        let blocked = store
            .execute(
                &worker,
                "run-worker",
                &transition(&dependent_id, 1, &Uuid::new_v4().to_string()),
            )
            .unwrap_err();
        assert_eq!(code(&blocked), "dependency_blocked");
        let started = store
            .execute(
                &worker,
                "run-worker",
                &transition(&prerequisite_id, 1, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let attempt_id = started["attempt_id"].as_str().unwrap().to_owned();
        store
            .execute(
                &worker,
                "run-worker",
                &submit(&prerequisite_id, 1, Some(&attempt_id), &Uuid::new_v4().to_string()),
            )
            .unwrap();
        store
            .execute(
                &issuer,
                "run-issuer",
                &review(&prerequisite_id, 1, true, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let unblocked = store.task(&issuer, &dependent_id).unwrap();
        assert_eq!(unblocked.state, "queued");
        assert_eq!(unblocked.version, 2);
        assert!(unblocked.wait_reason.is_none());
        let running = store
            .execute(
                &worker,
                "run-worker",
                &transition(&dependent_id, 1, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        assert_eq!(running["state"], "running");
    }

    #[test]
    fn dependency_cycles_and_foreign_edges_are_refused() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        actor(&store, "worker");
        let first = store
            .execute(
                &issuer,
                "run-issuer",
                &assign(&issuer, "worker", None, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let first_id = first["id"].as_str().unwrap().to_owned();
        let second = store
            .execute(
                &issuer,
                "run-issuer",
                &assign(&issuer, "worker", None, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let second_id = second["id"].as_str().unwrap().to_owned();
        let self_edge = store
            .execute(
                &issuer,
                "run-issuer",
                &Operation::TaskSetDependencies {
                    task_id: first_id.clone(),
                    dependencies: vec![first_id.clone()],
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&self_edge), "invalid_input");
        let blocked = store
            .execute(
                &issuer,
                "run-issuer",
                &Operation::TaskSetDependencies {
                    task_id: first_id.clone(),
                    dependencies: vec![second_id.clone()],
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(blocked["state"], "blocked");
        assert_eq!(blocked["version"], 2);
        let cycle = store
            .execute(
                &issuer,
                "run-issuer",
                &Operation::TaskSetDependencies {
                    task_id: second_id.clone(),
                    dependencies: vec![first_id.clone()],
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&cycle), "dependency_cycle");
        assert!(store.task(&issuer, &second_id).unwrap().dependencies.is_empty());
        let far = store.register("terminal-far", "claude", "/other", "far").unwrap();
        let peer = store.register("terminal-peer", "claude", "/other", "peer").unwrap();
        let foreign = store
            .execute(
                &far,
                "run-far",
                &assign(&far, "peer", None, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let foreign_id = foreign["id"].as_str().unwrap().to_owned();
        assert_ne!(foreign["project"], "/project");
        assert_eq!(peer.project, "/other");
        let cross = store
            .execute(
                &issuer,
                "run-issuer",
                &Operation::TaskSetDependencies {
                    task_id: first_id.clone(),
                    dependencies: vec![foreign_id],
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&cross), "scope_denied");
        let preserved = store.task(&issuer, &first_id).unwrap();
        assert_eq!(preserved.dependencies, vec![second_id]);
        assert_eq!(preserved.version, 2);
    }

    #[test]
    fn pool_claims_admit_one_winner_and_only_eligible_agents() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let other = actor(&store, "other");
        let pool = store
            .execute(
                &issuer,
                "run-issuer",
                &Operation::TaskCreatePool {
                    description: "shared work".into(),
                    acceptance: "tests".into(),
                    eligible: vec!["worker".into(), "other".into()],
                    reviewer: None,
                    dependencies: vec![],
                    start_deadline: None,
                    execution_timeout_seconds: None,
                    review_timeout_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(pool["state"], "queued");
        assert_eq!(pool["assignee"], "");
        assert_eq!(pool["eligible"].as_array().unwrap().len(), 2);
        assert_eq!(pool["wait_reason"], "Unclaimed pool task");
        let pool_id = pool["id"].as_str().unwrap().to_owned();
        let issuer_claim = store
            .execute(
                &issuer,
                "run-issuer",
                &Operation::TaskClaim {
                    task_id: pool_id.clone(),
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&issuer_claim), "unauthorized");
        let stale = store
            .execute(
                &worker,
                "run-worker",
                &Operation::TaskClaim {
                    task_id: pool_id.clone(),
                    expected_version: Some(99),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&stale), "version_conflict");
        let claimed = store
            .execute(
                &worker,
                "run-worker",
                &Operation::TaskClaim {
                    task_id: pool_id.clone(),
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(claimed["assignee"], worker.id.as_str());
        assert_eq!(claimed["version"], 2);
        let loser = store
            .execute(
                &other,
                "run-other",
                &Operation::TaskClaim {
                    task_id: pool_id.clone(),
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&loser), "scope_denied");
        let second_pool = store
            .execute(
                &issuer,
                "run-issuer",
                &Operation::TaskCreatePool {
                    description: "only other".into(),
                    acceptance: "tests".into(),
                    eligible: vec!["other".into()],
                    reviewer: None,
                    dependencies: vec![],
                    start_deadline: None,
                    execution_timeout_seconds: None,
                    review_timeout_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        let second_id = second_pool["id"].as_str().unwrap().to_owned();
        let ineligible = store
            .execute(
                &worker,
                "run-worker",
                &Operation::TaskClaim {
                    task_id: second_id.clone(),
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&ineligible), "scope_denied");
        store
            .execute(
                &other,
                "run-other",
                &Operation::TaskClaim {
                    task_id: second_id.clone(),
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        store
            .execute(
                &other,
                "run-other",
                &transition(&second_id, 1, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let third_pool = store
            .execute(
                &issuer,
                "run-issuer",
                &Operation::TaskCreatePool {
                    description: "queued behind".into(),
                    acceptance: "tests".into(),
                    eligible: vec!["other".into()],
                    reviewer: None,
                    dependencies: vec![],
                    start_deadline: None,
                    execution_timeout_seconds: None,
                    review_timeout_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        let third_id = third_pool["id"].as_str().unwrap().to_owned();
        let busy = store
            .execute(
                &other,
                "run-other",
                &Operation::TaskClaim {
                    task_id: third_id,
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&busy), "invalid_state");
    }

    #[test]
    fn deadlines_expire_and_timeouts_request_stop() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let past = store
            .execute(
                &issuer,
                "run-issuer",
                &Operation::TaskAssign {
                    to: "worker".into(),
                    description: "work".into(),
                    acceptance: "evidence".into(),
                    reviewer: None,
                    dependencies: vec![],
                    start_deadline: Some("2000-01-01T00:00:00Z".into()),
                    execution_timeout_seconds: None,
                    review_timeout_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&past), "invalid_input");
        let task = store
            .execute(
                &issuer,
                "run-issuer",
                &Operation::TaskAssign {
                    to: "worker".into(),
                    description: "work".into(),
                    acceptance: "evidence".into(),
                    reviewer: None,
                    dependencies: vec![],
                    start_deadline: Some(future(60)),
                    execution_timeout_seconds: Some(1),
                    review_timeout_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        let task_id = task["id"].as_str().unwrap().to_owned();
        assert!(task["start_deadline"].as_u64().unwrap() > now());
        diesel::sql_query("UPDATE tasks SET start_deadline = ? WHERE id = ?")
            .bind::<BigInt, _>((now() as i64) - 1_000)
            .bind::<Text, _>(&task_id)
            .execute(&mut *store.connection.borrow_mut())
            .unwrap();
        store
            .execute(&issuer, "run-issuer", &Operation::AgentList)
            .unwrap();
        let expired = store.task(&issuer, &task_id).unwrap();
        assert_eq!(expired.state, "expired");
        assert_eq!(expired.version, 2);
        assert!(expired.start_deadline.is_none());
        let retried = store
            .execute(
                &issuer,
                "run-issuer",
                &Operation::TaskRetry {
                    task_id: task_id.clone(),
                    reason: "another attempt".into(),
                    start_deadline: Some(future(60)),
                    clear_start_deadline: false,
                    override_uncertain: false,
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(retried["state"], "queued");
        assert_eq!(retried["revision"], 2);
        let started = store
            .execute(
                &worker,
                "run-worker",
                &transition(&task_id, 2, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let attempt_id = started["attempt_id"].as_str().unwrap().to_owned();
        diesel::sql_query("UPDATE tasks SET execution_deadline = ? WHERE id = ?")
            .bind::<BigInt, _>((now() as i64) - 1_000)
            .bind::<Text, _>(&task_id)
            .execute(&mut *store.connection.borrow_mut())
            .unwrap();
        store
            .execute(&worker, "run-worker", &Operation::AgentList)
            .unwrap();
        let requested = store.task(&issuer, &task_id).unwrap();
        assert_eq!(requested.state, "cancel_requested");
        let finished = store
            .execute(
                &worker,
                "run-worker",
                &Operation::TaskFinishCancel {
                    task_id: task_id.clone(),
                    revision: 2,
                    reason: "stopped after timeout".into(),
                    attempt_id: Some(attempt_id),
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(finished["state"], "cancelled");
        let stopped = store.task(&issuer, &task_id).unwrap();
        assert_eq!(stopped.attempts[0].outcome.as_deref(), Some("cancelled"));
        let retry: Operation = serde_json::from_value(json!({"op":"task_retry", "task_id":task_id,
            "reason":"Try again", "start_deadline":future(60), "request_id":Uuid::new_v4().to_string()})).unwrap();
        store.execute(&issuer, "run-issuer", &retry).unwrap();
        diesel::sql_query("UPDATE tasks SET start_deadline = 0 WHERE id = ?")
            .bind::<Text, _>(&task_id).execute(&mut *store.connection.borrow_mut()).unwrap();
        store.execute(&issuer, "run-issuer", &Operation::AgentList).unwrap();
        let expired_retry = store.task(&issuer, &task_id).unwrap();
        assert_eq!(expired_retry.revision, 3);
        assert_eq!(expired_retry.state, "expired", "An old revision's attempt must not suppress a new start deadline");
    }

    #[test]
    fn clock_jumps_do_not_resurrect_work_or_transfer_execution() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let baseline = now();
        let day_ms = Duration::from_secs(24 * 60 * 60).as_millis() as u64;
        let queued = store
            .execute(
                &issuer,
                "issuer-run",
                &serde_json::from_value(json!({
                    "op": "task_assign", "to": "worker", "description": "Queued work",
                    "acceptance": "Evidence", "start_deadline": future(3600),
                    "request_id": Uuid::new_v4().to_string()
                }))
                .unwrap(),
            )
            .unwrap();
        let queued_id = queued["id"].as_str().unwrap();
        let running = store
            .execute(
                &issuer,
                "issuer-run",
                &serde_json::from_value(json!({
                    "op": "task_assign", "to": "worker", "description": "Running work",
                    "acceptance": "Evidence", "execution_timeout_seconds": 3600,
                    "request_id": Uuid::new_v4().to_string()
                }))
                .unwrap(),
            )
            .unwrap();
        let running_id = running["id"].as_str().unwrap();
        let start = transition(running_id, 1, &Uuid::new_v4().to_string());
        let started = store.execute(&worker, "worker-run", &start).unwrap();
        let original = store.task(&issuer, running_id).unwrap();
        assert_eq!(original.attempts.len(), 1);

        store
            .sweep_at(&issuer.project, baseline.saturating_sub(day_ms))
            .unwrap();
        assert_eq!(store.task(&issuer, queued_id).unwrap().state, "queued");
        assert_eq!(
            store.task(&issuer, running_id).unwrap().version,
            original.version
        );
        store.sweep_at(&issuer.project, baseline + day_ms).unwrap();
        let expired = store.task(&issuer, queued_id).unwrap();
        let overdue = store.task(&issuer, running_id).unwrap();
        assert_eq!(expired.state, "expired");
        assert!(expired.attempts.is_empty());
        assert_eq!(overdue.state, "cancel_requested");
        assert_eq!(json!(overdue.attempts), json!(original.attempts));
        assert_eq!(overdue.assignee, worker.id);
        let events = store.events(&issuer.project, None, Some(200)).unwrap();

        // Moving UTC back cannot undo a committed expiry or prove execution stopped.
        for observed in [baseline.saturating_sub(day_ms), baseline + 2 * day_ms, baseline] {
            store.sweep_at(&issuer.project, observed).unwrap();
            assert_eq!(
                store.task(&issuer, queued_id).unwrap().version,
                expired.version
            );
            assert_eq!(
                store.task(&issuer, running_id).unwrap().version,
                overdue.version
            );
            assert_eq!(
                store.events(&issuer.project, None, Some(200)).unwrap(),
                events
            );
        }
        assert_eq!(
            store.execute(&worker, "worker-run", &start).unwrap(),
            started
        );
        let current = store.task(&issuer, running_id).unwrap();
        assert_eq!(current.state, "cancel_requested");
        assert_eq!(json!(current.attempts), json!(original.attempts));
        assert_eq!(
            code(
                &store
                    .execute(
                        &issuer,
                        "issuer-run",
                        &serde_json::from_value(json!({
                            "op": "task_retry", "task_id": running_id, "reason": "Clock moved",
                            "request_id": Uuid::new_v4().to_string()
                        }))
                        .unwrap()
                    )
                    .unwrap_err()
            ),
            "invalid_state"
        );
    }

    #[test]
    fn operator_reservation_maintenance_is_scoped_fenced_and_preserves_unknown_attempts() {
        let store = Store::open(":memory:").unwrap();
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_str().unwrap();
        let issuer = store.register("issuer", "codex", root, "issuer").unwrap();
        let worker = store.register("worker", "codex", root, "worker").unwrap();
        let task = store.execute(&issuer, "issuer-run", &assign(&issuer, "worker", None, &Uuid::new_v4().to_string())).unwrap();
        let id = task["id"].as_str().unwrap();
        store.execute(&worker, "worker-run", &transition(id, 1, &Uuid::new_v4().to_string())).unwrap();
        let reserved = store.execute(&worker, "worker-run", &Operation::FileReserve {
            paths: vec!["fixture.rs".into()], mode: "exclusive".into(), task_id: Some(id.into()),
            attempt_id: None, ttl_seconds: Some(600), request_id: Uuid::new_v4().to_string(),
        }).unwrap();
        let reservation = store.reservation_by_id(reserved["reservation_ids"][0].as_str().unwrap()).unwrap().unwrap();
        let intent = |expiry, ttl| ControllerOperation::ReservationUpdate {
            reservation_id: reservation.id.clone(), workspace: reservation.workspace.clone(), expected_owner: worker.id.clone(),
            expected_expires_at: expiry, ttl_seconds: ttl, reason: "Reviewed maintenance".into(), request_id: Uuid::new_v4().to_string(),
        };
        assert_eq!(code(&store.execute_controller("/foreign", &intent(reservation.expires_at as u64, Some(60))).unwrap_err()), "scope_denied");
        assert_eq!(code(&store.execute_controller(root, &intent(0, Some(60))).unwrap_err()), "invalid_state");
        let renewed = store.execute_controller(root, &intent(reservation.expires_at as u64, Some(60))).unwrap();
        let expiry = renewed["expires_at"].as_u64().unwrap();
        store.execute_controller(root, &ControllerOperation::TaskForceCancel { task_id: id.into(), reason: "Unknown effects".into(), expected_version: None, request_id: Uuid::new_v4().to_string() }).unwrap();
        assert_eq!(code(&store.execute_controller(root, &intent(expiry, Some(60))).unwrap_err()), "execution_unknown");
        let released = store.execute_controller(root, &intent(expiry, None)).unwrap();
        assert_eq!(released["execution_stopped"], false);
        assert!(store.reservation_by_id(&reservation.id).unwrap().is_none());
        let task = store.operator_task(root, id).unwrap();
        assert_eq!(task.attempts[0].certainty, "unknown");
        assert!(task.attempts[0].finished_at.is_none());
    }

    #[test]
    fn reservations_are_exclusive_expiring_and_releasable() {
        let store = Store::open(":memory:").unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let root = workspace.path().to_str().unwrap();
        let worker = store.register("worker", "codex", root, "worker").unwrap();
        let other = store.register("other", "codex", root, "other").unwrap();
        let exclusive = store
            .execute(
                &worker,
                "run-worker",
                &Operation::FileReserve {
                    paths: vec!["src/main.rs".into()],
                    mode: "exclusive".into(),
                    task_id: None,
                    attempt_id: None,
                    ttl_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(exclusive["reservation_ids"].as_array().unwrap().len(), 1);
        assert!(exclusive["expires_at"].as_u64().unwrap() > now());
        let conflict = store
            .execute(
                &other,
                "run-other",
                &Operation::FileReserve {
                    paths: vec!["src/main.rs".into()],
                    mode: "exclusive".into(),
                    task_id: None,
                    attempt_id: None,
                    ttl_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&conflict), "reservation_conflict");
        let shared_conflict = store
            .execute(
                &other,
                "run-other",
                &Operation::FileReserve {
                    paths: vec!["src/main.rs".into()],
                    mode: "shared".into(),
                    task_id: None,
                    attempt_id: None,
                    ttl_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&shared_conflict), "reservation_conflict");
        for actor in [&other, &worker] {
            store
                .execute(
                    actor,
                    "run-shared",
                    &Operation::FileReserve {
                        paths: vec!["docs/notes.md".into()],
                        mode: "shared".into(),
                        task_id: None,
                        attempt_id: None,
                        ttl_seconds: None,
                        request_id: Uuid::new_v4().to_string(),
                    },
                )
                .unwrap();
        }
        let batch = store
            .execute(
                &worker,
                "run-worker",
                &Operation::FileReserve {
                    paths: vec!["docs/a.md".into(), "docs/b.md".into()],
                    mode: "exclusive".into(),
                    task_id: None,
                    attempt_id: None,
                    ttl_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        let batch_ids: Vec<String> = batch["reservation_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(batch_ids.len(), 2);
        let partial = store
            .execute(
                &other,
                "run-other",
                &Operation::FileReserve {
                    paths: vec!["docs/a.md".into(), "other.txt".into()],
                    mode: "exclusive".into(),
                    task_id: None,
                    attempt_id: None,
                    ttl_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&partial), "reservation_conflict");
        let leaked = store
            .execute(
                &other,
                "run-other",
                &Operation::FileReservations {
                    path: Some("other.txt".into()),
                    cursor: None,
                    limit: None,
                    include_expired: false,
                },
            )
            .unwrap();
        assert!(leaked["reservations"].as_array().unwrap().is_empty());
        let renew_foreign = store
            .execute(
                &other,
                "run-other",
                &Operation::FileRenew {
                    reservation_ids: vec![batch_ids[0].clone()],
                    ttl_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&renew_foreign), "unauthorized");
        let released = store
            .execute(
                &worker,
                "run-worker",
                &Operation::FileRelease {
                    reservation_ids: batch_ids.clone(),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(released["released"].as_array().unwrap().len(), 2);
        let again = store
            .execute(
                &worker,
                "run-worker",
                &Operation::FileRelease {
                    reservation_ids: batch_ids,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert!(again["released"].as_array().unwrap().is_empty());
        // An expired lease tied to an active attempt survives the sweep with an
        // abandoned warning; once the attempt finishes the row is collected.
        let owning = store
            .execute(
                &other,
                "run-other",
                &assign(&other, "worker", None, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let owning_id = owning["id"].as_str().unwrap().to_owned();
        let started = store
            .execute(
                &worker,
                "run-worker",
                &transition(&owning_id, 1, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let attempt_id = started["attempt_id"].as_str().unwrap().to_owned();
        let short = store
            .execute(
                &worker,
                "run-worker",
                &Operation::FileReserve {
                    paths: vec!["tmp/y.rs".into()],
                    mode: "exclusive".into(),
                    task_id: Some(owning_id.clone()),
                    attempt_id: Some(attempt_id.clone()),
                    ttl_seconds: Some(1),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        let short_id = short["reservation_ids"][0].as_str().unwrap().to_owned();
        diesel::sql_query("UPDATE reservations SET expires_at = ? WHERE id = ?")
            .bind::<BigInt, _>((now() as i64) - 1_000)
            .bind::<Text, _>(&short_id)
            .execute(&mut *store.connection.borrow_mut())
            .unwrap();
        let expired_renew = store
            .execute(
                &worker,
                "run-worker",
                &Operation::FileRenew {
                    reservation_ids: vec![short_id.clone()],
                    ttl_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&expired_renew), "invalid_state");
        let visible = store
            .execute(
                &worker,
                "run-worker",
                &Operation::FileReservations {
                    path: Some("tmp/y.rs".into()),
                    cursor: None,
                    limit: None,
                    include_expired: true,
                },
            )
            .unwrap();
        assert_eq!(visible["reservations"][0]["expired"], true);
        assert_eq!(visible["reservations"][0]["abandoned"], true);
        store
            .execute(
                &worker,
                "run-worker",
                &submit(&owning_id, 1, Some(&attempt_id), &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let swept = store
            .execute(
                &worker,
                "run-worker",
                &Operation::FileReservations {
                    path: Some("tmp/y.rs".into()),
                    cursor: None,
                    limit: None,
                    include_expired: true,
                },
            )
            .unwrap();
        assert!(swept["reservations"].as_array().unwrap().is_empty());
    }

    #[test]
    fn evidence_is_attempt_scoped_and_operator_verified() {
        let store = Store::open(":memory:").unwrap();
        use sha2::{Digest, Sha256};
        let checkout = tempfile::tempdir().unwrap();
        let root = checkout.path().canonicalize().unwrap();
        let project = root.to_str().unwrap();
        std::fs::create_dir(root.join("src")).unwrap();
        let contents = b"pub fn answer() -> u8 { 42 }\n";
        std::fs::write(root.join("src/lib.rs"), contents).unwrap();
        let hash = format!("{:x}", Sha256::digest(contents));
        assert_eq!(code(&verify_file(project, ".env", Some(&hash)).unwrap_err()), "invalid_input");
        let oversized = std::fs::File::create(root.join("oversized.bin")).unwrap();
        oversized.set_len(64 * 1024 * 1024 + 1).unwrap();
        assert_eq!(code(&verify_file(project, "oversized.bin", Some(&hash)).unwrap_err()), "invalid_input");
        drop(oversized);
        let git = |args: &[&str]| {
            std::process::Command::new("git").args(["-C", project]).args(args)
                .stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null())
                .output().unwrap()
        };
        assert!(git(&["init", "--quiet", "--template="]).status.success());
        assert!(git(&["add", "src/lib.rs"]).status.success());
        assert!(git(&["-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
            "-c", "commit.gpgsign=false", "-c", "core.hooksPath=missing-hooks", "commit", "--quiet", "-m", "Fixture"]).status.success());
        let head = git(&["rev-parse", "HEAD"]);
        let head = std::str::from_utf8(&head.stdout).unwrap().trim();
        verify_commit(project, Some(head)).unwrap();
        assert_eq!(code(&verify_commit(project, Some("--bad")).unwrap_err()), "invalid_input");
        assert_eq!(code(&verify_commit(project, Some(&"0".repeat(40))).unwrap_err()), "invalid_state");
        let issuer = store.register("issuer", "custom", project, "issuer").unwrap();
        let worker = store.register("worker", "custom", project, "worker").unwrap();
        let task = store
            .execute(
                &issuer,
                "run-issuer",
                &assign(&issuer, "worker", None, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let task_id = task["id"].as_str().unwrap().to_owned();
        let started = store
            .execute(
                &worker,
                "run-worker",
                &transition(&task_id, 1, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let attempt_id = started["attempt_id"].as_str().unwrap().to_owned();
        let absolute = store
            .execute(
                &worker,
                "run-worker",
                &Operation::EvidenceAdd {
                    task_id: task_id.clone(),
                    kind: "file".into(),
                    attempt_id: Some(attempt_id.clone()),
                    path: Some("/etc/passwd".into()),
                    hash: Some(hash.clone()),
                    commit: None,
                    repository: None,
                    branch: None,
                    base: None,
                    head: None,
                    command: None,
                    outcome: None,
                    exit_code: None,
                    summary: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&absolute), "invalid_input");
        let file = store
            .execute(
                &worker,
                "run-worker",
                &Operation::EvidenceAdd {
                    task_id: task_id.clone(),
                    kind: "file".into(),
                    attempt_id: Some(attempt_id.clone()),
                    path: Some("src/lib.rs".into()),
                    hash: Some(hash.clone()),
                    commit: None,
                    repository: None,
                    branch: None,
                    base: None,
                    head: None,
                    command: None,
                    outcome: None,
                    exit_code: None,
                    summary: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(file["verified"], false);
        let file_id = file["evidence_id"].as_str().unwrap().to_owned();
        let test = store
            .execute(
                &worker,
                "run-worker",
                &Operation::EvidenceAdd {
                    task_id: task_id.clone(),
                    kind: "test".into(),
                    attempt_id: Some(attempt_id.clone()),
                    path: None,
                    hash: None,
                    commit: None,
                    repository: None,
                    branch: None,
                    base: None,
                    head: None,
                    command: Some("cargo test -p warp-agent-bus".into()),
                    outcome: Some("passed".into()),
                    exit_code: Some(0),
                    summary: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        let test_id = test["evidence_id"].as_str().unwrap().to_owned();
        let foreign_reference = store
            .execute(
                &worker,
                "run-worker",
                &Operation::TaskSubmit {
                    task_id: task_id.clone(),
                    revision: 1,
                    result: "done".into(),
                    evidence: "tests pass".into(),
                    expected_version: None,
                    attempt_id: Some(attempt_id.clone()),
                    evidence_ids: vec![Uuid::new_v4().to_string()],
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&foreign_reference), "scope_denied");
        let submitted = store
            .execute(
                &worker,
                "run-worker",
                &Operation::TaskSubmit {
                    task_id: task_id.clone(),
                    revision: 1,
                    result: "done".into(),
                    evidence: "tests pass".into(),
                    expected_version: None,
                    attempt_id: Some(attempt_id),
                    evidence_ids: vec![test_id.clone(), file_id.clone()],
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(submitted["state"], "submitted");
        assert_eq!(store.task(&issuer, &task_id).unwrap().evidence_records.len(), 2);
        let unverifiable = store
            .execute_controller(
                project,
                &ControllerOperation::EvidenceVerify {
                    evidence_id: test_id,
                    verified: true,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&unverifiable), "invalid_state");
        let unknown = store
            .execute_controller(
                project,
                &ControllerOperation::EvidenceVerify {
                    evidence_id: Uuid::new_v4().to_string(),
                    verified: true,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&unknown), "scope_denied");
        std::fs::write(root.join("src/lib.rs"), b"changed").unwrap();
        let mismatch = store.execute_controller(project, &ControllerOperation::EvidenceVerify {
            evidence_id: file_id.clone(), verified: true,
            request_id: Uuid::new_v4().to_string()
        }).unwrap_err();
        assert_eq!(code(&mismatch), "invalid_state");
        assert!(!store.task(&issuer, &task_id).unwrap().evidence_records.iter()
            .find(|evidence| evidence.id == file_id).unwrap().verified);
        std::fs::write(root.join("src/lib.rs"), contents).unwrap();
        let verified = store
            .execute_controller(
                project,
                &ControllerOperation::EvidenceVerify {
                    evidence_id: file_id.clone(),
                    verified: true,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(verified["verified"], true);
        assert_eq!(verified["task_id"], task_id.as_str());
        let task = store.task(&issuer, &task_id).unwrap();
        assert!(task
            .evidence_records
            .iter()
            .find(|evidence| evidence.id == file_id)
            .unwrap()
            .verified);
    }

    #[test]
    fn threads_are_participant_scoped_and_search_is_literal() {
        let store = Store::open(":memory:").unwrap();
        let alice = actor(&store, "alice");
        let bob = actor(&store, "bob");
        let carol = actor(&store, "carol");
        let root = store
            .execute(
                &alice,
                "run-alice",
                &Operation::AgentSend {
                    to: "bob".into(),
                    body: "please run under_score checks".into(),
                    subject: None,
                    thread_id: None,
                    reply_to: None,
                    task_id: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        let root_id = root["id"].as_str().unwrap().to_owned();
        assert!(root["thread_id"].is_null());
        let reply = store
            .execute(
                &bob,
                "run-bob",
                &Operation::AgentSend {
                    to: "alice".into(),
                    body: "acknowledged".into(),
                    subject: None,
                    thread_id: None,
                    reply_to: Some(root_id.clone()),
                    task_id: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(reply["thread_id"], root_id.as_str());
        assert_eq!(reply["reply_to"], root_id.as_str());
        for (body, from) in [
            ("underX hidden marker", &carol),
            ("progress 100% complete", &alice),
            ("progress 1000 complete", &carol),
        ] {
            store
                .execute(
                    from,
                    "run-sender",
                    &Operation::AgentSend {
                        to: "bob".into(),
                        body: body.into(),
                        subject: None,
                        thread_id: None,
                        reply_to: None,
                        task_id: None,
                        request_id: Uuid::new_v4().to_string(),
                    },
                )
                .unwrap();
        }
        let thread = store
            .execute(
                &bob,
                "run-bob",
                &Operation::ThreadGet {
                    thread_id: root_id.clone(),
                    cursor: None,
                    limit: None,
                },
            )
            .unwrap();
        let messages = thread["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["body"], "please run under_score checks");
        assert_eq!(messages[1]["body"], "acknowledged");
        let outsider = store
            .execute(
                &carol,
                "run-carol",
                &Operation::ThreadGet {
                    thread_id: root_id.clone(),
                    cursor: None,
                    limit: None,
                },
            )
            .unwrap_err();
        assert_eq!(code(&outsider), "scope_denied");
        let operator_thread = store.execute(&Store::operator("/project"), OPERATOR_EPOCH,
            &Operation::ThreadGet { thread_id: root_id.clone(), cursor: None, limit: Some(1) }).unwrap();
        assert_eq!(operator_thread["messages"].as_array().unwrap().len(), 1);
        assert_eq!(operator_thread["messages"][0]["body"], "please run under_score checks");
        assert!(store.execute(&Store::operator("/unrelated"), OPERATOR_EPOCH,
            &Operation::ThreadGet { thread_id: root_id.clone(), cursor: None, limit: None }).unwrap()["messages"].as_array().unwrap().is_empty());
        assert!(store.register("forged", OPERATOR_PROGRAM, "/project", "operator").is_err());
        let operator_search = store.execute(&Store::operator("/project"), OPERATOR_EPOCH,
            &Operation::MessageSearch { query: "under_".into(), task_id: None, thread_id: None, cursor: None, limit: None }).unwrap();
        assert_eq!(operator_search["messages"].as_array().unwrap().len(), 1);
        assert!(store.execute(&Store::operator("/unrelated"), OPERATOR_EPOCH,
            &Operation::MessageSearch { query: "under_".into(), task_id: None, thread_id: None, cursor: None, limit: None }).unwrap()["messages"].as_array().unwrap().is_empty());
        let literal = store
            .execute(
                &bob,
                "run-bob",
                &Operation::MessageSearch {
                    query: "under_".into(),
                    task_id: None,
                    thread_id: None,
                    cursor: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(literal["messages"].as_array().unwrap().len(), 1);
        assert_eq!(literal["messages"][0]["body"], "please run under_score checks");
        let padded = store
            .execute(
                &bob,
                "run-bob",
                &Operation::MessageSearch {
                    query: "  under_score  ".into(),
                    task_id: None,
                    thread_id: None,
                    cursor: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(padded["messages"].as_array().unwrap().len(), 1);
        let percent = store
            .execute(
                &bob,
                "run-bob",
                &Operation::MessageSearch {
                    query: "%".into(),
                    task_id: None,
                    thread_id: None,
                    cursor: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(percent["messages"].as_array().unwrap().len(), 1);
        assert_eq!(percent["messages"][0]["body"], "progress 100% complete");
        let scoped = store
            .execute(
                &carol,
                "run-carol",
                &Operation::MessageSearch {
                    query: "under_score".into(),
                    task_id: None,
                    thread_id: None,
                    cursor: None,
                    limit: None,
                },
            )
            .unwrap();
        assert!(scoped["messages"].as_array().unwrap().is_empty());
        let empty = store
            .execute(
                &bob,
                "run-bob",
                &Operation::MessageSearch {
                    query: "   ".into(),
                    task_id: None,
                    thread_id: None,
                    cursor: None,
                    limit: None,
                },
            )
            .unwrap_err();
        assert_eq!(code(&empty), "invalid_input");
    }

    #[test]
    fn history_retains_unknown_attempts_even_outside_the_detail_window() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let assigned = store.execute(&issuer, "run-issuer", &assign(&issuer, "worker", None, &Uuid::new_v4().to_string())).unwrap();
        let id = assigned["id"].as_str().unwrap();
        store.execute(&worker, "run-worker", &transition(id, 1, &Uuid::new_v4().to_string())).unwrap();
        store.execute_controller("/project", &ControllerOperation::TaskForceCancel {
            task_id: id.into(), reason: "Accept uncertainty".into(), expected_version: None,
            request_id: Uuid::new_v4().to_string(),
        }).unwrap();
        // Hide the original uncertainty behind more than one detail page of completed attempts.
        for revision in 2..=18 {
            diesel::sql_query("INSERT INTO attempts (id, task_id, revision, owner, run, certainty, outcome, finished_at) VALUES (?, ?, ?, ?, 'fixture', 'finished', 'submitted', 1)")
                .bind::<Text, _>(Uuid::new_v4().to_string()).bind::<Text, _>(id)
                .bind::<Integer, _>(revision).bind::<Text, _>(&worker.id)
                .execute(&mut *store.connection.borrow_mut()).unwrap();
        }
        assert!(store.operator_task("/project", id).unwrap().attempts.iter().all(|a| a.finished_at.is_some()));
        let error = store.execute_controller("/project", &ControllerOperation::TaskArchive {
            task_id: id.into(), request_id: Uuid::new_v4().to_string(),
        }).unwrap_err();
        assert_eq!(code(&error), "execution_unknown");
        diesel::sql_query("UPDATE tasks SET updated_at = 0 WHERE id = ?")
            .bind::<Text, _>(id).execute(&mut *store.connection.borrow_mut()).unwrap();
        let aged = store.execute_controller("/project", &ControllerOperation::ArchiveAged {
            older_than_days: Some(1), request_id: Uuid::new_v4().to_string(),
        }).unwrap();
        assert!(aged["archived"].as_array().unwrap().is_empty());
        // Rows archived by an older binary must also survive the new purge guard.
        diesel::sql_query("UPDATE tasks SET archived = 1 WHERE id = ?")
            .bind::<Text, _>(id).execute(&mut *store.connection.borrow_mut()).unwrap();
        diesel::sql_query("UPDATE messages SET acknowledged = 1 WHERE task_id = ?")
            .bind::<Text, _>(id).execute(&mut *store.connection.borrow_mut()).unwrap();
        assert_eq!(store.purge_preview("/project").unwrap()["tasks"], 0);
        let purge = store.history_purge("/project", &Store::operator("/project"), true, true).unwrap();
        assert!(purge["purged_tasks"].as_array().unwrap().is_empty());
        assert_eq!(store.count("SELECT COUNT(*) AS count FROM attempts WHERE task_id = ?", &[id]).unwrap(), 18);
    }

    #[test]
    fn purge_preserves_thread_roots_and_cross_task_replies() {
        let store = Store::open(":memory:").unwrap();
        let alice = actor(&store, "alice");
        let bob = actor(&store, "bob");
        let assigned = store.execute(&alice, "run-alice", &assign(&alice, "bob", None, &Uuid::new_v4().to_string())).unwrap();
        let task_id = assigned["id"].as_str().unwrap();
        let root = store.execute(&alice, "run-alice", &Operation::AgentSend {
            to: "bob".into(), body: "original task context".into(), subject: None,
            thread_id: None, reply_to: None, task_id: Some(task_id.into()),
            request_id: Uuid::new_v4().to_string(),
        }).unwrap();
        let root_id = root["id"].as_str().unwrap();
        let reply = store.execute(&bob, "run-bob", &Operation::AgentSend {
            to: "alice".into(), body: "retained correction".into(), subject: None,
            thread_id: None, reply_to: Some(root_id.into()), task_id: None,
            request_id: Uuid::new_v4().to_string(),
        }).unwrap();
        // Model a legacy archived task with acknowledged parents and an unread standalone reply.
        diesel::sql_query("UPDATE tasks SET archived = 1, state = 'cancelled' WHERE id = ?")
            .bind::<Text, _>(task_id).execute(&mut *store.connection.borrow_mut()).unwrap();
        diesel::sql_query("UPDATE messages SET acknowledged = 1 WHERE task_id = ?")
            .bind::<Text, _>(task_id).execute(&mut *store.connection.borrow_mut()).unwrap();
        assert_eq!(store.purge_preview("/project").unwrap()["tasks"], 0);
        assert!(store.history_purge("/project", &Store::operator("/project"), true, true).unwrap()["purged_tasks"].as_array().unwrap().is_empty());
        // The same references protect standalone acknowledged roots after task linkage is removed.
        diesel::sql_query("UPDATE messages SET task_id = NULL WHERE id = ?")
            .bind::<Text, _>(root_id).execute(&mut *store.connection.borrow_mut()).unwrap();
        assert_eq!(store.purge_preview("/project").unwrap()["messages"], 0);
        diesel::sql_query("UPDATE messages SET acknowledged = 1 WHERE id = ?")
            .bind::<Text, _>(reply["id"].as_str().unwrap()).execute(&mut *store.connection.borrow_mut()).unwrap();
        assert_eq!(store.purge_preview("/project").unwrap()["messages"], 1);
        assert_eq!(store.history_purge("/project", &Store::operator("/project"), false, true).unwrap()["purged_messages"], 1);
        let thread = store.thread_get(&alice, root_id, None, None).unwrap();
        assert_eq!(thread["messages"].as_array().unwrap().len(), 1);
        assert_eq!(thread["messages"][0]["body"], "original task context");
    }

    #[test]
    fn purge_confirmation_is_fenced_and_replay_preserves_its_original_result() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        actor(&store, "worker");
        let sequence = store.purge_preview("/project").unwrap()["sequence"].as_u64().unwrap();
        store.execute(&issuer, "run-issuer", &send(&issuer, "worker", "new unread work", &Uuid::new_v4().to_string())).unwrap();
        let old = ControllerOperation::HistoryPurge { expected_sequence: Some(sequence), archived_tasks: true, acknowledged_messages: true, request_id: Uuid::new_v4().to_string() };
        assert_eq!(code(&store.execute_controller("/project", &old).unwrap_err()), "version_conflict");
        let current = store.purge_preview("/project").unwrap()["sequence"].as_u64().unwrap();
        let confirmed = ControllerOperation::HistoryPurge { expected_sequence: Some(current), archived_tasks: true, acknowledged_messages: true, request_id: Uuid::new_v4().to_string() };
        let result = store.execute_controller("/project", &confirmed).unwrap();
        let after = store.history_sequence("/project").unwrap();
        assert!(after > current);
        assert_eq!(store.execute_controller("/project", &confirmed).unwrap(), result);
        assert_eq!(store.history_sequence("/project").unwrap(), after);
    }

    #[test]
    fn history_export_is_ordered_and_purge_preserves_unread_work() {
        let store = Store::open(":memory:").unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let task = store
            .execute(
                &issuer,
                "run-issuer",
                &assign(&issuer, "worker", None, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let task_id = task["id"].as_str().unwrap().to_owned();
        store
            .execute(
                &issuer,
                "run-issuer",
                &send(&issuer, "worker", "hello", &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let first = store
            .execute_controller(
                "/project",
                &ControllerOperation::HistoryExport {
                    after: None,
                    limit: Some(3),
                },
            )
            .unwrap();
        assert_eq!(first["records"].as_array().unwrap().len(), 3);
        let cursor = first["cursor"].as_u64().unwrap();
        let second = store
            .execute_controller(
                "/project",
                &ControllerOperation::HistoryExport {
                    after: Some(cursor),
                    limit: Some(3),
                },
            )
            .unwrap();
        // Assignment and send each consume a task/message record plus its
        // message_queued event, six records total.
        assert_eq!(second["records"].as_array().unwrap().len(), 3);
        let started = store
            .execute(
                &worker,
                "run-worker",
                &transition(&task_id, 1, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let attempt_id = started["attempt_id"].as_str().unwrap().to_owned();
        store
            .execute(
                &worker,
                "run-worker",
                &submit(&task_id, 1, Some(&attempt_id), &Uuid::new_v4().to_string()),
            )
            .unwrap();
        store
            .execute(
                &issuer,
                "run-issuer",
                &review(&task_id, 1, true, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let records = export_all(&store);
        let sequences: Vec<u64> = records
            .iter()
            .map(|record| record["sequence"].as_u64().unwrap())
            .collect();
        assert!(sequences.windows(2).all(|window| window[0] < window[1]));
        assert!(records.iter().any(|record| record["type"] == "task"));
        assert!(records.iter().any(|record| record["type"] == "event"));
        let accepted = store
            .pending(&worker)
            .unwrap()
            .into_iter()
            .find(|message| {
                message.task_id.as_deref() == Some(task_id.as_str()) && message.kind == "accepted"
            })
            .unwrap();
        store
            .execute(
                &worker,
                "run-worker",
                &Operation::AgentAck {
                    message_id: accepted.id,
                },
            )
            .unwrap();
        let hello = store
            .pending(&worker)
            .unwrap()
            .into_iter()
            .find(|message| message.body == "hello")
            .unwrap();
        store
            .execute(
                &worker,
                "run-worker",
                &Operation::AgentAck {
                    message_id: hello.id,
                },
            )
            .unwrap();
        let archive = store
            .execute_controller(
                "/project",
                &ControllerOperation::TaskArchive {
                    task_id: task_id.clone(),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(archive["archived"], true);
        let preview = store
            .execute_controller("/project", &ControllerOperation::PurgePreview)
            .unwrap();
        assert_eq!(preview["tasks"], 1);
        assert_eq!(preview["messages"], 1);
        let purged = store
            .execute_controller(
                "/project",
                &ControllerOperation::HistoryPurge { expected_sequence: None,
                    archived_tasks: true,
                    acknowledged_messages: true,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(purged["purged_tasks"], json!([task_id.clone()]));
        assert_eq!(purged["purged_messages"], 1);
        assert_eq!(
            code(&store
                .execute(
                    &issuer,
                    "run-issuer",
                    &Operation::TaskGet {
                        task_id: task_id.clone(),
                    },
                )
                .unwrap_err()),
            "scope_denied"
        );
        let records = export_all(&store);
        assert!(!records
            .iter()
            .any(|record| record["data"]["id"] == task_id.as_str()));
        let pending_task = store
            .execute(
                &issuer,
                "run-issuer",
                &assign(&issuer, "worker", None, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let pending_id = pending_task["id"].as_str().unwrap().to_owned();
        store
            .execute_controller(
                "/project",
                &ControllerOperation::TaskForceCancel {
                    task_id: pending_id.clone(),
                    reason: "changed plans".into(),
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        store
            .execute_controller(
                "/project",
                &ControllerOperation::TaskArchive {
                    task_id: pending_id.clone(),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        let preview = store
            .execute_controller("/project", &ControllerOperation::PurgePreview)
            .unwrap();
        assert_eq!(preview["tasks"], 0);
        let purged = store
            .execute_controller(
                "/project",
                &ControllerOperation::HistoryPurge { expected_sequence: None,
                    archived_tasks: true,
                    acknowledged_messages: false,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert!(purged["purged_tasks"].as_array().unwrap().is_empty());
        assert_eq!(store.task(&issuer, &pending_id).unwrap().state, "cancelled");
        let third = store
            .execute(
                &issuer,
                "run-issuer",
                &assign(&issuer, "worker", None, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        let third_id = third["id"].as_str().unwrap().to_owned();
        let started = store
            .execute(
                &worker,
                "run-worker",
                &transition(&third_id, 1, &Uuid::new_v4().to_string()),
            )
            .unwrap();
        assert_eq!(started["state"], "running");
        let cancel = store
            .execute(
                &issuer,
                "run-issuer",
                &Operation::TaskCancel {
                    task_id: third_id.clone(),
                    reason: "scope changed".into(),
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(cancel["state"], "cancel_requested");
        let stopped = store
            .execute(
                &worker,
                "run-worker",
                &Operation::TaskFinishCancel {
                    task_id: third_id.clone(),
                    revision: 1,
                    reason: "stopped".into(),
                    attempt_id: None,
                    expected_version: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(stopped["state"], "cancelled");
        let cancelled = store
            .pending(&worker)
            .unwrap()
            .into_iter()
            .find(|message| {
                message.task_id.as_deref() == Some(third_id.as_str()) && message.kind == "cancelled"
            })
            .unwrap();
        store
            .execute(
                &worker,
                "run-worker",
                &Operation::AgentAck {
                    message_id: cancelled.id,
                },
            )
            .unwrap();
        diesel::sql_query("UPDATE tasks SET updated_at = ? WHERE id = ?")
            .bind::<BigInt, _>((now() as i64) - 2 * 24 * 60 * 60 * 1000)
            .bind::<Text, _>(&third_id)
            .execute(&mut *store.connection.borrow_mut())
            .unwrap();
        let invalid_days = store
            .execute_controller(
                "/project",
                &ControllerOperation::ArchiveAged {
                    older_than_days: Some(0),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&invalid_days), "invalid_input");
        let aged = store
            .execute_controller(
                "/project",
                &ControllerOperation::ArchiveAged {
                    older_than_days: Some(1),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(aged["archived"], json!([third_id.clone()]));
        let preview = store
            .execute_controller("/project", &ControllerOperation::PurgePreview)
            .unwrap();
        assert_eq!(preview["tasks"], 1);
        let purged = store
            .execute_controller(
                "/project",
                &ControllerOperation::HistoryPurge { expected_sequence: None,
                    archived_tasks: true,
                    acknowledged_messages: false,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(purged["purged_tasks"], json!([third_id]));
    }

    #[test]
    fn reservation_overlap_requires_explicit_scope_and_current_membership() {
        let store = Store::open(":memory:").unwrap();
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let first_root = first.path().canonicalize().unwrap();
        let second_root = second.path().canonicalize().unwrap();
        let left = store
            .register("left", "codex", first_root.to_str().unwrap(), "left")
            .unwrap();
        let right = store
            .register("right", "codex", second_root.to_str().unwrap(), "right")
            .unwrap();
        let reserve = |agent: &Agent, paths: Vec<String>, mode: &str| {
            store.execute(
                agent,
                "run",
                &Operation::FileReserve {
                    paths,
                    mode: mode.into(),
                    task_id: None,
                    attempt_id: None,
                    ttl_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
        };
        reserve(&right, vec!["private.txt".into()], "exclusive").unwrap();
        let space = store
            .execute_controller(
                &left.project,
                &ControllerOperation::SpaceCreate {
                    name: "Worktrees".into(),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap()["space_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let repository = Uuid::new_v4().to_string();
        for agent in [&left, &right] {
            store
                .execute_controller(
                    &agent.project,
                    &ControllerOperation::WorkspaceMap {
                        space_id: space.clone(),
                        root: agent.project.clone(),
                        repository_id: Some(repository.clone()),
                        model: agent.program.clone(),
                        branch: None,
                        base_commit: None,
                        request_id: Uuid::new_v4().to_string(),
                    },
                )
                .unwrap();
        }
        let membership = |agent: &Agent, join: bool| {
            let operation = if join {
                ControllerOperation::SpaceJoin {
                    space_id: space.clone(),
                    agent: agent.name.clone(),
                    request_id: Uuid::new_v4().to_string(),
                }
            } else {
                ControllerOperation::SpaceLeave {
                    space_id: space.clone(),
                    agent: agent.name.clone(),
                    request_id: Uuid::new_v4().to_string(),
                }
            };
            store
                .execute_controller(&agent.project, &operation)
                .unwrap();
        };
        membership(&left, true);
        let pre_join = reserve(&right, vec!["src/main.rs".into()], "exclusive").unwrap();
        assert!(pre_join["overlap_warnings"].as_array().unwrap().is_empty());
        membership(&right, true);
        let hidden = reserve(&left, vec!["private.txt".into(), "src".into()], "exclusive").unwrap();
        assert!(hidden["overlap_warnings"].as_array().unwrap().is_empty());
        store
            .execute(
                &right,
                "run",
                &Operation::FileRelease {
                    reservation_ids: vec![pre_join["reservation_ids"][0].as_str().unwrap().into()],
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        let overlapping = reserve(&right, vec!["src/main.rs".into()], "exclusive").unwrap();
        assert_eq!(overlapping["reservation_ids"].as_array().unwrap().len(), 1);
        assert_eq!(overlapping["overlap_warnings"].as_array().unwrap().len(), 1);
        assert_eq!(overlapping["overlap_warnings"][0]["path"], "src");
        assert_eq!(overlapping["overlap_warnings"][0]["kind"], "merge_overlap");
        assert_eq!(overlapping["warnings_truncated"], false);
        let physical = reserve(&left, vec!["src/main.rs".into()], "exclusive").unwrap_err();
        assert_eq!(code(&physical), "reservation_conflict");
        for agent in [&left, &right] {
            let shared = reserve(agent, vec!["readme.txt".into()], "shared").unwrap();
            assert!(shared["overlap_warnings"].as_array().unwrap().is_empty());
        }
        reserve(
            &right,
            (0..51).map(|index| format!("bulk/file-{index}")).collect(),
            "exclusive",
        )
        .unwrap();
        let bounded = reserve(&left, vec!["bulk".into()], "exclusive").unwrap();
        assert_eq!(
            bounded["overlap_warnings"].as_array().unwrap().len(),
            PAGE_DEFAULT as usize
        );
        assert_eq!(bounded["warnings_truncated"], true);
        reserve(&right, vec!["expired.txt".into()], "exclusive").unwrap();
        diesel::sql_query(
            "UPDATE reservations SET expires_at = 1 WHERE workspace = ? AND path = 'expired.txt'",
        )
        .bind::<Text, _>(&right.project)
        .execute(&mut *store.connection.borrow_mut())
        .unwrap();
        let expired = reserve(&left, vec!["expired.txt".into()], "exclusive").unwrap();
        assert!(expired["overlap_warnings"].as_array().unwrap().is_empty());
        membership(&left, false);
        let departed = reserve(&right, vec!["src/another.rs".into()], "exclusive").unwrap();
        assert!(departed["overlap_warnings"].as_array().unwrap().is_empty());
        // Leaving a space does not remove its owner's physical coordination lease.
        assert_eq!(
            code(&reserve(&left, vec!["src/another.rs".into()], "exclusive").unwrap_err()),
            "reservation_conflict"
        );
        membership(&left, true);
        store
            .execute_controller(
                &left.project,
                &ControllerOperation::WorkspaceMap {
                    space_id: space,
                    root: left.project.clone(),
                    repository_id: Some(Uuid::new_v4().to_string()),
                    model: "codex".into(),
                    branch: None,
                    base_commit: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        let unrelated = reserve(&right, vec!["src/unrelated.rs".into()], "exclusive").unwrap();
        assert!(unrelated["overlap_warnings"].as_array().unwrap().is_empty());
    }

    #[test]
    fn v3_workspace_upgrade_preserves_private_reservations_and_backup() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bus.sqlite");
        let path = path.to_str().unwrap();
        let store = Store::open(path).unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let root = workspace.path().canonicalize().unwrap();
        let worker = store
            .register("worker", "codex", root.to_str().unwrap(), "worker")
            .unwrap();
        store
            .execute(
                &worker,
                "run",
                &Operation::FileReserve {
                    paths: vec!["keep.txt".into()],
                    mode: "exclusive".into(),
                    task_id: None,
                    attempt_id: None,
                    ttl_seconds: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        store
            .connection
            .borrow_mut()
            .batch_execute(
                "DROP TABLE agent_workspace_bindings;
             DROP INDEX workspaces_repository;
             ALTER TABLE workspaces DROP COLUMN repository_id;
             ALTER TABLE reservations DROP COLUMN space_id;
             ALTER TABLE reservations DROP COLUMN repository_id;
             ALTER TABLE reservations DROP COLUMN workspace_id;
             UPDATE meta SET value = '3' WHERE key = 'schema_version';",
            )
            .unwrap();
        store.set_legacy_payload(SENTINEL_V3).unwrap();
        drop(store);
        let upgraded = Store::open(path).unwrap();
        assert_eq!(
            upgraded.meta_version().unwrap().as_deref(),
            Some(SCHEMA_VERSION)
        );
        assert_eq!(
            read_legacy_payload(&format!("{path}.pre-upgrade-v3"))
                .unwrap()
                .as_deref(),
            Some(SENTINEL_V3)
        );
        assert_eq!(upgraded.count("SELECT COUNT(*) AS count FROM reservations WHERE space_id IS NULL AND repository_id IS NULL AND workspace_id IS NULL", &[]).unwrap(), 1);
        let existing = upgraded
            .execute(
                &worker,
                "run",
                &Operation::FileReservations {
                    path: None,
                    cursor: None,
                    limit: None,
                    include_expired: false,
                },
            )
            .unwrap();
        assert_eq!(existing["reservations"][0]["path"], "keep.txt");
        drop(upgraded);
        assert!(Store::open(path).is_ok());
    }

    #[test]
    fn spaces_workspaces_and_devices_are_operator_managed() {
        let store = Store::open(":memory:").unwrap();
        actor(&store, "worker");
        let created = store
            .execute_controller(
                "/project",
                &ControllerOperation::SpaceCreate {
                    name: "Alpha".into(),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(created["name"], "Alpha");
        assert_eq!(created["device"], "local");
        assert_eq!(created["private"], false);
        let space_id = created["space_id"].as_str().unwrap().to_owned();
        let duplicate = store
            .execute_controller(
                "/project",
                &ControllerOperation::SpaceCreate {
                    name: "Alpha".into(),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&duplicate), "invalid_input");
        let reserved = store
            .execute_controller(
                "/project",
                &ControllerOperation::SpaceCreate {
                    name: "private".into(),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&reserved), "invalid_input");
        store
            .execute_controller(
                "/project",
                &ControllerOperation::SpaceJoin {
                    space_id: space_id.clone(),
                    agent: "worker".into(),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        let joined = store
            .execute_controller("/project", &ControllerOperation::SpaceList { cursor: None, limit: None })
            .unwrap();
        let spaces = joined["spaces"].as_array().unwrap();
        assert!(spaces[0]["id"].is_null());
        assert_eq!(spaces[0]["private"], true);
        assert_eq!(spaces[1]["name"], "Alpha");
        assert_eq!(spaces[1]["members"][0], "worker");
        let unknown = store
            .execute_controller(
                "/project",
                &ControllerOperation::SpaceJoin {
                    space_id: Uuid::new_v4().to_string(),
                    agent: "worker".into(),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&unknown), "scope_denied");
        store
            .execute_controller(
                "/project",
                &ControllerOperation::SpaceLeave {
                    space_id: space_id.clone(),
                    agent: "worker".into(),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        let left = store
            .execute_controller("/project", &ControllerOperation::SpaceList { cursor: None, limit: None })
            .unwrap();
        assert!(left["spaces"][1]["members"].as_array().unwrap().is_empty());
        let missing = store
            .execute_controller(
                "/project",
                &ControllerOperation::SpaceLeave {
                    space_id: space_id.clone(),
                    agent: "worker".into(),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&missing), "invalid_state");
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_str().unwrap().to_owned();
        let mapped = store
            .execute_controller(
                "/project",
                &ControllerOperation::WorkspaceMap {
                    space_id: space_id.clone(),
                    root: root.clone(),
                    repository_id: None,
                    model: "claude".into(),
                    branch: Some("main".into()),
                    base_commit: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        let first_id = mapped["workspace_id"].as_str().unwrap().to_owned();
        assert_eq!(
            mapped["root"],
            std::fs::canonicalize(&root).unwrap().to_str().unwrap()
        );
        assert_eq!(mapped["model"], "claude");
        let remapped = store
            .execute_controller(
                "/project",
                &ControllerOperation::WorkspaceMap {
                    space_id: space_id.clone(),
                    root: root.clone(),
                    repository_id: None,
                    model: "codex".into(),
                    branch: None,
                    base_commit: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(remapped["workspace_id"], first_id.as_str());
        assert_eq!(remapped["model"], "codex");
        let invalid = store
            .execute_controller(
                "/project",
                &ControllerOperation::WorkspaceMap {
                    space_id: space_id.clone(),
                    root: "/definitely/not/a/directory".into(),
                    repository_id: None,
                    model: "claude".into(),
                    branch: None,
                    base_commit: None,
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(code(&invalid), "invalid_input");
        let devices = store.execute_controller("/project", &ControllerOperation::DeviceList).unwrap_err();
        assert_eq!(code(&devices), "feature_unavailable");
    }

    #[test]
    fn v6_cutover_keeps_uncertain_work_and_original_intents_without_device_runtime() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("bus.sqlite");
        let path = database.to_str().unwrap();
        let store = Store::open(path).unwrap();
        let worker = store.register("legacy", "codex", "/legacy", "legacy-worker").unwrap();
        let local = store.register("local", "codex", "/local", "local-worker").unwrap();
        let task = store.execute(&Store::operator("/legacy"), OPERATOR_EPOCH, &Operation::TaskAssign {
            to: worker.name.clone(), description: "Legacy work".into(), acceptance: "Preserve uncertainty".into(),
            reviewer: None, dependencies: vec![], start_deadline: None,
            execution_timeout_seconds: None, review_timeout_seconds: None, request_id: Uuid::new_v4().to_string(),
        }).unwrap();
        let task_id = task["id"].as_str().unwrap();
        let start = transition(task_id, 1, &Uuid::new_v4().to_string());
        let receipt = store.execute(&worker, "legacy-run", &start).unwrap();
        let before = store.operator_task("/legacy", task_id).unwrap();
        diesel::sql_query("INSERT INTO remote_actor_bindings VALUES (?, 'old-device', 'old-workspace', 'old-space', 'old-native', 'legacy-run', 0)")
            .bind::<Text, _>(&worker.id).execute(&mut *store.connection.borrow_mut()).unwrap();
        store.connection.borrow_mut().batch_execute(
            "INSERT INTO devices VALUES ('old-device','Legacy','unused',1,0,0);
             INSERT INTO remote_runs VALUES ('legacy-run','old-actor','old-native-run',0,0);
             INSERT INTO invitations VALUES ('old-invite','unused','[]',0,0);
             INSERT INTO remote_pending_intents VALUES ('old-coordinator','old-device','old-space','old-actor','old-run','old-request','original intent',NULL,0);
             UPDATE meta SET value='6' WHERE key='schema_version';").unwrap();
        store.set_legacy_payload(SENTINEL_V6).unwrap();
        drop(store);
        let upgraded = Store::open(path).unwrap();
        let after = upgraded.operator_task("/legacy", task_id).unwrap();
        assert_eq!(after.state, "running");
        assert_eq!(after.description, before.description);
        assert_eq!(after.version, before.version + 1);
        assert_eq!(after.attempts[0].certainty, "unknown");
        assert!(after.attempts[0].finished_at.is_none());
        assert_eq!(upgraded.count("SELECT COUNT(*) AS count FROM devices WHERE revoked=0", &[]).unwrap(), 0);
        assert_eq!(upgraded.count("SELECT COUNT(*) AS count FROM remote_actor_bindings WHERE revoked=0", &[]).unwrap(), 0);
        assert_eq!(upgraded.count("SELECT COUNT(*) AS count FROM remote_runs WHERE closed=0", &[]).unwrap(), 0);
        assert_eq!(upgraded.count("SELECT COUNT(*) AS count FROM invitations WHERE consumed=0", &[]).unwrap(), 0);
        assert_eq!(upgraded.count("SELECT COUNT(*) AS count FROM remote_pending_intents WHERE operation='original intent' AND response IS NULL", &[]).unwrap(), 1);
        let saved = diesel::sql_query("SELECT response AS value FROM requests WHERE actor=? AND request_id=?")
            .bind::<Text, _>(&worker.id).bind::<Text, _>(start.request_id().unwrap())
            .get_result::<ValueRow>(&mut *upgraded.connection.borrow_mut()).unwrap().value;
        assert_eq!(serde_json::from_str::<Value>(&saved).unwrap(), receipt);
        assert!(upgraded.execute(&local, "local-run", &Operation::AgentList).is_ok());
        assert_eq!(read_legacy_payload(&format!("{path}.pre-upgrade-v6")).unwrap().as_deref(), Some(SENTINEL_V6));
        drop(upgraded);
        assert_eq!(Store::open(path).unwrap().operator_task("/legacy", task_id).unwrap().version, after.version);
    }

    #[test]
    fn v4_upgrade_preserves_private_attempts_and_creates_no_shared_admissions() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bus.sqlite");
        let path = path.to_str().unwrap();
        let store = Store::open(path).unwrap();
        let issuer = actor(&store, "issuer");
        let worker = actor(&store, "worker");
        let assigned = store.execute(&issuer, "issuer-run", &assign(&issuer, "worker", None, &Uuid::new_v4().to_string())).unwrap();
        let task_id = assigned["id"].as_str().unwrap();
        let started = store.execute(&worker, "worker-run", &transition(task_id, 1, &Uuid::new_v4().to_string())).unwrap();
        let before = json!(store.task(&worker, task_id).unwrap());
        store.connection.borrow_mut().batch_execute("DROP TABLE agent_workspace_bindings; UPDATE meta SET value='4' WHERE key='schema_version';").unwrap();
        store.set_legacy_payload(SENTINEL_V4).unwrap();
        drop(store);
        let upgraded = Store::open(path).unwrap();
        assert_eq!(upgraded.meta_version().unwrap().as_deref(), Some(SCHEMA_VERSION));
        assert_eq!(json!(upgraded.task(&worker, task_id).unwrap()), before);
        assert_eq!(upgraded.count("SELECT COUNT(*) AS count FROM agent_workspace_bindings", &[]).unwrap(), 0);
        assert_eq!(read_legacy_payload(&format!("{path}.pre-upgrade-v4")).unwrap().as_deref(), Some(SENTINEL_V4));
        assert_eq!(read_legacy_payload(path).unwrap().as_deref(), Some(SENTINEL));
        assert_eq!(upgraded.execute(&worker, "worker-run", &transition(task_id, 1, &Uuid::new_v4().to_string())).unwrap_err().downcast_ref::<DomainError>().unwrap().code, "invalid_state");
        assert!(started["attempt_id"].is_string());
        assert!(upgraded.register("untrusted", "codex", &format!("space:{}", Uuid::new_v4()), "untrusted").is_err());
    }

    fn v2_database(path: &str) {
        let mut connection = SqliteConnection::establish(path).unwrap();
        connection
            .batch_execute(
                "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO meta(key, value) VALUES ('schema_version', '2');
                 CREATE TABLE agent_bus_v1 (id INTEGER PRIMARY KEY CHECK(id=1), payload TEXT NOT NULL);
                 CREATE TABLE agents (id TEXT PRIMARY KEY, terminal TEXT NOT NULL, name TEXT NOT NULL, program TEXT NOT NULL, project TEXT NOT NULL, UNIQUE(project, name));
                 CREATE TABLE tasks (id TEXT PRIMARY KEY, project TEXT NOT NULL, issuer TEXT NOT NULL, assignee TEXT NOT NULL, reviewer TEXT NOT NULL, description TEXT NOT NULL, acceptance TEXT NOT NULL, state TEXT NOT NULL, revision INTEGER NOT NULL, version INTEGER NOT NULL, result TEXT, evidence TEXT, created_seq INTEGER NOT NULL);
                 CREATE TABLE messages (id TEXT PRIMARY KEY, project TEXT NOT NULL, sender TEXT NOT NULL, recipient TEXT NOT NULL, body TEXT NOT NULL, subject TEXT, thread_id TEXT, reply_to TEXT, task_id TEXT, revision INTEGER, kind TEXT NOT NULL, acknowledged INTEGER NOT NULL, sequence INTEGER NOT NULL);
                 CREATE TABLE events (id TEXT PRIMARY KEY, project TEXT NOT NULL, sequence INTEGER NOT NULL, kind TEXT NOT NULL, actor TEXT NOT NULL, resource TEXT, attempt TEXT, observed_at INTEGER, imported INTEGER NOT NULL, payload TEXT NOT NULL);
                 CREATE TABLE sequences (project TEXT PRIMARY KEY, value INTEGER NOT NULL);
                 CREATE TABLE requests (actor TEXT NOT NULL, request_id TEXT NOT NULL, epoch TEXT NOT NULL, fingerprint TEXT NOT NULL, response TEXT NOT NULL, created_at INTEGER NOT NULL, PRIMARY KEY(actor, request_id));",
            )
            .unwrap();
        diesel::sql_query("INSERT INTO agent_bus_v1(id, payload) VALUES (1, ?)")
            .bind::<Text, _>(SENTINEL_V2)
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("INSERT INTO agents(id, terminal, name, program, project) VALUES ('agent-old', 't-old', 'old', 'claude', '/p')")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("INSERT INTO tasks(id, project, issuer, assignee, reviewer, description, acceptance, state, revision, version, result, evidence, created_seq) VALUES ('task-v2', '/p', 'agent-old', 'agent-old', 'agent-old', 'old work', 'old acceptance', 'accepted', 1, 1, 'ok', 'ran', 1)")
            .execute(&mut connection)
            .unwrap();
    }

    #[test]
    fn v2_databases_upgrade_additively_with_backup() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bus.sqlite");
        let path = path.to_str().unwrap();
        let v1_backup = format!("{path}.pre-upgrade");
        let v1_payload = legacy_payload();
        legacy_database(&v1_backup, &v1_payload);
        v2_database(path);
        let store = Store::open(path).unwrap();
        let backup = format!("{path}.pre-upgrade-v2");
        assert!(std::path::Path::new(&backup).exists());
        assert_eq!(read_legacy_payload(&v1_backup).unwrap().as_deref(), Some(v1_payload.as_str()));
        assert_eq!(read_legacy_payload(path).unwrap().as_deref(), Some(SENTINEL));
        assert_eq!(
            read_legacy_payload(&backup).unwrap().as_deref(),
            Some(SENTINEL_V2)
        );
        let old = store.register("t-old", "claude", "/p", "old").unwrap();
        assert_eq!(old.id, "agent-old");
        let preserved = store.task(&old, "task-v2").unwrap();
        assert_eq!(preserved.state, "accepted");
        assert_eq!(preserved.result.as_deref(), Some("ok"));
        assert!(!preserved.archived);
        let archived = store
            .execute_controller(
                "/p",
                &ControllerOperation::TaskArchive {
                    task_id: "task-v2".into(),
                    request_id: Uuid::new_v4().to_string(),
                },
            )
            .unwrap();
        assert_eq!(archived["archived"], true);
        let other = store.register("t2", "claude", "/p", "other").unwrap();
        let sent = store
            .execute(
                &old,
                "run-old",
                &send(&old, "other", "post-upgrade", &Uuid::new_v4().to_string()),
            )
            .unwrap();
        assert_eq!(sent["body"], "post-upgrade");
        assert_eq!(store.pending(&other).unwrap().len(), 1);
        drop(store);
        let store = Store::open(path).unwrap();
        assert!(store.task(&old, "task-v2").unwrap().archived);
    }
}
