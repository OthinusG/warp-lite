//! System ConPTY and owned process job, independent of bundled desktop DLLs.
use super::*;
use std::{
    ffi::{OsStr, OsString},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle},
    },
};
use windows::{
    core::{Owned, HSTRING, PCWSTR, PWSTR},
    Win32::{
        Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::{
            Console::{ClosePseudoConsole, CreatePseudoConsole, ResizePseudoConsole, COORD, HPCON},
            JobObjects::*,
            Pipes::CreatePipe,
            Threading::*,
        },
    },
};

struct Console(HPCON);
impl Drop for Console {
    fn drop(&mut self) {
        unsafe {
            ClosePseudoConsole(self.0);
        }
    }
}
struct Attributes {
    data: Vec<usize>,
}
impl Attributes {
    fn pointer(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        LPPROC_THREAD_ATTRIBUTE_LIST(self.data.as_mut_ptr().cast())
    }
    unsafe fn new(console: HPCON) -> windows::core::Result<Self> {
        let mut bytes = 0;
        let _ = InitializeProcThreadAttributeList(None, 1, None, &mut bytes);
        if bytes == 0 || bytes > 65536 {
            return Err(windows::core::Error::from_thread());
        }
        let mut data = vec![0usize; bytes.div_ceil(std::mem::size_of::<usize>())];
        InitializeProcThreadAttributeList(
            Some(LPPROC_THREAD_ATTRIBUTE_LIST(data.as_mut_ptr().cast())),
            1,
            None,
            &mut bytes,
        )?;
        let mut attributes = Self { data };
        UpdateProcThreadAttribute(
            attributes.pointer(),
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
            Some(console.0 as _),
            std::mem::size_of::<HPCON>(),
            None,
            None,
        )?;
        Ok(attributes)
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        unsafe {
            DeleteProcThreadAttributeList(self.pointer());
        }
    }
}

pub(super) struct Process {
    handle: Owned<HANDLE>,
    job: Owned<HANDLE>,
    console: Option<Console>,
}
// These are independently owned kernel handles; the service serializes process control.
unsafe impl Send for Process {}
impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            let _ = TerminateJobObject(*self.job, 1);
        }
    }
}
fn size(value: Size) -> COORD {
    COORD {
        X: value.columns as i16,
        Y: value.rows as i16,
    }
}
fn error(value: windows::core::Error) -> io::Error {
    io::Error::other(value)
}

/// The existing desktop Windows argv encoder; filenames never become a shell program.
fn append_quoted(arg: &OsStr, command: &mut Vec<u16>) {
    if !arg.is_empty()
        && !arg
            .encode_wide()
            .any(|c| matches!(c, 32 | 9 | 10 | 11 | 34))
    {
        command.extend(arg.encode_wide());
        return;
    }
    command.push(34);
    let arg: Vec<_> = arg.encode_wide().collect();
    let mut index = 0;
    while index < arg.len() {
        let mut slashes = 0;
        while index < arg.len() && arg[index] == 92 {
            index += 1;
            slashes += 1;
        }
        if index == arg.len() {
            command.extend(std::iter::repeat_n(92, slashes * 2));
            break;
        }
        command.extend(std::iter::repeat_n(
            92,
            if arg[index] == 34 {
                slashes * 2 + 1
            } else {
                slashes
            },
        ));
        command.push(arg[index]);
        index += 1;
    }
    command.push(34);
}

fn environment(executable: &Path) -> io::Result<Vec<u16>> {
    let mut command = tokio::process::Command::new(executable);
    crate::session::without_terminal_binding(&mut command);
    command
        .env_remove("VIBE_MCP_SERVERS")
        .env("TERM", "xterm-256color");
    let mut values: Vec<(OsString, OsString)> = std::env::vars_os().collect();
    for (key, value) in command.as_std().get_envs() {
        values.retain(|(existing, _)| {
            !existing
                .to_string_lossy()
                .eq_ignore_ascii_case(&key.to_string_lossy())
        });
        if let Some(value) = value {
            values.push((key.into(), value.into()));
        }
    }
    values.sort_by_cached_key(|(key, _)| key.to_string_lossy().to_uppercase());
    let mut block = Vec::new();
    for (key, value) in values {
        block.extend(key.encode_wide());
        block.push(61);
        block.extend(value.encode_wide());
        block.push(0);
        if block.len() > 524288 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
    }
    block.push(0);
    Ok(block)
}

pub(super) fn spawn(
    executable: &Path,
    args: &[String],
    root: &Path,
    dimensions: Size,
) -> io::Result<NativePty> {
    unsafe {
        fn pipe() -> io::Result<(File, File)> {
            let mut read = HANDLE::default();
            let mut write = HANDLE::default();
            unsafe {
                CreatePipe(&mut read, &mut write, None, 65536).map_err(error)?;
            }
            Ok((unsafe { File::from_raw_handle(read.0) }, unsafe {
                File::from_raw_handle(write.0)
            }))
        }
        let (input, writer) = pipe()?;
        let (reader, output) = pipe()?;
        let console = Console(
            CreatePseudoConsole(
                size(dimensions),
                HANDLE(input.as_raw_handle()),
                HANDLE(output.as_raw_handle()),
                0,
            )
            .map_err(error)?,
        );
        let mut attributes = Attributes::new(console.0).map_err(error)?;
        let mut startup = STARTUPINFOEXW::default();
        startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        // As in the desktop PTY, let ConPTY supply handles instead of inheriting
        // the account service's redirected/null standard handles.
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.lpAttributeList = attributes.pointer();
        let mut command = Vec::new();
        append_quoted(executable.as_os_str(), &mut command);
        for arg in args {
            command.push(32);
            append_quoted(OsStr::new(arg), &mut command);
        }
        command.push(0);
        let environment = environment(executable)?;
        let executable = HSTRING::from(executable.as_os_str());
        let root = HSTRING::from(root.as_os_str());
        let job = Owned::new(CreateJobObjectW(None, PCWSTR::null()).map_err(error)?);
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            *job,
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            std::mem::size_of_val(&limits) as u32,
        )
        .map_err(error)?;
        let mut info = PROCESS_INFORMATION::default();
        CreateProcessW(
            &executable,
            Some(PWSTR(command.as_mut_ptr())),
            None,
            None,
            false,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT | CREATE_SUSPENDED,
            Some(environment.as_ptr().cast()),
            &root,
            &startup.StartupInfo,
            &mut info,
        )
        .map_err(error)?;
        let handle = Owned::new(info.hProcess);
        let thread = Owned::new(info.hThread);
        if let Err(failure) = AssignProcessToJobObject(*job, *handle) {
            let _ = TerminateProcess(*handle, 1);
            return Err(error(failure));
        }
        let process = Process {
            handle,
            job,
            console: Some(console),
        };
        if ResumeThread(*thread) == u32::MAX {
            return Err(io::Error::last_os_error());
        }
        drop(input);
        drop(output);
        Ok(NativePty {
            reader,
            writer,
            process,
        })
    }
}
impl Process {
    pub(super) fn active(&self) -> io::Result<bool> {
        let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        unsafe {
            QueryInformationJobObject(
                Some(*self.job),
                JobObjectBasicAccountingInformation,
                (&mut info as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                std::mem::size_of_val(&info) as u32,
                None,
            )
            .map_err(error)?;
        }
        Ok(info.ActiveProcesses != 0)
    }
    pub(super) fn resize(&self, _: &File, dimensions: Size) -> io::Result<()> {
        let console = self.console.as_ref().ok_or(io::ErrorKind::NotConnected)?;
        unsafe { ResizePseudoConsole(console.0, size(dimensions)).map_err(error) }
    }
    pub(super) fn exit_code(&mut self) -> io::Result<Option<i32>> {
        unsafe {
            match WaitForSingleObject(*self.handle, 0) {
                WAIT_TIMEOUT => Ok(None),
                WAIT_OBJECT_0 => {
                    let mut code = 0;
                    GetExitCodeProcess(*self.handle, &mut code).map_err(error)?;
                    // A shell's background children still own the console. Close
                    // only after the whole owned job exits so its pipe reaches EOF.
                    if !self.active()? {
                        self.console.take();
                    }
                    Ok(Some(code as i32))
                }
                _ => Err(io::Error::last_os_error()),
            }
        }
    }
    pub(super) fn stop(&mut self) -> io::Result<bool> {
        let running = self.exit_code()?.is_none();
        if self.active()? {
            unsafe {
                TerminateJobObject(*self.job, 1).map_err(error)?;
            }
        }
        Ok(running)
    }
}
