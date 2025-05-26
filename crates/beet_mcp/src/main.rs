use anyhow::Result;
use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	init_tracing();
	McpServer::new(EmbedModel::mxbai_large())
		.await?
		.serve_stdio()
		.await
}
