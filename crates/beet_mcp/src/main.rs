use anyhow::Result;
use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	#[cfg(debug_assertions)]
	init_tracing(tracing::Level::DEBUG);
	#[cfg(not(debug_assertions))]
	init_tracing(tracing::Level::INFO);
	McpServer::new(EmbedModel::from_env())
		.await?
		.serve_stdio()
		.await
}
