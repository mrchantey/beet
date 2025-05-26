use beet_mcp::prelude::*;
use rig::embeddings::EmbeddingModel;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	init_tracing();
	let model = EmbedModel::mxbai_large();

	let content = "**Luminaris**: The central city built around the Nexus Spire, where reality is most stable. Home to the Council of Archons who govern the fragmented realms.";
	let embedding = model.embed_text(content).await?;
	println!("Embedding for content: {:#?}", embedding);
	Ok(())
}
