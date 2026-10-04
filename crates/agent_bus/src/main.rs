use anyhow::{bail, Result};
use rmcp::ServiceExt;
use warp_agent_bus::mcp::{Bridge, INSTRUCTIONS};

#[tokio::main]
async fn main() -> Result<()> {
    if let Some(executable) = std::env::args().next().and_then(|invocation| warp_agent_bus::session::native_executable(&invocation)) {
        let status = warp_agent_bus::session::launch(&executable, std::env::args_os().skip(1).collect()).await?;
        std::process::exit(status);
    }
    match std::env::args().nth(1).as_deref() {
        Some("mcp") => { Bridge::from_env()?.serve(warp_agent_bus::mcp::legacy_transport(rmcp::transport::stdio()).await?).await?.waiting().await?; }
        Some("remote-stdio") => {
            warp_agent_bus::transport::retired_remote_stdio().await?;
        }
        Some("instructions") => println!("{INSTRUCTIONS}"),
        Some("forward") => {
            let endpoint = std::env::args().nth(2).ok_or_else(|| anyhow::anyhow!("Missing native MCP relay"))?;
            warp_agent_bus::session::forward(&endpoint).await?;
        }
        _ => bail!("Usage: warpai-agent <mcp|instructions>. Configure this executable as a local stdio MCP server in a managed Warp Lite terminal."),
    }
    Ok(())
}
