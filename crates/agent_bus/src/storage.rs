//! Versioned, normalized SQLite storage: one write owner, short transactions, events with mutations.
use crate::{
    capacity_exceeded, epoch_expired, invalid_input, invalid_state, request_conflict, scope_denied,
    stale_attempt, stale_revision, subject as validate_subject, text, unauthorized, version_conflict,
    Agent, Attempt, Event, Message, Operation, Task, PAGE_DEFAULT, PAGE_MAX,
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
    collections::BTreeMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

pub(crate) const SCHEMA_VERSION: &str = "2";
/// The v1 loader ignores `user_version`; this payload makes the old deserializer fail instead of silently writing.
pub(crate) const SENTINEL: &str = r#"{"warp_lite_schema_version":2}"#;
/// Must exceed the maximum mutation epoch so cleanup can never make an old request execute again.
pub(crate) const REQUEST_RETENTION: Duration = Duration::from_secs(8 * 24 * 60 * 60);
const MAX_AGENTS: i64 = 1000;
const MAX_TASKS_PER_PROJECT: i64 = 10_000;
const MAX_PENDING_PER_PROJECT: i64 = 1000;
const MAX_MESSAGES_PER_PROJECT: i64 = 100_000;
const MAX_REQUESTS: i64 = 10_000;

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS agent_bus_v1 (id INTEGER PRIMARY KEY CHECK(id=1), payload TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS agents (id TEXT PRIMARY KEY, terminal TEXT NOT NULL, name TEXT NOT NULL, program TEXT NOT NULL, project TEXT NOT NULL, UNIQUE(project, name));
CREATE TABLE IF NOT EXISTS tasks (id TEXT PRIMARY KEY, project TEXT NOT NULL, issuer TEXT NOT NULL, assignee TEXT NOT NULL, reviewer TEXT NOT NULL, description TEXT NOT NULL, acceptance TEXT NOT NULL, state TEXT NOT NULL, revision INTEGER NOT NULL, version INTEGER NOT NULL, result TEXT, evidence TEXT, created_seq INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS tasks_project_seq ON tasks(project, created_seq);
CREATE TABLE IF NOT EXISTS attempts (id TEXT PRIMARY KEY, task_id TEXT NOT NULL, revision INTEGER NOT NULL, owner TEXT NOT NULL, run TEXT NOT NULL, certainty TEXT NOT NULL, outcome TEXT, started_at INTEGER, finished_at INTEGER);
CREATE INDEX IF NOT EXISTS attempts_task ON attempts(task_id);
CREATE TABLE IF NOT EXISTS feedback (id INTEGER PRIMARY KEY AUTOINCREMENT, task_id TEXT NOT NULL, revision INTEGER, author TEXT, body TEXT NOT NULL, accepted INTEGER, created_at INTEGER);
CREATE TABLE IF NOT EXISTS messages (id TEXT PRIMARY KEY, project TEXT NOT NULL, sender TEXT NOT NULL, recipient TEXT NOT NULL, body TEXT NOT NULL, subject TEXT, thread_id TEXT, reply_to TEXT, task_id TEXT, revision INTEGER, kind TEXT NOT NULL, acknowledged INTEGER NOT NULL, sequence INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS messages_recipient ON messages(recipient, acknowledged, sequence);
CREATE INDEX IF NOT EXISTS messages_project ON messages(project, sequence);
CREATE TABLE IF NOT EXISTS events (id TEXT PRIMARY KEY, project TEXT NOT NULL, sequence INTEGER NOT NULL, kind TEXT NOT NULL, actor TEXT NOT NULL, resource TEXT, attempt TEXT, observed_at INTEGER, imported INTEGER NOT NULL, payload TEXT NOT NULL);
CREATE UNIQUE INDEX IF NOT EXISTS events_project_seq ON events(project, sequence);
CREATE TABLE IF NOT EXISTS sequences (project TEXT PRIMARY KEY, value INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS requests (actor TEXT NOT NULL, request_id TEXT NOT NULL, epoch TEXT NOT NULL, fingerprint TEXT NOT NULL, response TEXT NOT NULL, created_at INTEGER NOT NULL, PRIMARY KEY(actor, request_id));
CREATE INDEX IF NOT EXISTS requests_created ON requests(created_at);";

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
            Some(version) => bail!("Unsupported agent bus schema version: {version}"),
            None => self.upgrade(path),
        }
    }

    fn create_schema(&self) -> Result<()> {
        self.connection.borrow_mut().batch_execute(SCHEMA)?;
        Ok(())
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

    /// Crash before commit leaves v1 usable; crash after commit reopens v2.
    fn upgrade(&self, path: &str) -> Result<()> {
        match read_legacy_payload(path)? {
            None => self.transaction(|| {
                self.create_schema()?;
                self.set_legacy_payload(SENTINEL)?;
                self.set_meta_version()?;
                Ok(())
            }),
            Some(payload) if payload == SENTINEL => bail!(
                "Agent bus database is marked as migrated but its schema marker is missing; restore the pre-v2 backup"
            ),
            Some(payload) => self.migrate(path, &payload),
        }
    }

    fn migrate(&self, path: &str, payload: &str) -> Result<()> {
        let legacy: LegacySnapshot = serde_json::from_str(payload)
            .map_err(|_| anyhow!("Unrecognized agent bus database content; refusing to migrate"))?;
        if path != ":memory:" {
            let backup = format!("{path}.pre-v2");
            if std::path::Path::new(&backup).exists() {
                let stored = read_legacy_payload(&backup)?;
                ensure!(
                    stored.as_deref() == Some(payload),
                    "Existing migration backup does not match this database; refusing to overwrite"
                );
            } else {
                let escaped = backup.replace('\'', "''");
                self.connection
                    .borrow_mut()
                    .batch_execute(&format!("VACUUM INTO '{escaped}'"))?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&backup, std::fs::Permissions::from_mode(0o600))?;
                }
            }
        }
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
            let mut created: BTreeMap<&str, u64> = BTreeMap::new();
            for task in legacy.tasks.values() {
                let created_seq = created.entry(&task.project).or_insert(0);
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
                    attempts: vec![],
                    created_seq: *created_seq,
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
                self.connection.borrow_mut().batch_execute("COMMIT")?;
                Ok(value)
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
        diesel::sql_query("INSERT INTO tasks(id, project, issuer, assignee, reviewer, description, acceptance, state, revision, version, result, evidence, created_seq) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
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
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    fn update_task_state(&self, task: &Task) -> Result<()> {
        diesel::sql_query("UPDATE tasks SET state=?, revision=?, version=?, result=?, evidence=? WHERE id=?")
            .bind::<Text, _>(&task.state)
            .bind::<Integer, _>(task.revision as i32)
            .bind::<BigInt, _>(task.version as i64)
            .bind::<Nullable<Text>, _>(&task.result)
            .bind::<Nullable<Text>, _>(&task.evidence)
            .bind::<Text, _>(&task.id)
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
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

    fn queue(&self, project: &str, mut message: Message) -> Result<Message> {
        ensure!(
            self.pending_count(project)? < MAX_PENDING_PER_PROJECT,
            capacity_exceeded("Pending message capacity reached")
        );
        ensure!(
            self.message_count(project)? < MAX_MESSAGES_PER_PROJECT,
            capacity_exceeded("Message history capacity reached")
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
        Ok(json!({
            "agents": {"used": self.agent_count()?, "limit": MAX_AGENTS},
            "tasks": {"used": self.task_count(project)?, "limit": MAX_TASKS_PER_PROJECT},
            "pending_messages": {"used": self.pending_count(project)?, "limit": MAX_PENDING_PER_PROJECT},
            "messages": {"used": self.message_count(project)?, "limit": MAX_MESSAGES_PER_PROJECT},
        }))
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

    pub fn register(&self, terminal: &str, program: &str, project: &str, name: &str) -> Result<Agent> {
        ensure!(
            !program.is_empty() && program.len() <= 64,
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

    /// Execution ownership dies with the run; persisted running work requires explicit recovery.
    pub fn recover(&self, actor: &Agent, run: &str) -> Result<()> {
        self.transaction(|| {
            let tasks = diesel::sql_query("SELECT id, project, issuer, assignee, reviewer, description, acceptance, state, revision, version, result, evidence, created_seq FROM tasks WHERE assignee = ? AND state = 'running'")
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
                    diesel::sql_query("UPDATE attempts SET certainty='interrupted', finished_at=? WHERE id=?")
                        .bind::<BigInt, _>(now() as i64)
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
                    "assignment",
                    "Interrupted task. Read it and explicitly start this revision to recover."
                        .into(),
                    &task.id,
                    task.revision,
                );
                self.queue(&task.project, message)?;
            }
            Ok(())
        })
    }

    fn pending_interrupt_notification(&self, actor: &Agent, task: &Task) -> Result<bool> {
        Ok(self.pending(actor)?.iter().any(|message| {
            message.kind == "assignment"
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

    pub fn pending(&self, actor: &Agent) -> Result<Vec<Message>> {
        Ok(diesel::sql_query("SELECT id, sender, recipient, body, subject, thread_id, reply_to, task_id, revision, kind, acknowledged, sequence FROM messages WHERE recipient = ? AND acknowledged = 0 ORDER BY sequence")
            .bind::<Text, _>(&actor.id)
            .load::<MessageRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(MessageRow::message)
            .collect())
    }

    pub fn next_work(&self, actor: &Agent, run: &str) -> Result<Option<Message>> {
        let executing = self.count(
            "SELECT COUNT(*) AS count FROM tasks WHERE assignee = ? AND state = 'running' AND id IN (SELECT task_id FROM attempts WHERE run = ? AND certainty = 'active')",
            &[actor.id.as_str(), run],
        )? > 0;
        Ok(self
            .pending(actor)?
            .into_iter()
            .find(|message| !executing || message.kind != "assignment"))
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
        Ok(diesel::sql_query("SELECT id, state, revision, version FROM tasks WHERE project = ? AND assignee = ? AND (issuer = ? OR reviewer = ? OR assignee = ?) ORDER BY created_seq")
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

    fn task_row(&self, selector: &str, binds: &[&str; 5]) -> Result<Option<Task>> {
        Ok(diesel::sql_query(selector.to_owned())
            .bind::<Text, _>(binds[0])
            .bind::<Text, _>(binds[1])
            .bind::<Text, _>(binds[2])
            .bind::<Text, _>(binds[3])
            .bind::<Text, _>(binds[4])
            .get_result::<TaskRow>(&mut *self.connection.borrow_mut())
            .optional()?
            .map(TaskRow::task))
    }

    pub fn task(&self, actor: &Agent, id: &str) -> Result<Task> {
        let task = self
            .task_row(
                "SELECT id, project, issuer, assignee, reviewer, description, acceptance, state, revision, version, result, evidence, created_seq FROM tasks WHERE id = ? AND project = ? AND (issuer = ? OR assignee = ? OR reviewer = ?)",
                &[id, actor.project.as_str(), actor.id.as_str(), actor.id.as_str(), actor.id.as_str()],
            )?
            .ok_or_else(|| scope_denied("Task not found"))?;
        self.assemble(task)
    }

    fn assemble(&self, mut task: Task) -> Result<Task> {
        task.attempts = diesel::sql_query("SELECT id, task_id, revision, owner, run, certainty, outcome, started_at, finished_at FROM attempts WHERE task_id = ? ORDER BY rowid")
            .bind::<Text, _>(&task.id)
            .load::<AttemptRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(AttemptRow::attempt)
            .collect();
        task.feedback = diesel::sql_query("SELECT body FROM feedback WHERE task_id = ? ORDER BY id")
            .bind::<Text, _>(&task.id)
            .load::<FeedbackRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(|row| row.body)
            .collect();
        task.executing_run = task
            .attempts
            .iter()
            .find(|attempt| attempt.certainty == "active" && attempt.revision == task.revision)
            .map(|attempt| attempt.run.clone());
        Ok(task)
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
        Ok(diesel::sql_query("SELECT id, terminal, name, program, project FROM agents WHERE project = ? AND (id = ? OR name = ?)")
            .bind::<Text, _>(&actor.project)
            .bind::<Text, _>(name)
            .bind::<Text, _>(name)
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
        let limit = Self::page_limit(limit)?;
        let rows = diesel::sql_query("SELECT id, sender, recipient, body, subject, thread_id, reply_to, task_id, revision, kind, acknowledged, sequence FROM messages WHERE recipient = ? AND acknowledged = 0 AND sequence > ? ORDER BY sequence LIMIT ?")
            .bind::<Text, _>(&actor.id)
            .bind::<BigInt, _>(cursor.unwrap_or(0) as i64)
            .bind::<BigInt, _>(limit as i64 + 1)
            .load::<MessageRow>(&mut *self.connection.borrow_mut())?;
        let messages: Vec<Message> = rows
            .into_iter()
            .take(limit as usize)
            .map(MessageRow::message)
            .collect();
        let cursor = messages.last().map(|message| message.sequence);
        Ok(json!({"messages": messages, "cursor": cursor}))
    }

    pub fn task_list(
        &self,
        actor: &Agent,
        state: Option<&str>,
        assignee: Option<&str>,
        cursor: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Value> {
        let limit = Self::page_limit(limit)?;
        let assignee = match assignee {
            Some(name) => match self.resolve(actor, name) {
                Ok(agent) => Some(agent.id),
                Err(_) => return Ok(json!({"tasks": [], "cursor": Value::Null})),
            },
            None => None,
        };
        let rows = diesel::sql_query("SELECT id, state, revision, version, assignee, reviewer, created_seq FROM tasks WHERE project = ? AND (issuer = ? OR assignee = ? OR reviewer = ?) AND state = COALESCE(?, state) AND assignee = COALESCE(?, assignee) AND created_seq > ? ORDER BY created_seq LIMIT ?")
            .bind::<Text, _>(&actor.project)
            .bind::<Text, _>(&actor.id)
            .bind::<Text, _>(&actor.id)
            .bind::<Text, _>(&actor.id)
            .bind::<Nullable<Text>, _>(state)
            .bind::<Nullable<Text>, _>(assignee)
            .bind::<BigInt, _>(cursor.unwrap_or(0) as i64)
            .bind::<BigInt, _>(limit as i64 + 1)
            .load::<TaskSummaryRow>(&mut *self.connection.borrow_mut())?;
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
        Ok(json!({"tasks": tasks, "cursor": cursor}))
    }

    pub fn events(&self, project: &str, after: Option<u64>, limit: Option<u32>) -> Result<Value> {
        let limit = Self::page_limit(limit)?;
        let rows = diesel::sql_query("SELECT id, project, sequence, kind, actor, resource, attempt, observed_at, imported, payload FROM events WHERE project = ? AND sequence > ? ORDER BY sequence LIMIT ?")
            .bind::<Text, _>(project)
            .bind::<BigInt, _>(after.unwrap_or(0) as i64)
            .bind::<BigInt, _>(limit as i64 + 1)
            .load::<EventRow>(&mut *self.connection.borrow_mut())?;
        let events: Vec<Event> = rows
            .into_iter()
            .take(limit as usize)
            .map(EventRow::event)
            .collect();
        let cursor = events.last().map(|event| event.sequence);
        Ok(json!({"events": events, "cursor": cursor}))
    }

    fn request_row(&self, actor: &str, request_id: &str) -> Result<Option<RequestRow>> {
        Ok(diesel::sql_query("SELECT epoch, fingerprint, response FROM requests WHERE actor = ? AND request_id = ?")
            .bind::<Text, _>(actor)
            .bind::<Text, _>(request_id)
            .get_result::<RequestRow>(&mut *self.connection.borrow_mut())
            .optional()?)
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
        match operation {
            Operation::AgentList => return Ok(json!(self.agents(&actor.project)?)),
            Operation::AgentInbox { cursor, limit } => return self.inbox(actor, *cursor, *limit),
            Operation::TaskList {
                state,
                assignee,
                cursor,
                limit,
            } => {
                return self.task_list(
                    actor,
                    state.as_deref(),
                    assignee.as_deref(),
                    *cursor,
                    *limit,
                )
            }
            Operation::TaskGet { task_id } => return Ok(json!(self.task(actor, task_id)?)),
            Operation::AgentRegister { .. } | Operation::AgentWait | Operation::AgentReady => {
                return Err(invalid_state("Operation requires live session handling"))
            }
            _ => {}
        }
        let Some(request_id) = operation.request_id() else {
            // AgentAck is idempotent by nature and carries no request ID.
            return self.transaction(|| self.mutate(actor, run, operation));
        };
        let serialized = serde_json::to_string(operation)?;
        if Uuid::parse_str(request_id).is_err() {
            return Err(invalid_input("request_id must be a UUID"));
        }
        let mut result = None;
        self.transaction(|| {
            if let Some(row) = self.request_row(&actor.id, request_id)? {
                // Expired epochs are rejected before a replay can masquerade as the original commit.
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
                self.request_count()? < MAX_REQUESTS,
                capacity_exceeded("Request capacity reached")
            );
            let value = self.mutate(actor, run, operation)?;
            self.store_request(&actor.id, request_id, run, &serialized, &value)?;
            result = Some(value);
            Ok(())
        })?;
        Ok(result.expect("committed mutation returns its stored response"))
    }

    fn agents(&self, project: &str) -> Result<Vec<Agent>> {
        Ok(diesel::sql_query("SELECT id, terminal, name, program, project FROM agents WHERE project = ? ORDER BY name")
            .bind::<Text, _>(project)
            .load::<AgentRow>(&mut *self.connection.borrow_mut())?
            .into_iter()
            .map(AgentRow::agent)
            .collect())
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
                text(body)?;
                if let Some(subject) = subject {
                    validate_subject(subject)?;
                }
                let recipient = self.resolve(actor, to)?;
                if let Some(task_id) = task_id {
                    self.task(actor, task_id)?;
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
                    message.kind != "assignment" && message.kind != "review",
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
                ..
            } => {
                text(description)?;
                text(acceptance)?;
                ensure!(
                    self.task_count(&actor.project)? < MAX_TASKS_PER_PROJECT,
                    capacity_exceeded("Task capacity reached")
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
                    attempts: vec![],
                    created_seq: self.next_sequence(&actor.project)?,
                    executing_run: None,
                };
                self.insert_task(&task)?;
                self.record(
                    &task.project,
                    "task_assigned",
                    &actor.id,
                    Some(&task.id),
                    None,
                    json!({"assignee": task.assignee, "reviewer": task.reviewer, "revision": task.revision}),
                )?;
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
                Ok(json!(task))
            }
            Operation::TaskStart {
                task_id,
                revision,
                expected_version,
                ..
            } => {
                let other_running = self.count(
                    "SELECT COUNT(*) AS count FROM tasks WHERE assignee = ? AND state = 'running' AND id != ?",
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
                let mut task = task;
                let active = self.active_attempts(&task.id)?;
                match task.state.as_str() {
                    "queued" => {}
                    "running" if active.iter().any(|attempt| attempt.run == run) => {
                        return Err(invalid_state("Task is already running in this session"))
                    }
                    "running" => {
                        for attempt in &active {
                            diesel::sql_query("UPDATE attempts SET certainty='interrupted', finished_at=? WHERE id=?")
                                .bind::<BigInt, _>(now() as i64)
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
                task.state = "running".into();
                task.version += 1;
                self.update_task_state(&task)?;
                diesel::sql_query("UPDATE messages SET acknowledged = 1 WHERE recipient = ? AND task_id = ? AND revision = ? AND acknowledged = 0")
                    .bind::<Text, _>(&actor.id)
                    .bind::<Text, _>(&task.id)
                    .bind::<Integer, _>(task.revision as i32)
                    .execute(&mut *self.connection.borrow_mut())?;
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
                diesel::sql_query("UPDATE attempts SET certainty='finished', outcome='submitted', finished_at=? WHERE id=?")
                    .bind::<BigInt, _>(now() as i64)
                    .bind::<Text, _>(&attempt.id)
                    .execute(&mut *self.connection.borrow_mut())?;
                let mut task = task;
                task.state = "submitted".into();
                task.result = Some(result.clone());
                task.evidence = Some(evidence.clone());
                task.version += 1;
                self.update_task_state(&task)?;
                self.record(
                    &task.project,
                    "task_submitted",
                    &actor.id,
                    Some(&task.id),
                    Some(&attempt.id),
                    json!({"revision": task.revision}),
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
                let mut task = self.task(actor, task_id)?;
                ensure!(
                    task.reviewer == actor.id,
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
                diesel::sql_query("UPDATE messages SET acknowledged = 1 WHERE recipient = ? AND task_id = ? AND revision = ? AND acknowledged = 0")
                    .bind::<Text, _>(&actor.id)
                    .bind::<Text, _>(&task.id)
                    .bind::<Integer, _>(task.revision as i32)
                    .execute(&mut *self.connection.borrow_mut())?;
                diesel::sql_query("INSERT INTO feedback(task_id, revision, author, body, accepted, created_at) VALUES (?,?,?,?,?,?)")
                    .bind::<Text, _>(&task.id)
                    .bind::<Integer, _>(task.revision as i32)
                    .bind::<Text, _>(&actor.id)
                    .bind::<Text, _>(feedback)
                    .bind::<Integer, _>(i32::from(*accepted))
                    .bind::<BigInt, _>(now() as i64)
                    .execute(&mut *self.connection.borrow_mut())?;
                task.state = if *accepted { "accepted" } else { "queued" }.into();
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
                    json!({"accepted": accepted, "revision": task.revision}),
                )?;
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
            _ => Err(invalid_state("Operation requires live session handling")),
        }
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
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
            attempts: vec![],
            created_seq: self.created_seq as u64,
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
        let pending = store.pending(&worker).unwrap();
        assert_eq!(pending.len(), 1);
        assert!(pending[0].body.contains("Interrupted task"));
        store.recover(&worker, "run-2").unwrap();
        assert_eq!(store.task(&worker, &task_id).unwrap().version, 3);
        assert_eq!(store.pending(&worker).unwrap().len(), 1);
        let restarted = store
            .execute(&worker, "run-2", &transition(&task_id, 1, &Uuid::new_v4().to_string()))
            .unwrap();
        assert_eq!(restarted["state"], "running");
        assert_eq!(restarted["version"], 4);
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
        let backup = format!("{path}.pre-v2");
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
            .unwrap();
        assert_eq!(interrupted["state"], "running");
        assert_eq!(interrupted["version"], 2);
        drop(store);
        let store = Store::open(path).unwrap();
        assert_eq!(store.task(&b, "task-1").unwrap().state, "running");
        assert_eq!(store.request_count().unwrap(), 2);
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
        assert!(!std::path::Path::new(&format!("{malformed}.pre-v2")).exists());
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
        assert!(std::path::Path::new(&format!("{orphan}.pre-v2")).exists());
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
}
