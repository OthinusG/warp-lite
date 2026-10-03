//! Validation and receipt-age rules shared by managed SSH status consumers.
use crate::proto::ManagedFence;
use uuid::Uuid;
pub const PROTOCOL_MAJOR: u32 = 1;

/// Terminal bytes are bounded and pinned to an admitted process, never local paths.
pub fn valid_terminal_state(state: &crate::proto::TerminalState) -> bool {
    let valid_id = |id: &str| Uuid::parse_str(id).is_ok_and(|id| !id.is_nil());
    state.fence.as_ref().is_some_and(|f| valid_fence(f, true))
        && valid_id(&state.session_id)
        && valid_id(&state.run_id)
        && state.attachment_generation > 0
        && !(state.processes_active == Some(false) && state.exit_code.is_none())
        && state.output.len() <= 32 * 1024
        && state
            .output_end
            .checked_sub(state.output_offset)
            .is_some_and(|remaining| {
                remaining >= state.output.len() as u64 && remaining <= 256 * 1024
            })
}

/// Nil or malformed IDs never identify a verified attachment.
pub fn valid_fence(fence: &ManagedFence, require_project: bool) -> bool {
    let valid_id = |value: &str| Uuid::parse_str(value).is_ok_and(|id| !id.is_nil());
    valid_id(&fence.service_id)
        && valid_id(&fence.service_boot_id)
        && valid_id(&fence.connection_id)
        && if require_project {
            valid_id(&fence.project_id)
        } else {
            fence.project_id.is_empty()
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_state_refuses_invalid_ownership_and_unbounded_replay() {
        let mut state = crate::proto::TerminalState {
            fence: Some(ManagedFence {
                service_id: Uuid::new_v4().to_string(),
                service_boot_id: Uuid::new_v4().to_string(),
                connection_id: Uuid::new_v4().to_string(),
                project_id: Uuid::new_v4().to_string(),
            }),
            session_id: Uuid::new_v4().to_string(),
            run_id: Uuid::new_v4().to_string(),
            attachment_generation: 1,
            output_end: 2,
            output: b"ok".to_vec(),
            ..crate::proto::TerminalState::default()
        };
        assert!(valid_terminal_state(&state));
        state.processes_active = Some(false);
        assert!(!valid_terminal_state(&state));
        state.exit_code = Some(0);
        assert!(valid_terminal_state(&state));
        state.exit_code = None;
        state.processes_active = None;
        state.output_end = 1;
        assert!(!valid_terminal_state(&state));
        state.output_end = 2;
        state.output_offset = 3;
        assert!(!valid_terminal_state(&state));
        state.output_offset = 0;
        state.output_end = 256 * 1024 + 1;
        assert!(!valid_terminal_state(&state));
        state.output_end = 64 * 1024;
        state.output = vec![0; 32 * 1024 + 1];
        assert!(!valid_terminal_state(&state));
        state.output = vec![0; 32 * 1024];
        assert!(valid_terminal_state(&state));
        state.run_id = Uuid::nil().to_string();
        assert!(!valid_terminal_state(&state));
    }
}
