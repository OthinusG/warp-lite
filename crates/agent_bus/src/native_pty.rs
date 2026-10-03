//! Native PTY ownership without desktop services. The service retains this owner.
use std::{fs::File, io, path::Path};

#[cfg(unix)]
#[path = "native_pty_unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "native_pty_windows.rs"]
mod platform;

#[derive(Clone, Copy)]
pub struct Size {
    pub columns: u16,
    pub rows: u16,
}
impl Size {
    fn validate(self) -> io::Result<()> {
        if self.columns == 0 || self.rows == 0 || self.columns > 1000 || self.rows > 1000 {
            Err(io::ErrorKind::InvalidInput.into())
        } else {
            Ok(())
        }
    }
}

/// Dropping an attachment must not drop this service-owned PTY/process handle.
pub struct NativePty {
    pub reader: File,
    pub writer: File,
    process: platform::Process,
}
impl NativePty {
    pub fn spawn(executable: &Path, args: &[String], root: &Path, size: Size) -> io::Result<Self> {
        size.validate()?;
        if !executable.is_absolute()
            || executable
                .to_str()
                .is_none_or(|path| path.len() > 4096 || path.contains('\0'))
            || !root.is_absolute()
            || !root.is_dir()
            || args.len() > 64
            || args.iter().any(|s| s.contains('\0') || s.len() > 8192)
            || args.iter().map(String::len).sum::<usize>() > 65536
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        platform::spawn(executable, args, root, size)
    }
    pub fn resize(&self, size: Size) -> io::Result<()> {
        size.validate()?;
        self.process.resize(&self.writer, size)
    }
    /// None means still running; a stop request is not an observed exit.
    pub fn exit_code(&mut self) -> io::Result<Option<i32>> {
        self.process.exit_code()
    }
    pub fn stop(&mut self) -> io::Result<bool> {
        self.process.stop()
    }
}
