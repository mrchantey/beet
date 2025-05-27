use crate::prelude::*;
use anyhow::Result;
use rig::Embed;
use rig::embeddings::EmbeddingModel;
use rig::embeddings::EmbeddingsBuilder;
use rig::vector_store::VectorStoreIndex;
use rig_sqlite::Column;
use rig_sqlite::ColumnValue;
use rig_sqlite::SqliteVectorStore;
use rig_sqlite::SqliteVectorStoreTable;
use rmcp::schemars;
use rusqlite::ffi::sqlite3_auto_extension;
use serde::Deserialize;
use serde::Serialize;
use sqlite_vec::sqlite3_vec_init;
use std::path::Path;
use std::usize;
use tokio_rusqlite::Connection;



#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RagQuery {
	#[schemars(description = "The max results to return")]
	pub max_docs: usize,
	#[schemars(description = "The search term that will be \
		converted to an embedding and used to query the vector database\
		")]
	pub search_query: String,
}

impl RagQuery {
	/// Create a new RagQuery with the given search query and max docs.
	pub fn new(search_query: &str, max_docs: usize) -> Self {
		Self {
			max_docs,
			search_query: search_query.to_string(),
		}
	}
}


/// Database wrapper for a sqlite vector store.
///
/// ## Example
///
/// ```rust
/// # use beet_mcp::prelude::*;
/// # tokio_test::block_on(async {
/// let db = Database::connect(EmbedModel::mxbai_large(), ":memory:").await.unwrap();
///	let documents = vec![
///	    Document::new(
///	        "doc0",
///	        "- **Resonance**: The fundamental force that replaced conventional physics, allowing both magic and technology to function by attuning to different frequency bands.",
///	    ),
///	    Document::new(
///	        "doc1",
///	        "- **Echo Fragments**: Crystallized memories of lost realities that can be used as power sources or to temporarily recreate what was lost.",
///	    ),
///	    Document::new(
///	        "doc2",
///	        "- **Reality Anchors**: Ancient artifacts that stabilize regions against the constant flux of the Ethereal Sea.",
///	    ),
///	];
///
///	db.store(documents).await.unwrap();
///	let results = db.query(&RagQuery::new("resonance", 1)).await.unwrap();
///	assert_eq!(results.len(), 1);
///	assert_eq!(results[0].id, "doc0");
/// # })
/// ```
#[derive(Clone)]
pub struct Database<E: 'static + Clone + EmbeddingModel> {
	pub vector_store: SqliteVectorStore<E, Document>,
	embedding_model: E,
}


impl<E: EmbeddingModel> Database<E> {
	/// Connect to the database using the provided embedding model
	/// for storing and querying.
	/// The model used for querying must be the same as the one used
	/// to store the documents.
	pub async fn connect(
		embedding_model: E,
		connection_string: &str,
	) -> Result<Self> {
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

		let conn = Connection::open(connection_string).await?;

		let vector_store =
			SqliteVectorStore::new(conn, &embedding_model).await?;

		Ok(Self {
			vector_store,
			embedding_model,
		})
	}

	pub async fn is_empty(&self) -> Result<bool> {
		// expensive way to do this, but rig doesnt expose the conn
		Ok(self.query(&RagQuery::new("foo", 1)).await?.is_empty())
	}

	// TODO parallel split and group store


	pub async fn load_and_store_file(
		&self,
		splitter: &SplitText,
		path: impl AsRef<Path>,
	) -> Result<()> {
		let path = path.as_ref();
		let content = tokio::fs::read_to_string(path).await?;
		let documetns = splitter.split_to_documents(path, &content);
		self.store(documetns).await
	}


	pub async fn split_and_store(
		&self,
		split_text: &SplitText,
		path: impl AsRef<Path>,
		content: &str,
	) -> Result<()> {
		let path = path.as_ref();
		let documents = split_text.split_to_documents(path, content);
		tracing::debug!(
			"Storing {} documents from {}",
			documents.len(),
			path.to_string_lossy()
		);
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

	pub async fn query(&self, query: &RagQuery) -> Result<Vec<QueryResult>> {

		let index = self
			.vector_store
			.clone()
			.index(self.embedding_model.clone());

		// Query the index
		let results = index
			.top_n::<Document>(&query.search_query, query.max_docs)
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
impl std::fmt::Display for QueryResult {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.to_markdown())
	}
}

#[derive(Debug, Clone, Embed, Deserialize, PartialEq, Eq, Hash)]
pub struct Document {
	/// id for tracing the origin of the document
	pub id: String,
	#[embed]
	pub content: String,
}

impl Document {
	pub fn new(path: &str, content: &str) -> Self {
		// we want to index the path too its very relevent for the embeddings
		let joined_content = format!("uri: {path}\ncontent: {content}");

		Self {
			id: path.to_string(),
			content: joined_content,
		}
	}
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



// nexus arcana for testing
impl<E: 'static + Clone + EmbeddingModel> Database<E> {
	/// Connect to the Nexus Arcana test database,
	/// populating it with initial data if necessary.
	pub async fn nexus_arcana(embedding_model: E, path: &str) -> Result<Self> {
		let db = Self::connect(embedding_model, path).await?;

		if db.is_empty().await? {
			tracing::info!("initializing nexus arcana db");
			let content = include_str!("../../nexus_arcana.md");
			let documents = SplitText::default()
				.split_to_documents("nexus_arcana.db", content);
			db.store(documents).await?;
		} else {
			tracing::info!("connecting to nexus arcana db");
		}

		Ok(db)
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[tokio::test]
	async fn store() {
		let db = Database::connect(EmbedModel::all_minilm(), ":memory:")
			.await
			.unwrap();
		let documents = vec![
			Document::new(
				"doc0",
				"- **Resonance**: The fundamental force that replaced conventional physics, allowing both magic and technology to function by attuning to different frequency bands.",
			),
			Document::new(
				"doc1",
				"- **Echo Fragments**: Crystallized memories of lost realities that can be used as power sources or to temporarily recreate what was lost.",
			),
			Document::new(
				"doc2",
				"- **Reality Anchors**: Ancient artifacts that stabilize regions against the constant flux of the Ethereal Sea.",
			),
		];

		db.store(documents).await.unwrap();
		let results = db.query(&RagQuery::new("resonance", 1)).await.unwrap();
		assert_eq!(results.len(), 1);
		assert_eq!(results[0].id, "doc0");
	}
	#[tokio::test]
	async fn load_and_store() {
		let db = Database::nexus_arcana(EmbedModel::all_minilm(), ":memory:")
			.await
			.unwrap();

		let results = db.query(&RagQuery::new("resonance", 1)).await.unwrap();
		assert_eq!(results.len(), 1);
		assert!(
			results[0]
				.document
				.content
				.contains("## Core Concepts\n\n- **Resonance**"),
		);
	}
}
