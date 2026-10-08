//! Git worktree identity, independent of desktop paths and repository remote URLs.
use crate::{invalid_input, scope_denied};
use anyhow::{ensure, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub(crate) struct Worktree {
    pub root: String,
    pub common: PathBuf,
    pub repository: String,
    pub checkout: String,
}

pub(crate) fn list(root: &str) -> Result<Vec<serde_json::Value>> {
    Worktree::discover(Path::new(root))?;
    let metadata = crate::companion::files::git_metadata(Path::new(root), &["worktree", "list", "--porcelain", "-z"]).map_err(|_| invalid_input("Worktree registry unavailable"))?;
    let mut rows = Vec::new();
    for record in metadata.split("\0\0").take(50) {
        let fields: Vec<_> = record.split('\0').collect();
        let Some(path) = fields.iter().find_map(|field| field.strip_prefix("worktree ")) else { continue; };
        let Ok(root) = crate::project_root(Path::new(path)) else { continue; };
        let branch = fields.iter().find_map(|field| field.strip_prefix("branch refs/heads/")).unwrap_or("detached HEAD");
        rows.push(serde_json::json!({"root":root,"branch":branch}));
    }
    Ok(rows)
}

pub(crate) fn create(root: &str, name: &str, base: &str) -> Result<String> {
    ensure!(!name.is_empty() && name.len() <= 64 && name.bytes().all(|byte|
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')),
        invalid_input("Worktree name must contain 1-64 letters, digits, hyphens or underscores"));
    ensure!(!base.is_empty() && base.len() <= 128 && !base.chars().any(char::is_control), invalid_input("Invalid base ref"));
    let checkout = Worktree::discover(Path::new(root))?;
    let commit = crate::companion::files::git_metadata(Path::new(&checkout.root),
        &["rev-parse", "--verify", "--end-of-options", &format!("{base}^{{commit}}")]).map_err(|_| invalid_input("Base commit unavailable"))?;
    let parent = Path::new(&checkout.root).parent().ok_or_else(|| invalid_input("Checkout parent unavailable"))?;
    let directory = parent.join(".warpai-worktrees").join(&checkout.repository);
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(name);
    if path.exists() {
        let existing = Worktree::discover(&path)?;
        ensure!(existing.repository == checkout.repository && branch(&existing.root).as_deref() == Some(&format!("warpai/{name}")),
            invalid_input("Worktree path is occupied; existing files were preserved"));
        crate::companion::files::git_metadata(Path::new(&existing.root), &["merge-base", "--is-ancestor", commit.trim(), "HEAD"]).map_err(|_| invalid_input("Existing worktree has a different base"))?;
        return Ok(existing.root);
    }
    let path = path.to_str().ok_or_else(|| invalid_input("Worktree path must be UTF-8"))?;
    #[cfg(windows)]
    let native_path = if let Some(unc) = path.strip_prefix(r"\\?\UNC\") { format!(r"\\{unc}") } else { path.strip_prefix(r"\\?\").unwrap_or(path).to_owned() };
    #[cfg(windows)]
    let path = native_path.as_str();
    crate::companion::files::git_metadata(Path::new(&checkout.root),
        &["worktree", "add", "-b", &format!("warpai/{name}"), "--", path, commit.trim()]).map_err(|_| invalid_input("Worktree creation failed; existing files and branches were preserved"))?;
    let created = Worktree::discover(Path::new(path))?;
    ensure!(created.repository == checkout.repository, scope_denied("Created checkout has another repository"));
    Ok(created.root)
}

pub(crate) fn branch(root: &str) -> Option<String> {
    if !Path::new(root).join(".git").exists() {
        return None;
    }
    crate::companion::files::git_metadata(Path::new(root), &["branch", "--show-current"])
        .ok()
        .map(|branch| {
            if branch.trim().is_empty() {
                "detached HEAD".into()
            } else {
                branch.trim().to_owned()
            }
        })
}

pub(crate) fn directory_identity(path: &Path) -> Result<String> {
    let handle = crate::companion::open_root(path)?;
    let mut digest = Sha256::new();
    digest.update(crate::companion::identity::native_file_identity(&handle)?);
    digest.update(
        handle
            .metadata()?
            .created()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
            .to_be_bytes(),
    );
    Ok(uuid::Uuid::from_bytes(digest.finalize()[..16].try_into().unwrap()).to_string())
}

impl Worktree {
    pub(crate) fn discover(path: &Path) -> Result<Self> {
        let root = crate::project_root(path)?;
        ensure!(
            !root.chars().any(char::is_control),
            invalid_input("Invalid checkout path")
        );
        let metadata = crate::companion::files::git_metadata(
            Path::new(&root),
            &[
                "rev-parse",
                "--path-format=absolute",
                "--show-toplevel",
                "--git-common-dir",
            ],
        )
        .map_err(|_| invalid_input("A registered Git worktree is required"))?;
        let mut lines = metadata.lines();
        let top = lines
            .next()
            .ok_or_else(|| invalid_input("Git checkout unavailable"))?;
        let common = lines
            .next()
            .ok_or_else(|| invalid_input("Git repository unavailable"))?;
        ensure!(
            lines.next().is_none(),
            invalid_input("Invalid Git metadata")
        );
        ensure!(
            Path::new(top).canonicalize()? == Path::new(&root),
            scope_denied("Git checkout does not match the selected directory")
        );
        let common = Path::new(common).canonicalize()?;
        let registered = crate::companion::files::git_metadata(
            Path::new(&root),
            &["worktree", "list", "--porcelain", "-z"],
        )
        .map_err(|_| invalid_input("Git worktree registration unavailable"))?;
        ensure!(
            registered
                .split('\0')
                .filter_map(|field| field.strip_prefix("worktree "))
                .any(
                    |path| Path::new(path).canonicalize().ok().as_deref() == Some(Path::new(&root))
                ),
            scope_denied("Checkout is not a registered worktree")
        );
        Ok(Self {
            repository: directory_identity(&common)?,
            checkout: directory_identity(Path::new(&root))?,
            root,
            common,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        transport::{self, Broker, Request, RunningBroker},
        ControllerOperation, Operation,
    };
    use serde_json::Value;

    pub(crate) struct Fixture {
        pub directory: tempfile::TempDir,
        pub main: PathBuf,
        pub linked: PathBuf,
        pub unjoined: PathBuf,
    }
    pub(crate) fn git(root: &Path, arguments: &[&str]) -> String {
        let output = std::process::Command::new(crate::installation::git_executable().unwrap())
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=",
            ])
            .args(arguments)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Git fixture command {:?} failed",
            arguments
        );
        String::from_utf8(output.stdout).unwrap()
    }
    impl Fixture {
        pub(crate) fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let main = directory.path().join("main checkout");
            std::fs::create_dir(&main).unwrap();
            git(&main, &["init", "--initial-branch=main"]);
            std::fs::write(main.join("source.txt"), b"base").unwrap();
            git(&main, &["add", "source.txt"]);
            git(&main, &["commit", "-m", "fixture base"]);
            let linked = directory.path().join("feature checkout");
            let unjoined = directory.path().join("unjoined checkout");
            git(
                &main,
                &["worktree", "add", "-b", "feature", linked.to_str().unwrap()],
            );
            git(
                &main,
                &[
                    "worktree",
                    "add",
                    "-b",
                    "unjoined",
                    unjoined.to_str().unwrap(),
                ],
            );
            Self {
                main: main.canonicalize().unwrap(),
                linked: linked.canonicalize().unwrap(),
                unjoined: unjoined.canonicalize().unwrap(),
                directory,
            }
        }
    }
    pub(crate) fn id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    pub(crate) fn call(broker: &Broker, request: &Request, operation: Operation) -> Result<Value> {
        let mut request = request.clone();
        request.operation = operation;
        transport::call(&broker.endpoint, &request)
    }
    pub(crate) fn client(broker: &Broker, root: &Path, name: &str) -> Request {
        let terminal = id();
        let capability = match broker.worktree_binding(root.to_str().unwrap()).unwrap() {
            Some(binding) => broker.prepare_bound_workspace(&terminal, &binding).unwrap(),
            None => broker.prepare(&terminal).unwrap(),
        };
        broker
            .activate(&terminal, "fixture", root.to_str().unwrap(), true)
            .unwrap();
        let mut request = Request {
            protocol_major: transport::PROTOCOL_MAJOR,
            terminal,
            capability,
            run: None,
            directory: Some(root.to_str().unwrap().into()),
            defer_initial_ready: false,
            native_activity: None,
            operation: Operation::AgentRegister { name: name.into() },
        };
        request.run = call(broker, &request, request.operation.clone()).unwrap()["run"]
            .as_str()
            .map(str::to_owned);
        request
    }
    pub(crate) fn assign(to: &str) -> Operation {
        Operation::TaskAssign {
            to: to.into(),
            description: "Implement owned feature".into(),
            acceptance: "Check owned files and report integration commit".into(),
            reviewer: None,
            request_id: id(),
            dependencies: vec![],
            start_deadline: None,
            execution_timeout_seconds: None,
            review_timeout_seconds: None,
        }
    }
    fn reserve() -> Operation {
        Operation::FileReserve {
            paths: vec!["source.txt".into()],
            mode: "exclusive".into(),
            task_id: None,
            attempt_id: None,
            ttl_seconds: None,
            request_id: id(),
        }
    }
    fn join(broker: &Broker, root: &Path) -> Value {
        broker
            .control(
                root.to_str().unwrap(),
                &ControllerOperation::WorktreeJoin {
                    root: root.to_str().unwrap().into(),
                    request_id: id(),
                },
            )
            .unwrap()
    }

    #[test]
    fn real_worktrees_share_tasks_but_keep_private_runs_files_and_evidence_separate() {
        let fixture = Fixture::new();
        let database = fixture.directory.path().join("bus.sqlite");
        let server = RunningBroker::start(&database).unwrap();
        let broker = &server.broker;
        let private = client(broker, &fixture.main, "private");
        let main = join(broker, &fixture.main);
        let linked = join(broker, &fixture.linked);
        assert_eq!(main["space_id"], linked["space_id"]);
        let lead = client(broker, &fixture.main, "lead");
        let worker = client(broker, &fixture.linked, "worker");
        let unjoined = client(broker, &fixture.unjoined, "outside");
        assert_eq!(
            call(broker, &lead, Operation::AgentList)
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(call(broker, &private, Operation::AgentList)
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty());
        assert!(call(broker, &unjoined, assign("worker")).is_err());
        call(broker, &lead, reserve()).unwrap();
        call(broker, &worker, reserve()).unwrap();
        let task = call(broker, &lead, assign("worker")).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        call(
            broker,
            &worker,
            Operation::TaskStart {
                task_id: task.clone(),
                revision: 1,
                expected_version: None,
                request_id: id(),
            },
        )
        .unwrap();
        std::fs::write(fixture.linked.join("source.txt"), b"feature").unwrap();
        let evidence = call(
            broker,
            &worker,
            Operation::EvidenceAdd {
                task_id: task.clone(),
                kind: "file".into(),
                attempt_id: None,
                path: Some("source.txt".into()),
                hash: Some(format!("{:x}", Sha256::digest(b"feature"))),
                commit: None,
                repository: None,
                branch: None,
                base: None,
                head: None,
                command: None,
                outcome: None,
                exit_code: None,
                summary: None,
                request_id: id(),
            },
        )
        .unwrap();
        let domain = format!("space:{}", main["space_id"].as_str().unwrap());
        broker
            .control(
                &domain,
                &ControllerOperation::EvidenceVerify {
                    evidence_id: evidence["evidence_id"].as_str().unwrap().into(),
                    verified: true,
                    request_id: id(),
                },
            )
            .unwrap();
        assert_eq!(
            std::fs::read(fixture.main.join("source.txt")).unwrap(),
            b"base"
        );
        call(
            broker,
            &worker,
            Operation::TaskSubmit {
                task_id: task.clone(),
                revision: 1,
                result: "Owned fixture verified".into(),
                evidence: "File hash verified".into(),
                evidence_ids: vec![evidence["evidence_id"].as_str().unwrap().into()],
                attempt_id: None,
                expected_version: None,
                request_id: id(),
            },
        )
        .unwrap();
        call(
            broker,
            &lead,
            Operation::TaskReview {
                task_id: task.clone(),
                revision: 1,
                accepted: true,
                feedback: "Accepted for integration".into(),
                expected_version: None,
                request_id: id(),
            },
        )
        .unwrap();
        assert!(call(
            broker,
            &private,
            Operation::TaskGet {
                task_id: task.clone()
            }
        )
        .is_err());
        let mut wrong = worker.clone();
        wrong.directory = Some(fixture.main.to_str().unwrap().into());
        assert!(call(broker, &wrong, Operation::AgentList).is_err());
        let clone = fixture.directory.path().join("independent clone");
        // Git clone interprets verbatim Windows source paths as transport URLs.
        let source = fixture.directory.path().join("main checkout");
        git(
            &fixture.main,
            &[
                "clone",
                "--local",
                source.to_str().unwrap(),
                clone.to_str().unwrap(),
            ],
        );
        assert_ne!(join(broker, &clone)["space_id"], main["space_id"]);
        let clone_client = client(broker, &clone.canonicalize().unwrap(), "clone");
        assert!(call(
            broker,
            &clone_client,
            Operation::TaskGet {
                task_id: task.clone()
            }
        )
        .is_err());
        broker
            .control(
                &domain,
                &ControllerOperation::WorktreeLeave {
                    root: fixture.linked.to_str().unwrap().into(),
                    request_id: id(),
                },
            )
            .unwrap();
        assert!(call(broker, &worker, Operation::AgentList).is_err());
        assert!(broker.peers(&lead.terminal).is_empty());
        join(broker, &fixture.linked);
        assert!(call(broker, &worker, Operation::AgentList).is_err());
        let fresh = client(broker, &fixture.linked, "worker");
        assert_eq!(
            call(
                broker,
                &fresh,
                Operation::TaskGet {
                    task_id: task.clone()
                }
            )
            .unwrap()["state"],
            "accepted"
        );
        drop(server);
        let reopened = RunningBroker::start(&database).unwrap();
        let fresh = client(&reopened.broker, &fixture.main, "lead");
        assert_eq!(
            call(
                &reopened.broker,
                &fresh,
                Operation::TaskGet { task_id: task }
            )
            .unwrap()["state"],
            "accepted"
        );
        let root = fixture.main.to_str().unwrap();
        let research = reopened
            .broker
            .control(
                root,
                &ControllerOperation::SpaceCreate {
                    name: "Research projects".into(),
                    request_id: id(),
                },
            )
            .unwrap();
        let mapped = reopened
            .broker
            .control(
                root,
                &ControllerOperation::WorkspaceMap {
                    space_id: research["space_id"].as_str().unwrap().into(),
                    root: root.into(),
                    repository_id: None,
                    model: "independent_worktrees".into(),
                    branch: None,
                    base_commit: None,
                    request_id: id(),
                },
            )
            .unwrap();
        reopened
            .broker
            .validate_workspace(&crate::WorkspaceBinding {
                id: mapped["workspace_id"].as_str().unwrap().into(),
                space: research["space_id"].as_str().unwrap().into(),
                root: root.into(),
            })
            .unwrap();
        assert!(reopened.broker.worktree_binding(root).unwrap().is_none());
        assert!(call(&reopened.broker, &fresh, Operation::AgentList).is_err());
    }

    #[test]
    fn replacement_checkout_and_forged_git_pointer_cannot_reuse_worktree_admission() {
        let fixture = Fixture::new();
        let server = RunningBroker::start(Path::new(":memory:")).unwrap();
        let broker = &server.broker;
        join(broker, &fixture.linked);
        let worker = client(broker, &fixture.linked, "worker");
        let stale_binding = broker.worktree_binding(fixture.linked.to_str().unwrap()).unwrap().unwrap();
        let old = fixture.directory.path().join("old checkout");
        std::fs::rename(&fixture.linked, &old).unwrap();
        std::fs::create_dir(&fixture.linked).unwrap();
        std::fs::copy(old.join(".git"), fixture.linked.join(".git")).unwrap();
        assert!(call(broker, &worker, Operation::AgentList).is_err());
        let terminal = id();
        assert!(broker.prepare_bound_workspace(&terminal, &stale_binding).is_err());
        join(broker, &fixture.linked);
        assert!(
            call(broker, &worker, Operation::AgentList).is_err(),
            "explicit rejoin must not restore a replaced root's capability"
        );
        let forged = fixture.directory.path().join("forged checkout");
        std::fs::create_dir(&forged).unwrap();
        std::fs::copy(old.join(".git"), forged.join(".git")).unwrap();
        assert!(Worktree::discover(&forged).is_err());
    }
}
