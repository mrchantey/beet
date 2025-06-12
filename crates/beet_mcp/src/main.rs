use anyhow::Result;
use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	#[cfg(debug_assertions)]
	init_tracing(tracing::Level::DEBUG);
	#[cfg(not(debug_assertions))]
	init_tracing(tracing::Level::INFO);
	McpServer::new(EmbedModel::from_env(), KNOWN_SOURCES.clone())
		.await?
		.serve_stdio()
		.await
	// McpServer::serve_sse(EmbedModel::from_env(), "127.0.0.1:8000").await
}
