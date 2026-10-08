//! Trusted panel enrollment keeps the native process and physical checkout intact.
use super::*;

pub(super) fn active_agent(state: &State, agent: &str, run: &str) -> bool {
    state.terminals.values().any(|binding| {
        !binding.revoked
            && binding.physical_root.as_ref().is_none_or(|(root, handle)| {
                crate::companion::root_matches(handle, root).unwrap_or(false)
            })
            && binding.live.as_ref().is_some_and(|live| {
                !live.expired
                    && live.started.elapsed() < MUTATION_EPOCH
                    && live.run == run
                    && live.agent.as_ref().is_some_and(|actor| {
                        actor.id == agent && state.store.authorize(actor).is_ok()
                    })
            })
    })
}

impl Broker {
    pub(crate) fn worktree_domain(&self, root: &str) -> Result<Option<String>> {
        self.store()?.store.worktree_domain(root)
    }
    pub(crate) fn worktree_role_receipt(
        &self,
        root: &str,
        operation: &ControllerOperation,
    ) -> Result<Option<Value>> {
        self.store()?.store.worktree_role_receipt(root, operation)
    }
    pub(super) fn forwarded(&self, terminal: &str) -> Option<Broker> {
        self.shared.forwarded.lock().ok()?.get(terminal).cloned()
    }

    pub(super) fn forwarded_wakes(&self, pending: bool) -> Vec<Wake> {
        let routes = self
            .shared
            .forwarded
            .lock()
            .map(|routes| routes.clone())
            .unwrap_or_default();
        routes
            .into_iter()
            .flat_map(|(terminal, broker)| {
                let wakes = if pending {
                    broker.pending_work()
                } else {
                    broker.wakeups()
                };
                wakes
                    .into_iter()
                    .filter(move |wake| wake.terminal == terminal)
            })
            .collect()
    }

    pub(crate) fn worktree_candidates(&self, root: &str) -> Result<Vec<Value>> {
        let repository = crate::worktrees::Worktree::discover(Path::new(root))?.repository;
        let state = self.store()?;
        Ok(state.terminals.values().filter(|binding| !binding.revoked)
            .filter_map(|binding| {
                let live = binding.live.as_ref()?;
                if live.expired || live.started.elapsed() >= MUTATION_EPOCH { return None; }
                let agent = live.agent.as_ref()?;
                state.store.authorize(agent).ok()?;
                let physical = state.store.physical_root(agent).ok()?;
                let checkout = crate::worktrees::Worktree::discover(Path::new(&physical)).ok()?;
                if checkout.repository != repository { return None; }
                Some(json!({"agent":agent,"run":live.run,"root":physical,"branch":crate::worktrees::branch(&physical)}))
            }).collect())
    }

    pub(crate) fn enroll_worktree(
        &self,
        source: &Broker,
        operation: &ControllerOperation,
    ) -> Result<Value> {
        let (root, id, run, role) = match operation {
            ControllerOperation::WorktreeCoordinator {
                root, agent, run, ..
            } => (root, agent, run, "coordinator"),
            ControllerOperation::WorktreeWorker {
                root, agent, run, ..
            } => (root, agent, run, "worker"),
            _ => return Err(invalid_input("Not an enrollment intent")),
        };
        ensure!(
            operation
                .request_id()
                .is_some_and(|request| Uuid::parse_str(request).is_ok()),
            invalid_input("request_id must be a UUID")
        );
        let root = crate::project_root(Path::new(root))?;
        if let Some(receipt) = self.worktree_role_receipt(&root, operation)? {
            return Ok(receipt);
        }
        let same = Arc::ptr_eq(&self.shared, &source.shared);
        let mut source_state = source.store()?;
        let terminal = source_state
            .terminals
            .iter()
            .find_map(|(terminal, binding)| {
                let live = binding.live.as_ref()?;
                (!binding.revoked
                    && !live.expired
                    && live.started.elapsed() < MUTATION_EPOCH
                    && live.run == *run
                    && (live.agent.as_ref().is_some_and(|agent| agent.id == *id)
                        || live
                            .origin_agent
                            .as_ref()
                            .is_some_and(|agent| agent.id == *id)))
                .then(|| terminal.clone())
            })
            .ok_or_else(|| scope_denied("Selected Agent run is no longer active"))?;
        let live = source_state.terminals[&terminal].live.as_ref().unwrap();
        let original = live.agent.as_ref().unwrap().clone();
        source_state.store.authorize(&original)?;
        ensure!(
            source_state.store.physical_root(&original)? == root,
            scope_denied("Agent must already run in the selected checkout")
        );
        ensure!(
            !live.waiting && live.wake.is_none(),
            invalid_state("Agent is receiving work; retry after it returns")
        );
        let physical = source_state.terminals[&terminal].physical_root.as_ref();
        ensure!(
            physical.is_none_or(
                |(path, handle)| crate::companion::root_matches(handle, path).unwrap_or(false)
            ),
            scope_denied("Selected physical checkout was replaced")
        );
        let enroll = |state: &State| -> Result<(crate::WorkspaceBinding, Agent, Value)> {
            let binding = match state.store.worktree_binding(&root)? {
                Some(binding) => binding,
                None => {
                    state.store.execute_controller(
                        &root,
                        &ControllerOperation::WorktreeJoin {
                            root: root.clone(),
                            request_id: Uuid::new_v4().to_string(),
                        },
                    )?;
                    state
                        .store
                        .worktree_binding(&root)?
                        .ok_or_else(|| invalid_state("Checkout admission unavailable"))?
                }
            };
            let domain = binding.domain();
            let roles = state.store.worktree_roles(&domain)?;
            ensure!(
                roles.iter().all(|entry| entry.root != root
                    || entry.agent == original.id
                    || (role == "coordinator" && entry.role == "coordinator")
                    || (role == "worker" && entry.role == "worker" && entry.run.is_none())
                    || state
                        .terminals
                        .get(&terminal)
                        .and_then(|binding| binding.live.as_ref())
                        .and_then(|live| live.agent.as_ref())
                        .is_some_and(|agent| entry.agent == agent.id)),
                scope_denied("Checkout already has a bound Agent")
            );
            if role == "worker" {
                ensure!(
                    roles.iter().any(|entry| entry.role == "coordinator"
                        && entry.run.as_ref().is_some_and(|run| active_agent(
                            state,
                            &entry.agent,
                            run
                        ))),
                    invalid_state("Select an active Coordinator first")
                );
            }
            let actor = state.store.register_in_workspace(
                &terminal,
                &original.program,
                &binding,
                &original.name,
            )?;
            let result = state
                .store
                .worktree_role_intent(&root, operation, &actor, run, role)?;
            Ok((binding, actor, result))
        };
        let (binding, actor, result) = if same {
            if source_state.store.worktree_domain(&root)?.as_deref()
                != Some(original.project.as_str())
            {
                ensure!(
                    !source_state.store.unresolved_project_work(&original)?,
                    invalid_state("Resolve existing Project work before enrolling this Agent")
                );
            }
            enroll(&source_state)?
        } else {
            ensure!(
                !source_state.store.unresolved_project_work(&original)?,
                invalid_state("Resolve existing Project work before enrolling this Agent")
            );
            let mut target = self.store()?;
            let (binding, actor, result) = enroll(&target)?;
            let native = source_state.terminals.remove(&terminal).unwrap();
            target.terminals.insert(terminal.clone(), native);
            source
                .shared
                .forwarded
                .lock()
                .map_err(|_| coordinator_unavailable("Broker unavailable"))?
                .insert(terminal.clone(), self.clone());
            drop(target);
            (binding, actor, result)
        };
        drop(source_state);
        let mut target = self.store()?;
        let native = target.terminals.get_mut(&terminal).unwrap();
        native.workspace = Some(binding);
        native.physical_root = Some((
            Path::new(&root).to_owned(),
            crate::companion::open_root(Path::new(&root))?,
        ));
        let live = native.live.as_mut().unwrap();
        if live.origin_agent.is_none() {
            live.origin_agent = Some(original);
        }
        live.project = actor.project.clone();
        live.agent = Some(actor.clone());
        live.generation += 1;
        live.delivered.clear();
        let message = if role == "coordinator" {
            "You are the explicitly selected Worktree Coordinator. Assign tasks and context to bound workers, collect results, designate a different worker for peer review, choose accepted results for integration with warp_task_integrate, and create the final integration commit in your checkout. Do not start Agents or change their ownership."
        } else {
            "You are an explicitly bound Worktree worker. Work only in your own checkout, execute assigned tasks, report results and commit/test evidence, and review another worker when designated. The Coordinator owns task allocation and integration decisions."
        };
        target.store.execute(
            &Store::operator(&actor.project),
            crate::storage::OPERATOR_EPOCH,
            &Operation::AgentSend {
                to: actor.name.clone(),
                body: message.into(),
                subject: Some("Worktree role".into()),
                thread_id: None,
                reply_to: None,
                task_id: None,
                request_id: operation.request_id().unwrap().into(),
            },
        )?;
        self.shared.changed.notify_all();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktrees::tests::{assign, call, client, git, id, Fixture};

    #[test]
    fn explicit_reselection_fences_old_coordinator_and_receipt_replay_cannot_restore_it() {
        let fixture = Fixture::new();
        let owner =
            RunningBroker::start(&fixture.directory.path().join("reselection.sqlite")).unwrap();
        let broker = &owner.broker;
        let first = client(broker, &fixture.main, "first");
        let replacement = client(broker, &fixture.main, "replacement");
        let selection = |request: &Request| {
            let actor = call(
                broker,
                request,
                Operation::AgentRegister {
                    name: String::new(),
                },
            )
            .unwrap();
            ControllerOperation::WorktreeCoordinator {
                root: fixture.main.to_str().unwrap().into(),
                agent: actor["agent"]["id"].as_str().unwrap().into(),
                run: request.run.clone().unwrap(),
                request_id: id(),
            }
        };
        let first_selection = selection(&first);
        broker
            .control(fixture.main.to_str().unwrap(), &first_selection)
            .unwrap();
        broker.readiness(&first.terminal, true);
        std::thread::sleep(Duration::from_millis(800));
        let stale_wake = broker
            .wakeups()
            .into_iter()
            .find(|wake| wake.terminal == first.terminal)
            .unwrap();
        assert!(broker.claim_wake(&stale_wake));
        broker
            .control(fixture.main.to_str().unwrap(), &selection(&replacement))
            .unwrap();
        assert!(call(broker, &first, assign("replacement")).is_err());
        assert!(call(
            broker,
            &first,
            Operation::AgentInbox {
                cursor: None,
                limit: None
            }
        )
        .is_err());
        let replay = broker
            .control(fixture.main.to_str().unwrap(), &first_selection)
            .unwrap();
        let panel = broker
            .operator_panel(&PanelQuery {
                project: fixture.main.to_str().unwrap().into(),
                worktree: true,
                ..Default::default()
            })
            .unwrap();
        assert_ne!(panel["roles"][0]["agent"], replay["agent"]["id"]);
        assert_eq!(panel["coordinator_online"], true);
        assert!(!broker.wake_valid(&stale_wake));
        broker.end(&replacement.terminal);
        assert!(!broker
            .operator_panel(&PanelQuery {
                project: fixture.main.to_str().unwrap().into(),
                worktree: true,
                ..Default::default()
            })
            .unwrap()["coordinator_online"]
            .as_bool()
            .unwrap());
    }

    #[test]
    fn worktree_projection_does_not_adopt_private_tasks_before_coordinator_selection() {
        let fixture = Fixture::new();
        let owner =
            RunningBroker::start(&fixture.directory.path().join("project-mode.sqlite")).unwrap();
        let broker = &owner.broker;
        let issuer = client(broker, &fixture.main, "issuer");
        let worker = client(broker, &fixture.main, "worker");
        call(broker, &issuer, assign("worker")).unwrap();
        let project = PanelQuery {
            project: fixture.main.to_str().unwrap().into(),
            ..Default::default()
        };
        assert_eq!(
            broker.operator_panel(&project).unwrap()["tasks"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let team = broker
            .operator_panel(&PanelQuery {
                worktree: true,
                ..project
            })
            .unwrap();
        assert!(team["tasks"].as_array().unwrap().is_empty());
        assert_eq!(team["candidates"].as_array().unwrap().len(), 2);
        let actor = call(
            broker,
            &worker,
            Operation::AgentRegister {
                name: String::new(),
            },
        )
        .unwrap();
        assert!(broker
            .control(
                fixture.main.to_str().unwrap(),
                &ControllerOperation::WorktreeCoordinator {
                    root: fixture.main.to_str().unwrap().into(),
                    agent: actor["agent"]["id"].as_str().unwrap().into(),
                    run: worker.run.unwrap(),
                    request_id: id(),
                }
            )
            .is_err());
        let non_git = broker
            .operator_panel(&PanelQuery {
                project: fixture.directory.path().to_str().unwrap().into(),
                worktree: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(non_git["worktree_available"], false);
        assert!(non_git["tasks"].as_array().unwrap().is_empty());
    }

    #[test]
    fn explicit_coordinator_workers_peer_review_and_integration_preserve_project_history() {
        let fixture = Fixture::new();
        let owner = RunningBroker::start(&fixture.directory.path().join("roles.sqlite")).unwrap();
        let broker = &owner.broker;
        let coordinator = client(broker, &fixture.main, "coordinator");
        let worker = client(broker, &fixture.linked, "worker");
        let reviewer = client(broker, &fixture.unjoined, "reviewer");
        let private_project = fixture.main.to_str().unwrap();
        let old_id = call(
            broker,
            &coordinator,
            Operation::AgentRegister { name: "".into() },
        )
        .unwrap()["agent"]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let select = |request: &Request, root: &Path, lead: bool| {
            let agent = call(
                broker,
                request,
                Operation::AgentRegister { name: "".into() },
            )
            .unwrap()["agent"]["id"]
                .as_str()
                .unwrap()
                .to_owned();
            let root = root.to_str().unwrap().into();
            let run = request.run.clone().unwrap();
            let request_id = id();
            if lead {
                ControllerOperation::WorktreeCoordinator {
                    root,
                    agent,
                    run,
                    request_id,
                }
            } else {
                ControllerOperation::WorktreeWorker {
                    root,
                    agent,
                    run,
                    request_id,
                }
            }
        };
        assert!(broker
            .control(private_project, &select(&worker, &fixture.linked, false))
            .is_err());
        broker
            .control(private_project, &select(&coordinator, &fixture.main, true))
            .unwrap();
        assert!(broker
            .control(private_project, &select(&coordinator, &fixture.main, false))
            .is_err());
        broker
            .control(private_project, &select(&worker, &fixture.linked, false))
            .unwrap();
        broker
            .control(
                private_project,
                &select(&reviewer, &fixture.unjoined, false),
            )
            .unwrap();
        assert_eq!(broker.run(&coordinator.terminal), coordinator.run);
        assert!(call(broker, &worker, assign("reviewer")).is_err());
        let mut assignment = assign("worker");
        if let Operation::TaskAssign { reviewer, .. } = &mut assignment {
            *reviewer = Some("reviewer".into());
        }
        let task = call(broker, &coordinator, assignment).unwrap()["id"]
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
        assert!(call(
            broker,
            &coordinator,
            Operation::TaskIntegrate {
                task_id: task.clone(),
                commit: None,
                request_id: id()
            }
        )
        .is_err());
        call(
            broker,
            &worker,
            Operation::TaskSubmit {
                task_id: task.clone(),
                revision: 1,
                result: "Ready".into(),
                evidence: "Checks passed".into(),
                evidence_ids: vec![],
                attempt_id: None,
                expected_version: None,
                request_id: id(),
            },
        )
        .unwrap();
        let review = Operation::TaskReview {
            task_id: task.clone(),
            revision: 1,
            accepted: true,
            feedback: "Peer checked".into(),
            expected_version: None,
            request_id: id(),
        };
        assert!(call(broker, &worker, review.clone()).is_err());
        call(broker, &reviewer, review).unwrap();
        assert!(call(
            broker,
            &reviewer,
            Operation::TaskIntegrate {
                task_id: task.clone(),
                commit: None,
                request_id: id()
            }
        )
        .is_err());
        let commit = git(&fixture.main, &["rev-parse", "HEAD"]).trim().to_owned();
        call(
            broker,
            &coordinator,
            Operation::TaskIntegrate {
                task_id: task.clone(),
                commit: Some(commit),
                request_id: id(),
            },
        )
        .unwrap();
        let project = broker
            .operator_panel(&PanelQuery {
                project: private_project.into(),
                terminal: Some(coordinator.terminal.clone()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(project["agents"][0]["agent"]["id"], old_id);
        assert!(project["tasks"].as_array().unwrap().is_empty());
        let team = broker
            .operator_panel(&PanelQuery {
                project: private_project.into(),
                worktree: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(team["roles"].as_array().unwrap().len(), 3);
        assert_eq!(team["coordinator_online"], true);
        let scope = team["project"].as_str().unwrap();
        let created = broker
            .control(
                scope,
                &ControllerOperation::WorktreeCreate {
                    root: private_project.into(),
                    name: "extra".into(),
                    base: "main".into(),
                    request_id: id(),
                },
            )
            .unwrap();
        assert_eq!(created["agent_started"], false);
        assert_eq!(
            broker.worktree_candidates(private_project).unwrap().len(),
            3
        );
        broker.end(&coordinator.terminal);
        broker
            .activate(&coordinator.terminal, "codex", private_project, false)
            .unwrap();
        let restarted = Request {
            run: broker.run(&coordinator.terminal),
            ..coordinator.clone()
        };
        let private_agent = call(
            broker,
            &restarted,
            Operation::AgentRegister { name: "".into() },
        )
        .unwrap();
        assert_eq!(private_agent["agent"]["project"], private_project);
        assert_eq!(
            broker
                .operator_panel(&PanelQuery {
                    project: private_project.into(),
                    worktree: true,
                    ..Default::default()
                })
                .unwrap()["coordinator_online"],
            false
        );
    }

    #[test]
    fn existing_remote_endpoint_forwards_scope_and_native_lifecycle_without_restarting() {
        let fixture = Fixture::new();
        let source = RunningBroker::start(&fixture.directory.path().join("source.sqlite")).unwrap();
        let team = RunningBroker::start(&fixture.directory.path().join("team.sqlite")).unwrap();
        let request = client(&source.broker, &fixture.main, "coordinator");
        let agent = call(
            &source.broker,
            &request,
            Operation::AgentRegister { name: "".into() },
        )
        .unwrap()["agent"]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let operation = ControllerOperation::WorktreeCoordinator {
            root: fixture.main.to_str().unwrap().into(),
            agent,
            run: request.run.clone().unwrap(),
            request_id: id(),
        };
        team.broker
            .enroll_worktree(&source.broker, &operation)
            .unwrap();
        assert_eq!(source.broker.run(&request.terminal), request.run);
        source.broker.input_bytes(&request.terminal, b"draft");
        let panel = team
            .broker
            .operator_panel(&PanelQuery {
                project: fixture.main.to_str().unwrap().into(),
                worktree: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(panel["agents"][0]["draft"], "present");
        call(
            &source.broker,
            &request,
            Operation::AgentInbox {
                cursor: None,
                limit: None,
            },
        )
        .unwrap();
        source.broker.end(&request.terminal);
        source
            .broker
            .revoke_remote_run(&request.terminal, request.run.as_deref());
        assert!(source.broker.forwarded(&request.terminal).is_none());
        let next_run = id();
        let capability = source
            .broker
            .prepare_remote_run(&request.terminal, "codex", &fixture.main, &next_run)
            .unwrap();
        let restarted = Request {
            capability,
            run: Some(next_run.clone()),
            ..request.clone()
        };
        let private_agent = call(
            &source.broker,
            &restarted,
            Operation::AgentRegister { name: "".into() },
        )
        .unwrap();
        assert_eq!(
            private_agent["agent"]["project"],
            fixture.main.to_str().unwrap()
        );
        source
            .broker
            .revoke_remote_run(&request.terminal, request.run.as_deref());
        assert_eq!(source.broker.run(&request.terminal), Some(next_run));
        assert!(call(
            &source.broker,
            &request,
            Operation::AgentInbox {
                cursor: None,
                limit: None
            }
        )
        .is_err());
        assert_eq!(
            team.broker
                .operator_panel(&PanelQuery {
                    project: fixture.main.to_str().unwrap().into(),
                    worktree: true,
                    ..Default::default()
                })
                .unwrap()["coordinator_online"],
            false
        );
    }
}
