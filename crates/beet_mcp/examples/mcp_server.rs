/// Running this example without a client isnt very meaningful
/// i recommend the mcp inspector:
/// ```sh
/// npm i -g @modelcontextprotocol/inspector
/// npx @modelcontextprotocol/inspector cargo run --example mcp_server
/// ```
use anyhow::Result;
use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	init_tracing(tracing::Level::INFO);
	McpServer::new(EmbedModel::from_env(), KNOWN_SOURCES.clone())
		.await?
		.serve_stdio()
		.await
	// McpServer::serve_sse(EmbedModel::from_env(), "127.0.0.1:8000").await
}
