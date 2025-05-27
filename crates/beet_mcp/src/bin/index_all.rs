use anyhow::Result;
use beet_mcp::prelude::*;



#[tokio::main]
async fn main() -> Result<()> {
	std::fs::remove_dir_all(".cache/repo-dbs").ok();
	init_tracing(tracing::Level::INFO);
	let model = EmbedModel::mxbai_large();
	IndexRepository::new(model)
		.index_all_known_crates(|(key, _)| {
			matches!(
				key.content_type,
				ContentType::Docs // | ContentType::Examples | ContentType::Guides // ContentType::Internals // entire src dir, very slow
			) && key.crate_meta.crate_version == "0.16.0"
		})
		.await?;
	Ok(())
}
