use anyhow::Result;
use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	init_env();
	McpServer::new().await?.serve_stdio().await
}
