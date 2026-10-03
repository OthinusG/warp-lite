//! Clean stdio endpoint invoked explicitly through verified system SSH.
#[tokio::main]
async fn main() {
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
