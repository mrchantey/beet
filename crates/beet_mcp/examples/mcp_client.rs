use anyhow::Result;
use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	init_tracing();
	let client = McpClient::new_stdio_dev().await?;
	let tools = client.list_tools().await?;
	println!("Tools: {:#?}", tools);

	let results = client.query_nexus("how does resonance work?", 2).await?;
	tracing::info!("Responses: {results:#?}");

	Ok(())
}
