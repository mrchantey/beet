use anyhow::Result;
use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	init_tracing(tracing::Level::INFO);
	McpServer::new(EmbedModel::from_env())
		.await?
		.serve_stdio()
		.await
}
