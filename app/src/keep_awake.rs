//! Keeps the system awake while tracked CLI agents are working.
//!
//! The user-facing toggle lives in the tab bar; this singleton owns the
//! `prevent_sleep` guard and re-evaluates it whenever agent sessions change.

use crate::terminal::cli_agent_sessions::CLIAgentSessionsModel;
use warpui::{Entity, ModelContext, SingletonEntity};

pub(crate) struct KeepAwake {
    enabled: bool,
    guard: Option<prevent_sleep::Guard>,
}

impl Entity for KeepAwake {
    type Event = ();
}

impl SingletonEntity for KeepAwake {}

impl KeepAwake {
    pub(crate) fn new(ctx: &mut ModelContext<Self>) -> Self {
        ctx.subscribe_to_model(&CLIAgentSessionsModel::handle(ctx), |me, _, ctx| {
            me.sync(ctx)
        });
        Self {
            enabled: false,
            guard: None,
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    /// Flips the toggle. Session-scoped only: not persisted, so it resets on
    /// relaunch.
    pub(crate) fn toggle(&mut self, ctx: &mut ModelContext<Self>) {
        self.enabled = !self.enabled;
        self.sync(ctx);
        ctx.emit(());
    }

    fn sync(&mut self, ctx: &mut ModelContext<Self>) {
        let should_hold = self.enabled && CLIAgentSessionsModel::as_ref(ctx).any_in_progress();
        if should_hold {
            if self.guard.is_none() {
                self.guard = Some(prevent_sleep::prevent_sleep("Warpai CLI agent in progress"));
            }
        } else {
            self.guard = None;
        }
    }
}
