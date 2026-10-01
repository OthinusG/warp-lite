//! Local coordination for third-party CLI agents; no model or cloud client lives here.
pub mod mcp;
pub mod launch;
pub mod transport;

use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, ensure, Result};
use diesel::{connection::SimpleConnection, prelude::*, sql_types::Text};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

pub const MAX_FRAME: usize = 1024 * 1024;
const MAX_RECORDS: usize = 1000;
const MAX_TEXT: usize = 8192;

pub fn project_root(path: &Path) -> Result<String> {
    let path = path.canonicalize()?;
    ensure!(path.is_dir(), "Project must be a directory");
    let root = path
        .ancestors()
        .find(|p| p.join(".git").exists())
        .unwrap_or(&path);
    Ok(root
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Project path must be UTF-8"))?
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
        request_id: String,
    },
    AgentInbox,
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
    TaskGet {
        task_id: String,
    },
    TaskStart {
        task_id: String,
        revision: u32,
        request_id: String,
    },
    TaskSubmit {
        task_id: String,
        revision: u32,
        result: String,
        evidence: String,
        request_id: String,
    },
    TaskReview {
        task_id: String,
        revision: u32,
        accepted: bool,
        feedback: String,
        request_id: String,
    },
}

impl Operation {
    fn request_id(&self) -> Option<&str> {
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
    pub task_id: Option<String>,
    pub revision: Option<u32>,
    pub kind: String,
    pub acknowledged: bool,
    pub sequence: u64,
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
    pub result: Option<String>,
    pub evidence: Option<String>,
    pub feedback: Vec<String>,
    // Runtime ownership expires at restart; persisted running tasks require explicit recovery.
    #[serde(skip)]
    pub executing_run: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Retry {
    operation: String,
    result: Value,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Snapshot {
    agents: BTreeMap<String, Agent>,
    messages: Vec<Message>,
    tasks: BTreeMap<String, Task>,
    retries: BTreeMap<String, Retry>,
    sequence: u64,
}

/// One transactional owner keeps task updates and their notifications inseparable.
pub struct Store {
    connection: SqliteConnection,
    snapshot: Snapshot,
}

#[derive(QueryableByName)]
struct Row {
    #[diesel(sql_type = Text)]
    payload: String,
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
        connection.batch_execute("PRAGMA busy_timeout=5000; CREATE TABLE IF NOT EXISTS agent_bus_v1 (id INTEGER PRIMARY KEY CHECK(id=1), payload TEXT NOT NULL);")?;
        let row = diesel::sql_query("SELECT payload FROM agent_bus_v1 WHERE id=1")
            .get_result::<Row>(&mut connection)
            .optional()?;
        let snapshot = row
            .map(|r| serde_json::from_str(&r.payload))
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            connection,
            snapshot,
        })
    }

    fn commit(&mut self, snapshot: Snapshot) -> Result<()> {
        // ponytail: bounded O(n) snapshot writes; normalize tables beyond 1000 records.
        let payload = serde_json::to_string(&snapshot)?;
        self.connection.transaction::<_, diesel::result::Error, _>(|connection| {
            diesel::sql_query("INSERT INTO agent_bus_v1(id,payload) VALUES(1,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload")
                .bind::<Text, _>(payload).execute(connection)?;
            Ok(())
        })?;
        self.snapshot = snapshot;
        Ok(())
    }

    pub fn register(
        &mut self,
        terminal: &str,
        program: &str,
        project: &str,
        name: &str,
    ) -> Result<Agent> {
        ensure!(
            !program.is_empty() && program.len() <= 64,
            "Invalid managed program"
        );
        ensure!(
            !name.is_empty()
                && name.len() <= 64
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')),
            "Name must contain 1-64 ASCII letters, numbers, hyphens, or underscores"
        );
        let id = self
            .snapshot
            .agents
            .values()
            .find(|a| a.project == project && a.name == name)
            .or_else(|| {
                self.snapshot.agents.values().find(|a| {
                    a.terminal == terminal && a.program == program && a.project == project
                })
            })
            .map(|a| a.id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        ensure!(
            !self
                .snapshot
                .agents
                .values()
                .any(|a| a.project == project && a.name == name && a.id != id),
            "Agent name is already registered in this project"
        );
        ensure!(
            self.snapshot.agents.contains_key(&id) || self.snapshot.agents.len() < MAX_RECORDS,
            "Agent capacity reached"
        );
        let agent = Agent {
            id: id.clone(),
            terminal: terminal.to_owned(),
            name: name.to_owned(),
            program: program.to_owned(),
            project: project.to_owned(),
        };
        let mut snapshot = self.snapshot.clone();
        snapshot.agents.insert(id, agent.clone());
        self.commit(snapshot)?;
        Ok(agent)
    }

    pub fn recover(&mut self, actor: &Agent, run: &str) -> Result<()> {
        let mut next = self.snapshot.clone();
        let interrupted: Vec<_> = next
            .tasks
            .values()
            .filter(|t| {
                t.assignee == actor.id
                    && t.state == "running"
                    && t.executing_run.as_deref() != Some(run)
            })
            .cloned()
            .collect();
        for task in interrupted {
            if !next.messages.iter().any(|m| {
                m.to == actor.id
                    && m.task_id.as_deref() == Some(&task.id)
                    && m.revision == Some(task.revision)
                    && !m.acknowledged
            }) {
                notify(
                    &mut next,
                    &task.issuer,
                    &actor.id,
                    "Interrupted task. Read it and explicitly start this revision to recover."
                        .into(),
                    Some(task.id),
                    Some(task.revision),
                    "assignment",
                )?;
            }
        }
        self.commit(next)
    }

    pub fn pending(&self, actor: &Agent) -> Vec<Message> {
        self.snapshot
            .messages
            .iter()
            .filter(|m| m.to == actor.id && !m.acknowledged)
            .cloned()
            .collect()
    }

    pub fn next_work(&self, actor: &Agent, run: &str) -> Option<Message> {
        let executing = self.snapshot.tasks.values().any(|t| {
            t.assignee == actor.id
                && t.state == "running"
                && t.executing_run.as_deref() == Some(run)
        });
        self.pending(actor)
            .into_iter()
            .find(|m| !executing || m.kind != "assignment")
    }

    /// Task states visible to this issuer/reviewer, grouped under the receiving peer.
    pub fn task_states(&self, actor: &Agent, assignee: &str) -> Vec<Value> {
        self.snapshot
            .tasks
            .values()
            .filter(|task| {
                task.project == actor.project
                    && task.assignee == assignee
                    && (task.issuer == actor.id
                        || task.reviewer == actor.id
                        || task.assignee == actor.id)
            })
            .map(|task| json!({"id": task.id, "state": task.state, "revision": task.revision}))
            .collect()
    }

    pub fn task(&self, actor: &Agent, id: &str) -> Result<&Task> {
        visible_task(&self.snapshot, actor, id)
    }

    /// Discovery may claim an offline identity only before this identity has acquired work.
    pub fn has_work(&self, actor: &Agent) -> bool {
        self.snapshot
            .tasks
            .values()
            .any(|task| [&task.issuer, &task.assignee, &task.reviewer].contains(&&actor.id))
            || self.snapshot.messages.iter().any(|message| {
                !message.acknowledged && (message.from == actor.id || message.to == actor.id)
            })
    }

    pub fn execute(&mut self, actor: &Agent, run: &str, operation: &Operation) -> Result<Value> {
        let serialized = serde_json::to_string(operation)?;
        let retry_key = operation
            .request_id()
            .map(|id| format!("{}:{id}", actor.id));
        if let Some(id) = operation.request_id() {
            ensure!(Uuid::parse_str(id).is_ok(), "request_id must be a UUID");
            if let Some(retry) = self.snapshot.retries.get(retry_key.as_ref().unwrap()) {
                ensure!(
                    retry.operation == serialized,
                    "request_id was reused for a different operation"
                );
                return Ok(retry.result.clone());
            }
            ensure!(
                self.snapshot.retries.len() < MAX_RECORDS,
                "Request capacity reached"
            );
        }
        let mut next = self.snapshot.clone();
        let result = match operation {
            Operation::AgentList => {
                return Ok(json!(next
                    .agents
                    .values()
                    .filter(|a| a.project == actor.project)
                    .collect::<Vec<_>>()))
            }
            Operation::AgentReady => anyhow::bail!("Readiness requires the live terminal broker"),
            Operation::AgentInbox => return Ok(json!(self.pending(actor))),
            Operation::AgentAck { message_id } => {
                let message = next
                    .messages
                    .iter_mut()
                    .find(|m| m.id == *message_id && m.to == actor.id)
                    .ok_or_else(|| anyhow::anyhow!("Message not found"))?;
                ensure!(
                    message.kind != "assignment" && message.kind != "review",
                    "Task notifications require their task transition, not message acknowledgement"
                );
                message.acknowledged = true;
                json!({"acknowledged": message_id})
            }
            Operation::AgentSend { to, body, .. } => {
                text(body)?;
                let recipient = resolve(&next, actor, to)?;
                let message = notify(
                    &mut next,
                    &actor.id,
                    &recipient.id,
                    body.clone(),
                    None,
                    None,
                    "message",
                )?;
                json!(message)
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
                ensure!(next.tasks.len() < MAX_RECORDS, "Task capacity reached");
                let assignee = resolve(&next, actor, to)?;
                let reviewer = match reviewer {
                    Some(name) => resolve(&next, actor, name)?,
                    None => actor.clone(),
                };
                ensure!(
                    assignee.id != reviewer.id,
                    "Assignee cannot review its own task"
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
                    result: None,
                    evidence: None,
                    feedback: vec![],
                    executing_run: None,
                };
                notify(
                    &mut next,
                    &actor.id,
                    &task.assignee,
                    "Task assigned. Read the task and explicitly start its revision.".into(),
                    Some(task.id.clone()),
                    Some(task.revision),
                    "assignment",
                )?;
                next.tasks.insert(task.id.clone(), task.clone());
                json!(task)
            }
            Operation::TaskGet { task_id } => {
                return Ok(json!(visible_task(&next, actor, task_id)?))
            }
            Operation::TaskStart {
                task_id, revision, ..
            } => {
                let other_running = next
                    .tasks
                    .values()
                    .any(|t| t.assignee == actor.id && t.state == "running" && t.id != *task_id);
                ensure!(!other_running, "An assigned task is already running");
                let task = visible_task(&next, actor, task_id)?.clone();
                ensure!(
                    task.assignee == actor.id && task.revision == *revision,
                    "Not the assignee or stale revision"
                );
                ensure!(
                    task.state == "queued"
                        || (task.state == "running" && task.executing_run.as_deref() != Some(run)),
                    "Task is not queued or interrupted"
                );
                let mut task = task;
                task.state = "running".into();
                task.executing_run = Some(run.to_owned());
                consume_task_messages(&mut next, actor, &task);
                next.tasks.insert(task.id.clone(), task.clone());
                json!(task)
            }
            Operation::TaskSubmit {
                task_id,
                revision,
                result,
                evidence,
                ..
            } => {
                text(result)?;
                text(evidence)?;
                let mut task = visible_task(&next, actor, task_id)?.clone();
                ensure!(
                    task.assignee == actor.id
                        && task.revision == *revision
                        && task.state == "running"
                        && task.executing_run.as_deref() == Some(run),
                    "Task submission is unauthorized, stale, or not running in this session"
                );
                task.state = "submitted".into();
                task.result = Some(result.clone());
                task.evidence = Some(evidence.clone());
                notify(&mut next, &actor.id, &task.reviewer, "Result submitted. Inspect the task's result and evidence, then accept or request changes.".into(), Some(task.id.clone()), Some(task.revision), "review")?;
                next.tasks.insert(task.id.clone(), task.clone());
                json!(task)
            }
            Operation::TaskReview {
                task_id,
                revision,
                accepted,
                feedback,
                ..
            } => {
                text(feedback)?;
                let mut task = visible_task(&next, actor, task_id)?.clone();
                ensure!(
                    task.reviewer == actor.id
                        && task.revision == *revision
                        && task.state == "submitted",
                    "Task review is unauthorized, stale, or not submitted"
                );
                consume_task_messages(&mut next, actor, &task);
                task.feedback.push(feedback.clone());
                task.state = if *accepted { "accepted" } else { "queued" }.into();
                if !accepted {
                    task.revision = task
                        .revision
                        .checked_add(1)
                        .ok_or_else(|| anyhow::anyhow!("Revision overflow"))?;
                    task.result = None;
                    task.evidence = None;
                    task.executing_run = None;
                }
                let kind = if *accepted { "accepted" } else { "assignment" };
                notify(
                    &mut next,
                    &actor.id,
                    &task.assignee,
                    feedback.clone(),
                    Some(task.id.clone()),
                    Some(task.revision),
                    kind,
                )?;
                next.tasks.insert(task.id.clone(), task.clone());
                json!(task)
            }
            Operation::AgentRegister { .. } | Operation::AgentWait => {
                bail!("Operation requires live session handling")
            }
        };
        if let Some(key) = retry_key {
            next.retries.insert(
                key,
                Retry {
                    operation: serialized,
                    result: result.clone(),
                },
            );
        }
        self.commit(next)?;
        Ok(result)
    }
}

fn text(value: &str) -> Result<()> {
    ensure!(
        !value.trim().is_empty()
            && value.len() <= MAX_TEXT
            && !value
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\t')),
        "Text must be nonempty, at most 8192 bytes, and contain no terminal control characters"
    );
    Ok(())
}

fn resolve(snapshot: &Snapshot, actor: &Agent, name: &str) -> Result<Agent> {
    snapshot
        .agents
        .values()
        .find(|a| a.project == actor.project && (a.name == name || a.id == name))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Agent not found in this project"))
}

fn visible_task<'a>(snapshot: &'a Snapshot, actor: &Agent, id: &str) -> Result<&'a Task> {
    snapshot
        .tasks
        .get(id)
        .filter(|t| {
            t.project == actor.project && [&t.issuer, &t.assignee, &t.reviewer].contains(&&actor.id)
        })
        .ok_or_else(|| anyhow::anyhow!("Task not found"))
}

fn consume_task_messages(snapshot: &mut Snapshot, actor: &Agent, task: &Task) {
    for message in &mut snapshot.messages {
        if message.to == actor.id
            && message.task_id.as_deref() == Some(&task.id)
            && message.revision == Some(task.revision)
        {
            message.acknowledged = true;
        }
    }
}

fn notify(
    snapshot: &mut Snapshot,
    from: &str,
    to: &str,
    body: String,
    task_id: Option<String>,
    revision: Option<u32>,
    kind: &str,
) -> Result<Message> {
    if snapshot.messages.len() >= MAX_RECORDS {
        snapshot.messages.retain(|m| !m.acknowledged);
    }
    ensure!(
        snapshot.messages.len() < MAX_RECORDS,
        "Pending message capacity reached"
    );
    snapshot.sequence += 1;
    let message = Message {
        id: Uuid::new_v4().to_string(),
        from: from.into(),
        to: to.into(),
        body,
        task_id,
        revision,
        kind: kind.into(),
        acknowledged: false,
        sequence: snapshot.sequence,
    };
    snapshot.messages.push(message.clone());
    Ok(message)
}
