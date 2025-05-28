use anyhow::Result;
use beet_mcp::prelude::*;



#[tokio::main]
#[rustfmt::skip]
async fn main() -> Result<()> {
	// std::fs::remove_dir_all(".cache/").ok();
	init_tracing(tracing::Level::INFO);
	let model = EmbedModel::from_env();
	IndexRepository::new(model &KNOWN_SOURCES)
		.try_index_all_known_crates(|(key, _)| {
			// TODO clap args
			matches!(
				key.content_type,
				|	ContentType::Docs
				| ContentType::Examples
				| ContentType::Guides
				// the slowest one, indexes all source code, takes about 5 mins for bevy 0.16
				// | ContentType::Internals
			) && key.crate_meta.crate_version == "0.16.0"
		})
		.await?;
	Ok(())
}
