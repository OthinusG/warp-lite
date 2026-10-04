use anyhow::{bail, Result};
use rmcp::ServiceExt;
use warp_agent_bus::mcp::{Bridge, INSTRUCTIONS};

#[tokio::main]
async fn main() -> Result<()> {
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
        _ => bail!("Usage: warpai-agent <mcp|instructions>. Configure this executable as a local stdio MCP server in a managed Warpai terminal."),
    }
    Ok(())
}
