use anyhow::{bail, Result};
use rmcp::ServiceExt;
use warp_agent_bus::mcp::{Bridge, INSTRUCTIONS};

#[tokio::main]
async fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("mcp") => { Bridge::from_env()?.serve(rmcp::transport::stdio()).await?.waiting().await?; }
        Some("instructions") => println!("{INSTRUCTIONS}"),
        _ => bail!("Usage: warp-agent <mcp|instructions>. Configure this executable as a local stdio MCP server in a managed Warp Lite terminal."),
    }
    Ok(())
}
