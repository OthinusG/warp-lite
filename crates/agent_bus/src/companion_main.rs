//! Clean stdio endpoint invoked explicitly through verified system SSH.
#[tokio::main]
async fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("forward") => {
            let result = match std::env::args().nth(2) {
                Some(endpoint) if std::env::args_os().len() == 3 => {
                    warp_agent_bus::session::forward(&endpoint).await
                }
                _ => Err(anyhow::anyhow!("Invalid relay arguments")),
            };
            if result.is_err() {
                eprintln!("Companion MCP relay unavailable");
                std::process::exit(1);
            }
            return;
        }
        Some("--bound-agent") => {
            let result = async {
                let mut arguments = std::env::args_os().skip(2);
                let program = arguments
                    .next()
                    .and_then(|s| s.into_string().ok())
                    .ok_or_else(|| anyhow::anyhow!("Missing Agent program"))?;
                let executable = std::path::PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("Missing native executable"))?,
                );
                anyhow::ensure!(
                    executable.is_absolute(),
                    "Native executable must be absolute"
                );
                warp_agent_bus::mcp::Bridge::from_env()?;
                let options = if program == "codex" {
                    let help = tokio::time::timeout(
                        std::time::Duration::from_secs(8),
                        tokio::process::Command::new(&executable)
                            .arg("--help")
                            .kill_on_drop(true)
                            .output(),
                    )
                    .await??;
                    anyhow::ensure!(help.status.success(), "Native CLI capability probe failed");
                    warp_agent_bus::launch::LaunchOptions::from_help(
                        &program,
                        &String::from_utf8_lossy(&help.stdout),
                    )
                } else {
                    Default::default()
                };
                warp_agent_bus::session::launch(
                    &warp_agent_bus::session::NativeLaunch {
                        program,
                        executable,
                        options,
                    },
                    arguments.collect(),
                )
                .await
            }
            .await;
            match result {
                Ok(status) => std::process::exit(status),
                Err(_) => {
                    eprintln!("Bound Agent launch unavailable");
                    std::process::exit(1);
                }
            }
        }
        Some("agent") => match launch_agent().await {
            Ok(status) => std::process::exit(status),
            Err(_) => {
                eprintln!("SSH Agent connection unavailable");
                std::process::exit(1);
            }
        },
        _ => (),
    }
    if std::env::args_os().len() == 2
        && std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("mcp"))
    {
        use rmcp::ServiceExt;
        let result = async {
            warp_agent_bus::mcp::Bridge::from_env()?
                .serve(warp_agent_bus::mcp::legacy_transport(rmcp::transport::stdio()).await?)
                .await?
                .waiting()
                .await?;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if result.is_err() {
            eprintln!("Companion MCP binding unavailable");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args_os().len() == 2
        && std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--version"))
    {
        println!(
            "warpai-companion {} protocol {}",
            env!("CARGO_PKG_VERSION"),
            remote_protocol::managed::PROTOCOL_MAJOR
        );
        return;
    }
    let service = std::env::args_os().len() == 2
        && std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--service"));
    if std::env::args_os().count() != 1 && !service {
        eprintln!("Companion accepts managed control on stdio only");
        std::process::exit(2);
    }
    let result = if service {
        warp_agent_bus::companion::serve_account_service().await
    } else {
        warp_agent_bus::companion::serve_stdio().await
    };
    if result.is_err() {
        // Peer bytes, native paths and OS diagnostics never enter protocol stderr.
        eprintln!("Companion control channel closed");
        std::process::exit(1);
    }
}

/// Explicit launch inside an existing SSH terminal; no shell command evaluation.
async fn launch_agent() -> anyhow::Result<i32> {
    use remote_protocol::proto::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut arguments = std::env::args().skip(2);
    let root = arguments
        .next()
        .ok_or_else(|| anyhow::anyhow!("Missing project root"))?;
    let program = arguments
        .next()
        .ok_or_else(|| anyhow::anyhow!("Missing Agent program"))?;
    let executable = arguments
        .next()
        .ok_or_else(|| anyhow::anyhow!("Missing executable"))?;
    let _terminal_mode = TerminalMode::raw()?;
    let mut dimensions = terminal_size();
    let mut client = warp_agent_bus::ssh_remote::HostClient::connect_local(&root)
        .await
        .map_err(|_| anyhow::anyhow!("Private project service unavailable"))?;
    let mut state = client
        .terminal_launch(TerminalLaunch {
            fence: client.fence().cloned(),
            session_id: uuid::Uuid::new_v4().to_string(),
            executable,
            arguments: arguments.collect(),
            columns: dimensions.0,
            rows: dimensions.1,
            agent_program: Some(program),
        })
        .await
        .map_err(|_| anyhow::anyhow!("Agent launch unavailable"))?;
    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut buffer = [0; 4096];
    let mut offset = 0;
    let mut sequence = 0;
    let mut input_open = true;
    loop {
        let mut control = TerminalControl {
            fence: client.fence().cloned(),
            session_id: state.session_id.clone(),
            run_id: state.run_id.clone(),
            action: TerminalAction::TerminalRead as i32,
            output_offset: offset,
            ..Default::default()
        };
        state = client
            .terminal_control(control.clone())
            .await
            .map_err(|_| anyhow::anyhow!("Agent control unavailable"))?;
        anyhow::ensure!(!state.output_truncated, "Agent output exceeded its buffer");
        stdout.write_all(&state.output).await?;
        stdout.flush().await?;
        offset = state.output_offset + state.output.len() as u64;
        if state.processes_active == Some(false)
            && state.output_closed
            && offset == state.output_end
        {
            control.action = TerminalAction::TerminalRelease as i32;
            control.output_offset = 0;
            client
                .terminal_control(control)
                .await
                .map_err(|_| anyhow::anyhow!("Agent release unavailable"))?;
            return Ok(state.exit_code.unwrap_or(1));
        }
        if offset < state.output_end {
            continue;
        }
        let resized = terminal_size();
        if resized != dimensions {
            let mut resize = control.clone();
            resize.action = TerminalAction::TerminalResize as i32;
            resize.output_offset = 0;
            resize.columns = resized.0;
            resize.rows = resized.1;
            client
                .terminal_control(resize)
                .await
                .map_err(|_| anyhow::anyhow!("Agent resize unavailable"))?;
            dimensions = resized;
        }
        tokio::select! {
            count = stdin.read(&mut buffer), if input_open => {
                let count = count?;
                control.output_offset = 0;
                if count == 0 {
                    input_open = false;
                    control.action = TerminalAction::TerminalStop as i32;
                } else {
                    sequence += 1;
                    control.action = TerminalAction::TerminalInput as i32;
                    control.input = buffer[..count].to_vec();
                    control.input_sequence = sequence;
                }
                client.terminal_control(control).await.map_err(|_| anyhow::anyhow!("Agent input unavailable"))?;
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(20)) => ()
        }
    }
}

#[cfg(unix)]
struct TerminalMode(Option<libc::termios>);
#[cfg(unix)]
impl TerminalMode {
    fn raw() -> anyhow::Result<Self> {
        use std::io::IsTerminal;
        if !std::io::stdin().is_terminal() {
            return Ok(Self(None));
        }
        unsafe {
            let mut original: libc::termios = std::mem::zeroed();
            anyhow::ensure!(
                libc::tcgetattr(0, &mut original) == 0,
                "Terminal mode unavailable"
            );
            let mut raw = original;
            libc::cfmakeraw(&mut raw);
            anyhow::ensure!(
                libc::tcsetattr(0, libc::TCSANOW, &raw) == 0,
                "Terminal mode unavailable"
            );
            Ok(Self(Some(original)))
        }
    }
}
#[cfg(unix)]
impl Drop for TerminalMode {
    fn drop(&mut self) {
        if let Some(original) = &self.0 {
            unsafe {
                libc::tcsetattr(0, libc::TCSANOW, original);
            }
        }
    }
}
#[cfg(windows)]
struct TerminalMode(
    Vec<(
        windows::Win32::Foundation::HANDLE,
        windows::Win32::System::Console::CONSOLE_MODE,
    )>,
);
#[cfg(windows)]
impl TerminalMode {
    fn raw() -> anyhow::Result<Self> {
        use std::io::IsTerminal;
        use windows::Win32::System::Console::*;
        let mut modes = Self(Vec::new());
        if !std::io::stdin().is_terminal() {
            return Ok(modes);
        }
        unsafe {
            for (id, input) in [(STD_INPUT_HANDLE, true), (STD_OUTPUT_HANDLE, false)] {
                let handle = GetStdHandle(id)?;
                let mut original = CONSOLE_MODE::default();
                GetConsoleMode(handle, &mut original)?;
                let mode = if input {
                    (original & !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT))
                        | ENABLE_VIRTUAL_TERMINAL_INPUT
                } else {
                    original | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING
                };
                modes.0.push((handle, original));
                SetConsoleMode(handle, mode)?;
            }
        }
        Ok(modes)
    }
}
#[cfg(windows)]
impl Drop for TerminalMode {
    fn drop(&mut self) {
        for (handle, original) in &self.0 {
            unsafe {
                let _ = windows::Win32::System::Console::SetConsoleMode(*handle, *original);
            }
        }
    }
}
fn terminal_size() -> (u32, u32) {
    #[cfg(unix)]
    unsafe {
        let mut size: libc::winsize = std::mem::zeroed();
        if libc::ioctl(0, libc::TIOCGWINSZ, &mut size) == 0
            && (1..=1000).contains(&size.ws_col)
            && (1..=1000).contains(&size.ws_row)
        {
            return (size.ws_col as u32, size.ws_row as u32);
        }
    }
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Console::*;
        if let Ok(handle) = GetStdHandle(STD_OUTPUT_HANDLE) {
            let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
            if GetConsoleScreenBufferInfo(handle, &mut info).is_ok() {
                let columns = i32::from(info.srWindow.Right) - i32::from(info.srWindow.Left) + 1;
                let rows = i32::from(info.srWindow.Bottom) - i32::from(info.srWindow.Top) + 1;
                if (1..=1000).contains(&columns) && (1..=1000).contains(&rows) {
                    return (columns as u32, rows as u32);
                }
            }
        }
    }
    (80, 24)
}
