use anyhow::Result;
use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	init_tracing(tracing::Level::INFO);
	let client = McpClient::new_stdio_dev().await?;
	let tools = client.list_tools(Default::default()).await?;
	println!("Tools: {:#?}", tools);

	// let results = client
	// 	.nexus_rag(&RagQuery::new("how does resonance work?", 2))
	// 	.await?;
	// tracing::info!("Responses: {results:#?}");

	// let results = client
	// 	.crate_rag(CrateRagQuery {
	// 		rag_query: RagQuery::new(
	// 			"how do you create a simple scene with a 2d camera",
	// 			2,
	// 		),
	// 		crate_meta: CrateMeta::bevy_0_16(),
	// 		content_type: ContentType::Guides.into(),
	// 	})
	// 	.await?;
	// tracing::info!("Crate Responses: {results:#?}");

	Ok(())
}
