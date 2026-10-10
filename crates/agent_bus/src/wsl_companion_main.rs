//! Dedicated WSL guest entry; shares the account service without sharing SSH state.
#[path = "companion_main.rs"]
mod companion;

#[cfg(not(target_os = "linux"))]
compile_error!("The dedicated WSL Companion is a Linux guest binary");

#[tokio::main]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("uninstall") {
        if std::env::args_os().len() != 2 || warp_agent_bus::wsl_setup::uninstall().is_err() {
            eprintln!("Close active WSL Companion sessions before uninstalling");
            std::process::exit(1);
        }
    } else if std::env::args().nth(1).as_deref() == Some("setup") {
        if std::env::args_os().len() != 2 || warp_agent_bus::wsl_setup::run().is_err() {
            eprintln!("WSL MCP setup unavailable");
            std::process::exit(1);
        }
    } else if std::env::args().nth(1).as_deref() == Some("mcp") {
        let result = match std::env::args().nth(2) {
            Some(program) if std::env::args_os().len() == 3 => {
                warp_agent_bus::companion::serve_guest_mcp(program).await
            }
            _ => Err(anyhow::anyhow!("Missing native Agent program")),
        };
        if result.is_err() {
            eprintln!("WSL MCP binding unavailable");
            std::process::exit(1);
        }
    } else {
        companion::run().await;
    }
}
