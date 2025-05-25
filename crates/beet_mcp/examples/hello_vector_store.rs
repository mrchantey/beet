use std::fs;

use beet_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	init_env();

	let db = Database::connect("vector_stores/hello_vector_store.db").await?;

	// populating the database should only be done once
	if !fs::exists("vector_stores/hello_vector_store.db")? {
		println!("Populating the vector store with initial data...");
		let documents = vec![
            Document {
                id: "doc0".to_string(),
                content: "Definition of a *flurbo*: A flurbo is a green alien that lives on cold planets".to_string(),
            },
            Document {
                id: "doc1".to_string(), 
                content: "Definition of a *glarb-glarb*: A glarb-glarb is a ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.".to_string(),
            },
            Document {
                id: "doc2".to_string(),
                content: "Definition of a *linglingdong*: A term used by inhabitants of the far side of the moon to describe humans.".to_string(),
            },
            ];
		db.store(documents).await?;
	}

	let results = db.query("what tools do old people use", 1).await?;
	println!("Query results: {:#?}", results);

	Ok(())
}
