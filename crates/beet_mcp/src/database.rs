use crate::prelude::*;
use anyhow::Result;
use rig::Embed;
use rig::embeddings::EmbeddingsBuilder;
use rig::providers::openai;
use rig::providers::openai::Client;
use rig::providers::openai::TEXT_EMBEDDING_ADA_002;
use rig::vector_store::VectorStoreIndex;
use rig_sqlite::Column;
use rig_sqlite::ColumnValue;
use rig_sqlite::SqliteVectorStore;
use rig_sqlite::SqliteVectorStoreTable;
use rmcp::model::CallToolResult;
use rusqlite::ffi::sqlite3_auto_extension;
use serde::Deserialize;
use sqlite_vec::sqlite3_vec_init;
use std::env;
use std::path::Path;
use std::usize;
use tokio_rusqlite::Connection;


const MODEL: &'static str = TEXT_EMBEDDING_ADA_002;

/// Cheap to clone, just a reqwest client and a crossbeam sender
#[derive(Clone)]
pub struct Database {
	// 	path: String,
	pub vector_store: SqliteVectorStore<openai::EmbeddingModel, Document>,
	embedding_model: openai::EmbeddingModel,
}

impl Database {
	pub async fn connect(connection_string: &str) -> Result<Self> {
		if connection_string != ":memory:" {
			// Ensure the directory exists for persistent databases
			let path = Path::new(connection_string);
			if let Some(parent) = path.parent() {
				std::fs::create_dir_all(parent)?;
			}
		}

		unsafe {
			sqlite3_auto_extension(Some(std::mem::transmute(
				sqlite3_vec_init as *const (),
			)));
		}



		let openai_api_key =
			env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
		let openai_client = Client::new(&openai_api_key);



		let conn = Connection::open(connection_string).await?;

		let embedding_model = openai_client.embedding_model(MODEL);
		let vector_store =
			SqliteVectorStore::new(conn, &embedding_model).await?;

		Ok(Self {
			vector_store,
			embedding_model,
		})
	}

	pub async fn is_empty(&self) -> Result<bool> {
		Ok(self.query("foo", 10).await?.is_empty())
	}

	pub async fn load_and_store(
		&self,
		path: impl AsRef<Path>,
		splitter: SplitText,
	) -> Result<()> {
		let path = path.as_ref();
		let content = std::fs::read_to_string(path)?;
		self.split_and_store(&path.to_string_lossy(), &content, splitter)
			.await
	}
	pub async fn split_and_store(
		&self,
		id: &str,
		content: &str,
		splitter: SplitText,
	) -> Result<()> {
		let documents = splitter
			.split_to_documents(&id, &content)
			.into_iter()
			.map(|doc| Document {
				id: doc.id,
				content: doc.content,
			})
			.collect::<Vec<_>>();
		tracing::info!("Storing {} documents from {}", documents.len(), id);
		self.store(documents).await?;
		Ok(())
	}

	pub async fn store(&self, documents: Vec<Document>) -> Result<()> {
		let embeddings = EmbeddingsBuilder::new(self.embedding_model.clone())
			.documents(documents)?
			.build()
			.await?;

		self.vector_store.add_rows(embeddings).await?;
		Ok(())
	}

	pub async fn query(
		&self,
		query: &str,
		top_n: usize,
	) -> Result<Vec<QueryResult>> {
		let index = self
			.vector_store
			.clone()
			.index(self.embedding_model.clone());

		// Query the index
		let results = index
			.top_n::<Document>(query, top_n)
			.await?
			.into_iter()
			.map(|(score, id, document)| QueryResult {
				score,
				id,
				document,
			})
			.collect::<Vec<_>>();

		Ok(results)
	}

	pub async fn query_mcp(
		&self,
		query: &str,
		top_n: usize,
	) -> Result<CallToolResult, rmcp::Error> {
		let results = self
			.query(&query, top_n)
			.await
			.map_err(|e| {
				rmcp::Error::internal_error(
					"vector_db_query_error",
					Some(serde_json::json!({ "error": e.to_string() })),
				)
			})?
			.into_iter()
			.map(|r| r.into())
			.collect();

		Ok(CallToolResult::success(results))
	}
}

#[derive(Debug, Clone)]
pub struct QueryResult {
	pub score: f64,
	pub id: String,
	pub document: Document,
}

impl QueryResult {
	/// Convert the query result to a markdown string.
	pub fn to_markdown(&self) -> String {
		format!(
			"### Document ID: {}\n\n**Score:** {:.4}\n\n{}\n",
			self.id, self.score, self.document.content
		)
	}
}


#[derive(Debug, Clone, Embed, Deserialize, PartialEq, Eq, Hash)]
pub struct Document {
	/// id for tracing the origin of the document, this is ignored in the vector store
	pub id: String,
	#[embed]
	pub content: String,
}

impl SqliteVectorStoreTable for Document {
	fn name() -> &'static str { "documents" }

	fn schema() -> Vec<Column> {
		vec![
			Column::new("id", "TEXT PRIMARY KEY"),
			Column::new("content", "TEXT"),
		]
	}

	fn id(&self) -> String { self.id.clone() }

	fn column_values(&self) -> Vec<(&'static str, Box<dyn ColumnValue>)> {
		vec![
			("id", Box::new(self.id.clone())),
			("content", Box::new(self.content.clone())),
		]
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[tokio::test]
	async fn store() {
		dotenv::dotenv().ok();

		let db = Database::connect(":memory:").await.unwrap();
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

		db.store(documents).await.unwrap();
		let results = db.query("ancient", 1).await.unwrap();
		assert_eq!(results.len(), 1);
		assert_eq!(results[0].id, "doc1");
	}
	#[tokio::test]
	async fn load_and_store() {
		dotenv::dotenv().ok();

		let db = Database::connect("vector_stores/nexus_arcana.db")
			.await
			.unwrap();
		// let db = Database::connect(":memory:").await.unwrap();
		db.load_and_store("nexus_arcana.md", SplitText::Newline)
			.await
			.unwrap();

		let results = db.query("resonance", 1).await.unwrap();
		assert_eq!(results.len(), 1);
		assert_eq!(
			results[0].document.content,
			"- **Resonance**: The fundamental force that replaced conventional physics, allowing both magic and technology to function by attuning to different frequency bands."
		);
	}
}
