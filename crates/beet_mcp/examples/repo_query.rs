use anyhow::Result;
use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	// clear db when changing repo or embedding model
	// std::fs::remove_file(db_path).ok();

	init_tracing(tracing::Level::INFO);
	// let repository = "https://github.com/bevyengine/bevy.git";
	// let embed_model = EmbedModel::all_minilm();
	let embedding_model = EmbedModel::from_env();

	let key = ContentSourceKey::new("bevy", "0.16.0", ContentType::Guides);

	// IndexRepository::new(
	// 	"pokemon-info",
	// 	"https://github.com/minsoeaung/pokemon-info.git",
	// )
	IndexRepository::new(embedding_model.clone())
		.try_index(&key)
		.await?;

	let db_path = key.local_db_path(&embedding_model);
	let results =
		Database::connect(embedding_model, &db_path.to_string_lossy())
			.await?
			.query(&RagQuery::new(
				"related! macro",
				// "how to create a character controller in version 0.16",
				10,
			))
			.await?;
	println!("Query results: {:#?}", results);

	// assert!(results.iter().any(|r| r.document.content.contains(
	// 	r#""name": "Ground",
	//       "immunes": ["Flying"],
	//       "weaknesses": ["Grass", "Bug"],
	//       "strengths": ["Fire", "Electric", "Poison", "Rock", "Steel"]"#
	// )));

	Ok(())
}
