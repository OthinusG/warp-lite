//! Account-owned PTYs; clean connections own input leases, never process lifetime.
use super::*;
use crate::native_pty::{NativePty, Size};
use std::{
    collections::{BTreeMap, VecDeque},
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
};

const MAX_SESSIONS: usize = 32;
const REPLAY_BYTES: usize = 256 * 1024;
const PAGE_BYTES: usize = 32 * 1024;
type Error = ManagedErrorCode;
type Owner = Arc<Mutex<Session>>;

#[derive(Default)]
pub(super) struct Terminals(Mutex<BTreeMap<String, Owner>>);
#[derive(Default)]
struct Lease {
    connection: Option<String>,
    generation: u64,
    revoked: Arc<AtomicBool>,
}
#[derive(Default)]
struct Output {
    bytes: VecDeque<u8>,
    end: u64,
    closed: bool,
}
impl Output {
    fn append(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.end = self
            .end
            .checked_add(bytes.len() as u64)
            .ok_or(Error::ManagedUnavailable)?;
        self.bytes.extend(bytes);
        let discard = self.bytes.len().saturating_sub(REPLAY_BYTES);
        self.bytes.drain(..discard);
        Ok(())
    }
}
struct Input {
    connection: String,
    generation: u64,
    bytes: Vec<u8>,
}
struct Session {
    launch: TerminalLaunch,
    run_id: String,
    binding: Option<tasks::RunBinding>,
    pty: NativePty,
    lease: Arc<Mutex<Lease>>,
    output: Arc<Mutex<Output>>,
    input: mpsc::SyncSender<Input>,
    connection: Option<String>,
    generation: u64,
    sequence: u64,
    failed: Arc<AtomicBool>,
    revoked: Arc<AtomicBool>,
    stop_requested: bool,
}

fn lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, Error> {
    mutex.lock().map_err(|_| Error::ManagedUnavailable)
}
fn dimensions(columns: u32, rows: u32) -> Result<Size, Error> {
    if !(1..=1000).contains(&columns) || !(1..=1000).contains(&rows) {
        return Err(Error::ManagedInvalidInput);
    }
    Ok(Size {
        columns: columns as u16,
        rows: rows as u16,
    })
}
fn valid_id(id: &str) -> bool {
    Uuid::parse_str(id).is_ok_and(|id| !id.is_nil())
}

impl Terminals {
    pub(super) fn launch(
        &self,
        request: TerminalLaunch,
        fence: &ManagedFence,
        root: &Path,
        tasks: &tasks::Projects,
    ) -> Result<TerminalState, Error> {
        if !valid_id(&request.session_id) {
            return Err(Error::ManagedInvalidInput);
        }
        if request.agent_program.as_ref().is_some_and(|program| {
            program.is_empty()
                || program.len() > 64
                || program == crate::OPERATOR_PROGRAM
                || !program
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
        }) {
            return Err(Error::ManagedInvalidInput);
        }
        let size = dimensions(request.columns, request.rows)?;
        let mut sessions = lock(&self.0)?;
        if let Some(owner) = sessions.get(&request.session_id) {
            let mut session = lock(owner)?;
            let original = &session.launch;
            if original.fence.as_ref().map(|f| &f.project_id) != Some(&fence.project_id)
                || original.executable != request.executable
                || original.arguments != request.arguments
                || original.columns != request.columns
                || original.rows != request.rows
                || original.agent_program != request.agent_program
            {
                return Err(Error::ManagedConflict);
            }
            // Reconciliation returns the existing run without replacing its input lease.
            return session.state(fence, None);
        }
        if sessions.len() >= MAX_SESSIONS {
            return Err(Error::ManagedCapacityExceeded);
        }
        let run_id = Uuid::new_v4().to_string();
        let binding = request
            .agent_program
            .as_deref()
            .map(|program| tasks.bind_run(fence, root, &request.session_id, &run_id, program))
            .transpose()?;
        let pty = NativePty::spawn_with_environment(
            Path::new(&request.executable),
            &request.arguments,
            root,
            size,
            binding
                .as_ref()
                .map_or(&[], |binding| binding.environment.as_slice()),
        )
        .map_err(path_error)?;
        let mut reader = pty.reader.try_clone().map_err(path_error)?;
        let mut writer = pty.writer.try_clone().map_err(path_error)?;
        let output = Arc::new(Mutex::new(Output::default()));
        let revoked = Arc::new(AtomicBool::new(false));
        let lease = Arc::new(Mutex::new(Lease {
            connection: Some(fence.connection_id.clone()),
            generation: 1,
            revoked: revoked.clone(),
        }));
        let (input, receiver) = mpsc::sync_channel::<Input>(16);
        let failed = Arc::new(AtomicBool::new(false));
        let collected = output.clone();
        std::thread::spawn(move || {
            let mut buffer = [0; 8192];
            loop {
                let count = match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => count,
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                };
                let Ok(mut output) = collected.lock() else {
                    return;
                };
                if output.append(&buffer[..count]).is_err() {
                    break;
                }
            }
            if let Ok(mut output) = collected.lock() {
                output.closed = true;
            }
        });
        let guarded = lease.clone();
        let write_failed = failed.clone();
        std::thread::spawn(move || {
            while let Ok(input) = receiver.recv() {
                let Ok(lease) = guarded.lock() else { break };
                if lease.connection.as_deref() != Some(&input.connection)
                    || lease.generation != input.generation
                    || lease.revoked.load(Ordering::Acquire)
                    || write_failed.load(Ordering::Acquire)
                {
                    continue;
                }
                // Replacement serializes with actual bytes, not only queue admission.
                if writer
                    .write_all(&input.bytes)
                    .and_then(|_| writer.flush())
                    .is_err()
                {
                    write_failed.store(true, Ordering::Release);
                }
            }
        });
        let mut session = Session {
            launch: request,
            run_id,
            binding,
            pty,
            lease,
            output,
            input,
            connection: Some(fence.connection_id.clone()),
            generation: 1,
            sequence: 0,
            failed,
            revoked,
            stop_requested: false,
        };
        let state = session.state(fence, None)?;
        sessions.insert(
            session.launch.session_id.clone(),
            Arc::new(Mutex::new(session)),
        );
        Ok(state)
    }

    pub(super) fn control(
        &self,
        request: TerminalControl,
        fence: &ManagedFence,
    ) -> Result<TerminalState, Error> {
        let owner = lock(&self.0)?
            .get(&request.session_id)
            .cloned()
            .ok_or(Error::ManagedStaleAttachment)?;
        let mut session = lock(&owner)?;
        if session.launch.fence.as_ref().map(|f| &f.project_id) != Some(&fence.project_id)
            || request.run_id != session.run_id
        {
            return Err(Error::ManagedStaleAttachment);
        }
        let action =
            TerminalAction::try_from(request.action).map_err(|_| Error::ManagedInvalidInput)?;
        if action != TerminalAction::TerminalInput
            && (!request.input.is_empty() || request.input_sequence != 0)
            || action != TerminalAction::TerminalResize
                && (request.columns != 0 || request.rows != 0)
            || action != TerminalAction::TerminalRead && request.output_offset != 0
        {
            return Err(Error::ManagedInvalidInput);
        }
        if action == TerminalAction::TerminalAttach {
            if request.attachment_generation != session.generation {
                return Err(Error::ManagedStaleAttachment);
            }
            let generation = session
                .generation
                .checked_add(1)
                .ok_or(Error::ManagedUnavailable)?;
            let guarded = session.lease.clone();
            // An in-flight native write makes attachment temporarily unavailable;
            // never wait on it while holding process control or blocking Stop.
            let mut lease = guarded.try_lock().map_err(|_| Error::ManagedUnavailable)?;
            session.generation = generation;
            session.connection = Some(fence.connection_id.clone());
            lease.generation = generation;
            lease.connection = session.connection.clone();
            session.revoked = Arc::new(AtomicBool::new(false));
            lease.revoked = session.revoked.clone();
            session.sequence = 0;
        } else {
            if (session.connection.as_deref() != Some(&fence.connection_id)
                && !(action == TerminalAction::TerminalStop && session.connection.is_none()))
                || session.generation != request.attachment_generation
            {
                return Err(Error::ManagedStaleAttachment);
            }
            match action {
                TerminalAction::TerminalDetach => {
                    session.revoked.store(true, Ordering::Release);
                    session.connection = None;
                }
                TerminalAction::TerminalInput => {
                    if request.input.is_empty()
                        || request.input.len() > 4096
                        || session.sequence.checked_add(1) != Some(request.input_sequence)
                    {
                        return Err(Error::ManagedInvalidInput);
                    }
                    if session.failed.load(Ordering::Acquire)
                        || session.pty.exit_code().map_err(path_error)?.is_some()
                    {
                        return Err(Error::ManagedUnavailable);
                    }
                    session
                        .input
                        .try_send(Input {
                            connection: fence.connection_id.clone(),
                            generation: session.generation,
                            bytes: request.input,
                        })
                        .map_err(|_| Error::ManagedCapacityExceeded)?;
                    session.sequence = request.input_sequence;
                }
                TerminalAction::TerminalResize => {
                    session
                        .pty
                        .resize(dimensions(request.columns, request.rows)?)
                        .map_err(path_error)?;
                }
                TerminalAction::TerminalStop => {
                    // Process stop never waits for a blocked native input writer.
                    session.pty.stop().map_err(path_error)?;
                    session.stop_requested = true;
                }
                TerminalAction::TerminalRead => (),
                TerminalAction::TerminalRelease => {
                    if session.pty.exit_code().map_err(path_error)?.is_none()
                        || session.pty.is_active().map_err(path_error)?
                        || !lock(&session.output)?.closed
                    {
                        return Err(Error::ManagedConflict);
                    }
                    let state = session.state(fence, None)?;
                    drop(session);
                    let mut sessions = lock(&self.0)?;
                    if sessions
                        .get(&request.session_id)
                        .is_none_or(|current| !Arc::ptr_eq(current, &owner))
                    {
                        return Err(Error::ManagedStaleAttachment);
                    }
                    sessions.remove(&request.session_id);
                    return Ok(state);
                }
                _ => return Err(Error::ManagedInvalidInput),
            }
        }
        session.state(
            fence,
            (action == TerminalAction::TerminalRead).then_some(request.output_offset),
        )
    }

    pub(super) fn list(&self, fence: &ManagedFence) -> Result<TerminalStates, Error> {
        let owners: Vec<_> = lock(&self.0)?.values().cloned().collect();
        let mut sessions = Vec::new();
        for owner in owners {
            let mut session = lock(&owner)?;
            if session.launch.fence.as_ref().map(|f| &f.project_id) == Some(&fence.project_id) {
                sessions.push(session.state(fence, None)?);
            }
        }
        Ok(TerminalStates { sessions })
    }

    pub(super) fn disconnect(&self, connection: &str) {
        let Ok(owners) = self
            .0
            .lock()
            .map(|m| m.values().cloned().collect::<Vec<_>>())
        else {
            return;
        };
        for owner in owners {
            if let Ok(mut session) = owner.lock() {
                if session.connection.as_deref() != Some(connection) {
                    continue;
                }
                session.connection = None;
                session.revoked.store(true, Ordering::Release);
            }
        }
    }

    pub(super) fn active(&self) -> bool {
        let Ok(owners) = self
            .0
            .lock()
            .map(|m| m.values().cloned().collect::<Vec<_>>())
        else {
            return true;
        };
        owners.into_iter().fold(false, |active, owner| {
            let owned = owner.lock().map_or(true, |mut s| {
                let active = s.pty.is_active().unwrap_or(true);
                if !active {
                    s.binding.take();
                }
                active || s.output.lock().map_or(true, |output| !output.closed)
            });
            active || owned
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_discards_old_bytes_without_losing_absolute_offsets() {
        let mut output = Output::default();
        output.append(&vec![1; REPLAY_BYTES]).unwrap();
        output.append(b"tail").unwrap();
        assert_eq!(output.end, REPLAY_BYTES as u64 + 4);
        assert_eq!(output.bytes.len(), REPLAY_BYTES);
        assert_eq!(
            output
                .bytes
                .iter()
                .rev()
                .take(4)
                .copied()
                .collect::<Vec<_>>(),
            b"liat"
        );
        output.end = u64::MAX;
        assert!(output.append(b"overflow").is_err());
        assert_eq!(output.bytes.len(), REPLAY_BYTES);
    }
}

impl Session {
    fn state(&mut self, fence: &ManagedFence, offset: Option<u64>) -> Result<TerminalState, Error> {
        let processes_active = Some(self.pty.is_active().map_err(path_error)?);
        let exit_code = self.pty.exit_code().map_err(path_error)?;
        if processes_active == Some(false) {
            self.binding.take();
        }
        let output = lock(&self.output)?;
        let first = output.end - output.bytes.len() as u64;
        let requested = offset.unwrap_or(output.end);
        if requested > output.end {
            return Err(Error::ManagedInvalidInput);
        }
        let start = requested.max(first);
        Ok(TerminalState {
            fence: Some(fence.clone()),
            session_id: self.launch.session_id.clone(),
            run_id: self.run_id.clone(),
            attachment_generation: self.generation,
            attached: self.connection.is_some(),
            stop_requested: self.stop_requested,
            exit_code,
            processes_active,
            output_offset: start,
            output_end: output.end,
            output: if offset.is_some() {
                output
                    .bytes
                    .iter()
                    .skip((start - first) as usize)
                    .take(PAGE_BYTES)
                    .copied()
                    .collect()
            } else {
                Vec::new()
            },
            output_truncated: requested < first,
            accepted_input_sequence: self.sequence,
            input_failed: self.failed.load(Ordering::Acquire),
            output_closed: output.closed,
        })
    }
}
