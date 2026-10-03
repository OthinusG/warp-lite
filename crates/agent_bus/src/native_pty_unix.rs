//! Audited openpty/setsid/controlling-terminal sequence from the desktop PTY.
use super::*;
use std::{
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::process::CommandExt,
    },
    process::{Child, Command, Stdio},
};

pub(super) struct Process {
    child: Child,
}
impl Drop for Process {
    fn drop(&mut self) {
        // WNOWAIT retains the leader's ID until this owner is released, even if
        // the shell exited before its background children. No unrelated group
        // can reuse it. Admission failures also cannot orphan a spawned process.
        let killed = unsafe { libc::kill(-(self.child.id() as i32), libc::SIGKILL) };
        if killed == 0 || self.exit_code().is_ok_and(|code| code.is_some()) {
            let _ = self.child.wait();
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
    environment: &[(String, String)],
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
    command.envs(environment.iter().cloned());
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
    #[cfg(target_os = "macos")]
    fn group_members(&self) -> io::Result<Vec<sysinfo::Pid>> {
        let mut pids = [0i32; 4096];
        let bytes = unsafe {
            libc::proc_listpids(
                2, /* PROC_PGRP_ONLY */
                self.child.id(),
                pids.as_mut_ptr().cast(),
                std::mem::size_of_val(&pids) as _,
            )
        };
        if bytes <= 0 || bytes as usize >= std::mem::size_of_val(&pids) || bytes as usize % 4 != 0 {
            return Err(io::Error::other("Owned process group is unavailable"));
        }
        Ok(pids[..bytes as usize / 4]
            .iter()
            .filter(|pid| **pid > 0)
            .map(|pid| sysinfo::Pid::from_u32(*pid as u32))
            .collect())
    }
    #[cfg(not(target_os = "macos"))]
    fn group_members(&self) -> io::Result<Vec<sysinfo::Pid>> {
        let mut pids = Vec::new();
        // ponytail: bounded /proc enumeration; native pidfd/cgroup ownership is
        // needed only if hosts exceed this 16K process scan ceiling.
        for (count, entry) in std::fs::read_dir("/proc")?.enumerate() {
            if count >= 16384 {
                return Err(io::Error::other("Owned process scan exceeds its bound"));
            }
            let entry = entry?;
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<i32>().ok())
                .filter(|pid| *pid > 0)
            else {
                continue;
            };
            let group = unsafe { libc::getpgid(pid) };
            if group < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
                return Err(io::Error::other("Owned process group is unavailable"));
            }
            if group == self.child.id() as i32 {
                pids.push(sysinfo::Pid::from_u32(pid as u32));
                if pids.len() >= 4096 {
                    return Err(io::Error::other("Owned process group exceeds its bound"));
                }
            }
        }
        Ok(pids)
    }
    #[cfg(target_os = "macos")]
    fn only_exited_leader(&self) -> bool {
        // Darwin rejects a group containing only the unreaped zombie with EPERM.
        self.group_members()
            .is_ok_and(|pids| pids.iter().all(|pid| pid.as_u32() == self.child.id()))
    }
    pub(super) fn active(&mut self) -> io::Result<bool> {
        if self.exit_code()?.is_none() {
            return Ok(true);
        }
        let pids: Vec<_> = self
            .group_members()?
            .into_iter()
            .filter(|pid| pid.as_u32() != self.child.id())
            .collect();
        if pids.is_empty() {
            return Ok(false);
        }
        let mut system = sysinfo::System::new();
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&pids),
            true,
            sysinfo::ProcessRefreshKind::nothing(),
        );
        for pid in pids {
            if let Some(process) = system.process(pid) {
                if process.status() != sysinfo::ProcessStatus::Zombie {
                    return Ok(true);
                }
            } else {
                // Only a confirmed disappearance can authorize cleanup.
                let group = unsafe { libc::getpgid(pid.as_u32() as i32) };
                if group == self.child.id() as i32
                    || (group < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH))
                {
                    return Err(io::Error::other("Owned process activity is unavailable"));
                }
            }
        }
        Ok(false)
    }
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
        let mut status: libc::siginfo_t = unsafe { std::mem::zeroed() };
        unsafe {
            checked(libc::waitid(
                libc::P_PID,
                self.child.id() as _,
                &mut status,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            ))?;
            if status.si_pid() == 0 {
                return Ok(None);
            }
            let code = status.si_status();
            Ok(Some(if status.si_code == libc::CLD_EXITED {
                code
            } else {
                -code
            }))
        }
    }
    pub(super) fn stop(&mut self) -> io::Result<bool> {
        let running = self.exit_code()?.is_none();
        // Include still-owned background children after observing the leader exit.
        let result = unsafe { libc::kill(-(self.child.id() as i32), libc::SIGTERM) };
        if result < 0 && !running && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
        {
            return Ok(false);
        }
        #[cfg(target_os = "macos")]
        if result < 0
            && !running
            && io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
            && self.only_exited_leader()
        {
            return Ok(false);
        }
        checked(result)?;
        Ok(running)
    }
}
