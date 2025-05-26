use anyhow::Result;
use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	// let repository = "https://github.com/bevyengine/bevy.git";
	let repository = "https://github.com/Calvin-LL/is-even-ai";

	init_tracing();
	McpServer::new(EmbedModel::mxbai_large())
		.await?
		.serve_stdio()
		.await
}



