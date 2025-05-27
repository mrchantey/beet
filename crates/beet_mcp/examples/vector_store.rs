use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	init_tracing(tracing::Level::INFO);

	let db = Database::connect(
		EmbedModel::mxbai_large(),
		".cache/examples/vector_store.db",
	)
	.await?;

	// populating the database should only be done once
	if db.is_empty().await? {
		println!("Populating the vector store with initial data...");
		let documents = vec![
			Document::new(
				"doc0",
				"**Resonance**: The fundamental force that replaced conventional physics, allowing both magic and technology to function by attuning to different frequency bands.",
			),
			Document::new(
				"doc1", 
				"**Echo Fragments**: Crystallized memories of lost realities that can be used as power sources or to temporarily recreate what was lost.",
			),
			Document::new(
				"doc2",
				"**Reality Anchors**: Ancient artifacts that stabilize regions against the constant flux of the Ethereal Sea.",
			),
		];
		db.store(documents).await?;
	}

	let results = db
		.query(&RagQuery::new("how does resonance work", 1))
		.await?;
	println!("Query results: {:#?}", results);

	Ok(())
}
