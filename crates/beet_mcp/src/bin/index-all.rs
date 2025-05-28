use anyhow::Result;
use beet_mcp::prelude::*;



#[tokio::main]
#[rustfmt::skip]
async fn main() -> Result<()> {
	// std::fs::remove_dir_all(".cache/repo-dbs").ok();
	init_tracing(tracing::Level::INFO);
	let model = EmbedModel::from_env();
	IndexRepository::new(model)
		.try_index_all_known_crates(|(key, _)| {
			matches!(
				key.content_type,
				// |	ContentType::Docs
				| ContentType::Examples
				// | ContentType::Guides
				// | ContentType::Internals // the slowest one, every src dir, about 5 mins for bevy
			) && key.crate_meta.crate_version == "0.16.0"
		})
		.await?;
	Ok(())
}
