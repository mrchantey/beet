use beet_mcp::prelude::*;
use rig::embeddings::EmbeddingModel;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	init_tracing(tracing::Level::INFO);
	let model = EmbedModel::from_env();

	let content = "**Luminaris**: The central city built around the Nexus Spire, where reality is most stable. Home to the Council of Archons who govern the fragmented realms.";
	let embedding = model.embed_text(content).await?;
	println!("Embedding for content: {:#?}", embedding);
	Ok(())
}
