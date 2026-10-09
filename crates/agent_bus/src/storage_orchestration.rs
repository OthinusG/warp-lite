//! Explicit roles reuse the task engine; they never grant file or process authority.
use super::*;

#[derive(QueryableByName, serde::Serialize)]
pub(crate) struct WorktreeRole {
    #[diesel(sql_type = Text)]
    pub agent: String,
    #[diesel(sql_type = Text)]
    pub role: String,
    #[diesel(sql_type = Text)]
    pub root: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub run: Option<String>,
}

#[derive(QueryableByName, serde::Serialize)]
pub(crate) struct WorktreeIntegration {
    #[diesel(sql_type = Nullable<Text>)]
    pub commit: Option<String>,
}

impl Store {
    pub(crate) fn worktree_integration(
        &self,
        project: &str,
        task: &str,
    ) -> Result<Option<WorktreeIntegration>> {
        Ok(diesel::sql_query(
            "SELECT commit_id AS 'commit' FROM worktree_integrations WHERE project=? AND task=?",
        )
        .bind::<Text, _>(project)
        .bind::<Text, _>(task)
        .get_result(&mut *self.connection.borrow_mut())
        .optional()?)
    }
    pub(crate) fn is_worktree_coordinator(&self, actor: &Agent) -> Result<bool> {
        Ok(self.count("SELECT COUNT(*) AS count FROM worktree_roles WHERE project=? AND agent=? AND role='coordinator' AND run IS NOT NULL", &[&actor.project, &actor.id])? == 1)
    }

    pub(crate) fn can_communicate(&self, actor: &Agent, peer: &Agent) -> Result<bool> {
        if actor.program == OPERATOR_PROGRAM || peer.program == OPERATOR_PROGRAM {
            return Ok(true);
        }
        let roles = self.worktree_roles(&actor.project)?;
        if roles.is_empty() {
            return Ok(true);
        }
        Ok(roles.iter().any(|role| role.role == "coordinator" && role.run.is_some()
            && (role.agent == actor.id || role.agent == peer.id))
            || self.physical_root(actor)? == self.physical_root(peer)?)
    }

    pub(crate) fn worktree_role_receipt(
        &self,
        root: &str,
        operation: &ControllerOperation,
    ) -> Result<Option<Value>> {
        let request = operation
            .request_id()
            .ok_or_else(|| invalid_input("Missing request ID"))?;
        let Some(row) = self.request_row(&Self::operator(root).id, request)? else {
            return Ok(None);
        };
        ensure!(
            row.fingerprint == serde_json::to_string(operation)?,
            request_conflict("Request ID belongs to another selection")
        );
        Ok(Some(serde_json::from_str(&row.response)?))
    }
    pub(crate) fn worktree_domain(&self, root: &str) -> Result<Option<String>> {
        let checkout = crate::worktrees::Worktree::discover(std::path::Path::new(root))?;
        Ok(diesel::sql_query("SELECT 'space:' || w.space_id AS value FROM worktree_admissions AS a JOIN workspaces AS w ON w.id=a.workspace_id WHERE a.repository=? LIMIT 1")
            .bind::<Text, _>(&checkout.repository).get_result::<ValueRow>(&mut *self.connection.borrow_mut()).optional()?.map(|row| row.value))
    }
    pub(crate) fn worktree_role_intent(
        &self,
        root: &str,
        operation: &ControllerOperation,
        actor: &Agent,
        run: &str,
        role: &str,
    ) -> Result<Value> {
        let request = operation
            .request_id()
            .ok_or_else(|| invalid_input("Missing request ID"))?;
        ensure!(
            Uuid::parse_str(request).is_ok(),
            invalid_input("request_id must be a UUID")
        );
        self.execute_mutation(
            &Self::operator(root).id,
            OPERATOR_EPOCH,
            request,
            &serde_json::to_string(operation)?,
            true,
            || self.set_worktree_role(actor, run, role),
        )
    }
    pub(crate) fn worktree_roles(&self, project: &str) -> Result<Vec<WorktreeRole>> {
        Ok(diesel::sql_query(
            "SELECT agent,role,root,run FROM worktree_roles WHERE project=? ORDER BY role,root",
        )
        .bind::<Text, _>(project)
        .load(&mut *self.connection.borrow_mut())?)
    }

    pub(crate) fn set_worktree_role(&self, actor: &Agent, run: &str, role: &str) -> Result<Value> {
        {
            self.authorize(actor)?;
            let root = self.physical_root(actor)?;
            let roles = self.worktree_roles(&actor.project)?;
            if role == "coordinator" {
                diesel::sql_query(
                    "UPDATE worktree_roles SET role='worker' WHERE project=? AND role='coordinator'",
                )
                .bind::<Text, _>(&actor.project)
                .execute(&mut *self.connection.borrow_mut())?;
            } else {
                ensure!(
                    role == "worker"
                        && roles
                            .iter()
                            .any(|entry| entry.role == "coordinator"),
                    invalid_state("Select an active Coordinator first")
                );
                ensure!(
                    roles
                        .iter()
                        .all(|entry| entry.agent != actor.id || entry.role == "worker"),
                    scope_denied("Coordinator cannot also be a worker")
                );
            }
            diesel::sql_query("INSERT INTO worktree_roles(project,agent,role,root,run) VALUES (?,?,?,?,?) ON CONFLICT(project,agent) DO UPDATE SET role=excluded.role,root=excluded.root,run=excluded.run")
                .bind::<Text, _>(&actor.project).bind::<Text, _>(&actor.id).bind::<Text, _>(role)
                .bind::<Text, _>(&root).bind::<Text, _>(run).execute(&mut *self.connection.borrow_mut())?;
            self.record(
                &actor.project,
                "worktree_role_selected",
                &Self::operator(&actor.project).id,
                Some(&actor.id),
                None,
                json!({"role":role,"root":root,"run":run}),
            )?;
            Ok(json!({"agent":actor,"role":role,"root":root,"run":run}))
        }
    }

    pub(crate) fn worktree_offline(&self, agent: &str, run: &str) -> Result<()> {
        diesel::sql_query("UPDATE worktree_roles SET run=NULL WHERE agent=? AND run=?")
            .bind::<Text, _>(agent)
            .bind::<Text, _>(run)
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    pub(crate) fn clear_worktree_runs(&self) -> Result<()> {
        diesel::sql_query("UPDATE worktree_roles SET run=NULL")
            .execute(&mut *self.connection.borrow_mut())?;
        Ok(())
    }

    pub(crate) fn unresolved_project_work(&self, actor: &Agent) -> Result<bool> {
        Ok(self.count("SELECT COUNT(*) AS count FROM tasks WHERE (issuer=? OR assignee=? OR reviewer=?) AND state NOT IN ('accepted','failed','cancelled','expired')",
            &[&actor.id, &actor.id, &actor.id])? > 0 || self.count(
                "SELECT COUNT(*) AS count FROM messages WHERE acknowledged=0 AND recipient=?", &[&actor.id])? > 0)
    }

    pub(crate) fn authorize_orchestration(
        &self,
        actor: &Agent,
        run: &str,
        operation: &Operation,
    ) -> Result<()> {
        let roles = self.worktree_roles(&actor.project)?;
        if roles.is_empty() {
            ensure!(
                !matches!(operation, Operation::TaskIntegrate { .. }),
                scope_denied("Integration decisions require Worktree mode")
            );
            return Ok(());
        }
        let coordinator = roles.iter().find(|entry| entry.role == "coordinator");
        let manages = matches!(
            operation,
            Operation::TaskAssign { .. }
                | Operation::TaskCreatePool { .. }
                | Operation::TaskReassign { .. }
                | Operation::TaskRetry { .. }
                | Operation::TaskSetDependencies { .. }
                | Operation::TaskCancel { .. }
                | Operation::TaskIntegrate { .. }
        );
        if manages {
            ensure!(
                coordinator.is_some_and(
                    |entry| entry.agent == actor.id && entry.run.as_deref() == Some(run)
                ),
                scope_denied("Only the selected active Coordinator can orchestrate this team")
            );
        } else if actor.program != OPERATOR_PROGRAM {
            ensure!(
                roles
                    .iter()
                    .any(|entry| entry.agent == actor.id && entry.run.as_deref() == Some(run)),
                scope_denied("Agent has no active Worktree role; select it in the panel")
            );
        }
        let worker = |name: &str| -> Result<()> {
            let agent = self.agent(&actor.project, name)?;
            ensure!(
                roles.iter().any(|entry| entry.agent == agent.id
                    && entry.role == "worker"
                    && entry.run.is_some()),
                scope_denied("Task recipient must be an explicitly bound active worker")
            );
            Ok(())
        };
        match operation {
            Operation::TaskAssign {
                to: assignee,
                reviewer,
                ..
            }
            | Operation::TaskReassign {
                assignee, reviewer, ..
            } => {
                worker(assignee)?;
                if let Some(reviewer) = reviewer {
                    let agent = self.agent(&actor.project, reviewer)?;
                    ensure!(
                        roles
                            .iter()
                            .any(|entry| entry.agent == agent.id && entry.run.is_some()),
                        scope_denied("Reviewer must be an explicitly bound active participant")
                    );
                }
            }
            Operation::TaskCreatePool { eligible, .. } => {
                for name in eligible {
                    worker(name)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn integrate_task(
        &self,
        actor: &Agent,
        task_id: &str,
        commit: Option<&str>,
    ) -> Result<Value> {
        let task = self.operator_task(&actor.project, task_id)?;
        ensure!(
            task.state == "accepted",
            invalid_state("Result must pass its designated review before integration")
        );
        if let Some(commit) = commit {
            ensure!(
                (40..=64).contains(&commit.len())
                    && commit.bytes().all(|byte| byte.is_ascii_hexdigit()),
                invalid_input("A full integration commit ID is required")
            );
            let root = self.physical_root(actor)?;
            let resolved = crate::companion::files::git_metadata(
                std::path::Path::new(&root),
                &[
                    "rev-parse",
                    "--verify",
                    "--end-of-options",
                    &format!("{commit}^{{commit}}"),
                ],
            )
            .map_err(|_| invalid_input("Integration commit unavailable"))?;
            ensure!(
                resolved.trim().eq_ignore_ascii_case(commit),
                invalid_input("Integration commit is unavailable")
            );
            crate::companion::files::git_metadata(
                std::path::Path::new(&root),
                &["merge-base", "--is-ancestor", commit, "HEAD"],
            )
            .map_err(|_| invalid_input("Integration commit is not in this checkout history"))?;
        }
        diesel::sql_query("INSERT INTO worktree_integrations(task,project,coordinator,commit_id) VALUES (?,?,?,?) ON CONFLICT(task) DO UPDATE SET coordinator=excluded.coordinator,commit_id=excluded.commit_id")
            .bind::<Text, _>(&task.id).bind::<Text, _>(&actor.project).bind::<Text, _>(&actor.id)
            .bind::<Nullable<Text>, _>(commit).execute(&mut *self.connection.borrow_mut())?;
        self.record(
            &actor.project,
            "worktree_integration_selected",
            &actor.id,
            Some(&task.id),
            None,
            json!({"commit":commit,"git_execution_implied":false}),
        )?;
        Ok(json!({"task_id":task.id,"commit":commit,"selected":true,"git_execution_implied":false}))
    }
}
