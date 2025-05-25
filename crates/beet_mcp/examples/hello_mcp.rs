/// Running this example without a client isnt very meaningful
/// i recommend the mcp inspector:
/// ```sh
/// npm i -g @modelcontextprotocol/inspector
/// npx @modelcontextprotocol/inspector cargo run --example hello_mcp
/// ```
use anyhow::Result;
use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	init_env();
	McpServer::new().await?.serve_stdio().await
}
