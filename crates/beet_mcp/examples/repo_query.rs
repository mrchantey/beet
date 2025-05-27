use anyhow::Result;
use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	// clear db when changing repo or embedding model
	// std::fs::remove_file(db_path).ok();

	init_tracing(tracing::Level::INFO);
	// let repository = "https://github.com/bevyengine/bevy.git";
	// let embed_model = EmbedModel::all_minilm();
	let embedding_model = EmbedModel::mxbai_large();

	let crate_meta = CrateMeta {
		crate_name: "bevy".to_string(),
		crate_version: "0.16.0".to_string(),
	};
	let scope = CrateDocumentType::PublicApi;

	// IndexRepository::new(
	// 	"pokemon-info",
	// 	"https://github.com/minsoeaung/pokemon-info.git",
	// )
	IndexRepository::try_index(embedding_model.clone(), &crate_meta, scope)
		.await?;

	let results =
		Database::connect(embedding_model, &crate_meta.local_db_path(scope))
			.await?
			.query(&RagQuery::new("how to create a character controller", 10))
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
