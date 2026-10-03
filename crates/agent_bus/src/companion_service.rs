//! Native single-owner read service; stdio attachments never own its lifetime.
use super::*;
use std::{process::Stdio, sync::Arc, time::Duration};
use tokio::sync::Semaphore;

#[cfg(unix)]
type Stream = tokio::net::UnixStream;
#[cfg(windows)]
type Stream = tokio::net::windows::named_pipe::NamedPipeClient;

fn endpoint(directory: &Path) -> std::io::Result<String> {
    let identity = identity::Identity::open(directory)?;
    #[cfg(unix)]
    {
        let runtime =
            PathBuf::from("/tmp").join(format!("warpai-companion-{}", identity.service_id));
        identity::private_directory(&runtime)?;
        Ok(runtime.join("control.sock").to_string_lossy().into_owned())
    }
    #[cfg(windows)]
    {
        Ok(format!(
            r"\\.\pipe\warpai-companion-{}",
            identity.service_id
        ))
    }
}

async fn connect(endpoint: &str) -> std::io::Result<Stream> {
    #[cfg(unix)]
    {
        Stream::connect(endpoint).await
    }
    #[cfg(windows)]
    {
        tokio::net::windows::named_pipe::ClientOptions::new().open(endpoint)
    }
}

fn start_service() -> std::io::Result<()> {
    let mut command = tokio::process::Command::new(std::env::current_exe()?);
    command
        .arg("--service")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    crate::session::without_terminal_binding(&mut command);
    command.env_remove("VIBE_MCP_SERVERS");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Only async-signal-safe setsid runs after fork; no allocator or Rust lock.
        unsafe {
            command.as_std_mut().pre_exec(|| {
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.as_std_mut().creation_flags(0x00000008 | 0x00000200); // DETACHED_PROCESS | NEW_PROCESS_GROUP
    }
    let mut child = command.spawn()?;
    // Reap the child if this proxy remains alive through the idle service exit.
    tokio::spawn(async move {
        let _ = child.wait().await;
    });
    Ok(())
}

pub(super) async fn proxy_stdio() -> Result<(), ProtocolError> {
    let directory = data_directory()?;
    let endpoint = endpoint(&directory)?;
    let stream = match connect(&endpoint).await {
        Ok(stream) => stream,
        Err(_) => {
            start_service()?;
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    if let Ok(stream) = connect(&endpoint).await {
                        break stream;
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            })
            .await
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::TimedOut))?
        }
    };
    let (input, output) = tokio::io::split(stream);
    let mut input = input.compat();
    let mut output = output.compat_write();
    let mut stdin = tokio::io::stdin().compat();
    let mut stdout = tokio::io::stdout().compat_write();
    loop {
        let request: ClientMessage =
            match read_message_with_limit(&mut stdin, MAX_MANAGED_MESSAGE_SIZE).await {
                Ok(request) => request,
                Err(ProtocolError::UnexpectedEof) => return Ok(()),
                Err(error) => return Err(error),
            };
        let reply: ServerMessage = tokio::time::timeout(Duration::from_secs(10), async {
            write_message_with_limit(&mut output, &request, MAX_MANAGED_MESSAGE_SIZE).await?;
            read_message_with_limit(&mut input, MAX_MANAGED_MESSAGE_SIZE).await
        })
        .await
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::TimedOut))??;
        write_message_with_limit(&mut stdout, &reply, MAX_MANAGED_MESSAGE_SIZE).await?;
    }
}

pub(super) async fn serve() -> Result<(), ProtocolError> {
    let directory = data_directory()?;
    identity::private_directory(&directory)?;
    let lock = identity::private_file(&directory.join("service.lock"))?;
    lock.try_lock().map_err(std::io::Error::other)?;
    let endpoint = endpoint(&directory)?;
    let boot = Uuid::new_v4().to_string();
    #[cfg(unix)]
    let listener = {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::remove_file(&endpoint) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
        let listener = tokio::net::UnixListener::bind(&endpoint)?;
        std::fs::set_permissions(&endpoint, std::fs::Permissions::from_mode(0o600))?;
        listener
    };
    #[cfg(windows)]
    let mut listener =
        crate::transport::windows_pipe::create(&endpoint, true).map_err(std::io::Error::other)?;
    let slots = Arc::new(Semaphore::new(32));
    let mut idle_since = Instant::now();
    loop {
        let stream = tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                // No retained work is advertised yet. Owned runs must also fence idle exit.
                if slots.available_permits() != 32 { idle_since = Instant::now(); }
                if idle_since.elapsed() >= Duration::from_secs(60) { break; }
                continue;
            }
            accepted = async {
                #[cfg(unix)]
                { listener.accept().await.map(|(stream, _)| stream) }
                #[cfg(windows)]
                {
                    listener.connect().await?;
                    let next = crate::transport::windows_pipe::create(&endpoint, false)
                        .map_err(std::io::Error::other)?;
                    Ok::<_, std::io::Error>(std::mem::replace(&mut listener, next))
                }
            } => accepted?,
        };
        idle_since = Instant::now();
        let Ok(slot) = slots.clone().try_acquire_owned() else {
            continue;
        };
        let directory = directory.clone();
        let boot = boot.clone();
        tokio::spawn(async move {
            let _slot = slot;
            let _ = serve_channel(stream, &directory, &boot).await;
        });
    }
    #[cfg(unix)]
    let _ = std::fs::remove_file(&endpoint);
    drop(lock);
    Ok(())
}
