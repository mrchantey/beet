use anyhow::Result;
use beet_mcp::prelude::*;



#[tokio::main]
async fn main() -> Result<()> {
	std::fs::remove_dir_all(".cache/repo-dbs").ok();
	init_tracing(tracing::Level::INFO);
	let model = EmbedModel::mxbai_large();
	let scope = CrateDocumentType::PublicApi;
	// let scope = CrateQueryScope::Internals; takes over an hour
	IndexRepository::index_all_known_crates(model, scope).await?;
	Ok(())
}
