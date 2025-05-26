use anyhow::Result;
use beet_mcp::prelude::*;
use sweet::prelude::GlobFilter;

#[tokio::main]
async fn main() -> Result<()> {
	let db_path = "vector_stores/examples/repo_query.db";
	// clear db when changing repo or embedding model
	std::fs::remove_file(db_path).ok();

	init_tracing();
	// let repository = "https://github.com/bevyengine/bevy.git";
	// let embed_model = EmbedModel::all_minilm();
	let embed_model = EmbedModel::mxbai_large();
	let db = Database::connect(embed_model, db_path).await?;


	// IndexRepository::new(
	// 	"pokemon-info",
	// 	"https://github.com/minsoeaung/pokemon-info.git",
	// )
	IndexRepository::new(
		"bevy-engine",
		"https://github.com/BevyEngine/bevy.git",
	)
	.index_repo(
		&db,
		GlobFilter::default()
			.with_exclude("*.git*")
			.with_include("*.rs")
			.with_include("*.md"),
	)
	.await?;

	let results = db
		.query("what are the strengths of ground type pokemon?", 10)
		.await?;
	println!("Query results: {:#?}", results);

	assert!(results.iter().any(|r| r.document.content.contains(
		r#""name": "Ground",
        "immunes": ["Flying"],
        "weaknesses": ["Grass", "Bug"],
        "strengths": ["Fire", "Electric", "Poison", "Rock", "Steel"]"#
	)));

	Ok(())

	// McpServer::new(EmbedModel::mxbai_large())
	// 	.await?
	// 	.serve_stdio()
	// 	.await
}
