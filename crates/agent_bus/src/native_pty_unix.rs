//! Audited openpty/setsid/controlling-terminal sequence from the desktop PTY.
use super::*;
use std::{
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::process::{CommandExt, ExitStatusExt},
    },
    process::{Child, Command, Stdio},
};

pub(super) struct Process {
    child: Child,
}
impl Drop for Process {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            // Failure during admission must not orphan a successfully spawned run.
            // The still-unreaped owned child fences process-group ID reuse.
            if unsafe { libc::kill(-(self.child.id() as i32), libc::SIGKILL) } == 0 {
                let _ = self.child.wait();
            }
        }
    }
}
fn size(value: Size) -> libc::winsize {
    libc::winsize {
        ws_row: value.rows,
        ws_col: value.columns,
        ws_xpixel: 0,
        ws_ypixel: 0,
    }
}
fn checked(value: i32) -> io::Result<()> {
    if value < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
pub(super) fn spawn(
    executable: &Path,
    args: &[String],
    root: &Path,
    dimensions: Size,
) -> io::Result<NativePty> {
    let (mut master, mut slave) = (-1, -1);
    let mut dimensions = size(dimensions);
    unsafe {
        checked(libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut dimensions,
        ))?;
    }
    let master = unsafe { File::from_raw_fd(master) };
    let slave = unsafe { File::from_raw_fd(slave) };
    unsafe {
        checked(libc::fcntl(
            master.as_raw_fd(),
            libc::F_SETFD,
            libc::FD_CLOEXEC,
        ))?;
        checked(libc::fcntl(
            slave.as_raw_fd(),
            libc::F_SETFD,
            libc::FD_CLOEXEC,
        ))?;
    }
    let mut command = tokio::process::Command::new(executable);
    crate::session::without_terminal_binding(&mut command);
    command.env_remove("VIBE_MCP_SERVERS");
    let mut command: Command = command.into_std();
    command
        .args(args)
        .current_dir(root)
        .env("TERM", "xterm-256color")
        .stdin(Stdio::from(slave.try_clone()?))
        .stdout(Stdio::from(slave.try_clone()?))
        .stderr(Stdio::from(slave.try_clone()?));
    unsafe {
        command.pre_exec(move || {
            // Only async-signal-safe native calls run after fork.
            for signal in 1..32 {
                if signal != libc::SIGKILL
                    && signal != libc::SIGSTOP
                    && libc::signal(signal, libc::SIG_DFL) == libc::SIG_ERR
                {
                    return Err(io::Error::last_os_error());
                }
            }
            let mut signals = std::mem::MaybeUninit::<libc::sigset_t>::uninit();
            checked(libc::sigemptyset(signals.as_mut_ptr()))?;
            checked(libc::sigprocmask(
                libc::SIG_SETMASK,
                signals.as_ptr(),
                std::ptr::null_mut(),
            ))?;
            checked(libc::setsid())?;
            checked(libc::ioctl(0, libc::TIOCSCTTY as _, 0))?;
            Ok(())
        });
    }
    let reader = master.try_clone()?;
    let child = command.spawn()?;
    Ok(NativePty {
        reader,
        writer: master,
        process: Process { child },
    })
}
impl Process {
    pub(super) fn resize(&self, file: &File, dimensions: Size) -> io::Result<()> {
        unsafe {
            checked(libc::ioctl(
                file.as_raw_fd(),
                libc::TIOCSWINSZ as _,
                &size(dimensions),
            ))
        }
    }
    pub(super) fn exit_code(&mut self) -> io::Result<Option<i32>> {
        Ok(self.child.try_wait()?.map(|status| {
            status
                .code()
                .unwrap_or_else(|| -status.signal().unwrap_or(1))
        }))
    }
    pub(super) fn stop(&mut self) -> io::Result<bool> {
        if self.child.try_wait()?.is_some() {
            return Ok(false);
        }
        // An unreaped owned child cannot have its PID recycled between this check and kill.
        unsafe {
            checked(libc::kill(-(self.child.id() as i32), libc::SIGTERM))?;
        }
        Ok(true)
    }
}
