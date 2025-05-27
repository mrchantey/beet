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
use sweet::prelude::GlobFilter;
use sweet::prelude::ReadDir;
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
///	    Document {
///	        id: "doc0".to_string(),
///	        content: "Definition of a *flurbo*: A flurbo is a green alien that lives on cold planets".to_string(),
///	    },
///	    Document {
///	        id: "doc1".to_string(),
///	        content: "Definition of a *glarb-glarb*: A glarb-glarb is a ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.".to_string(),
///	    },
///	    Document {
///	        id: "doc2".to_string(),
///	        content: "Definition of a *linglingdong*: A term used by inhabitants of the far side of the moon to describe humans.".to_string(),
///	    },
///	];
///
///	db.store(documents).await.unwrap();
///	let results = db.query(&RagQuery::new("ancient", 1)).await.unwrap();
///	assert_eq!(results.len(), 1);
///	assert_eq!(results[0].id, "doc1");
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
	pub async fn load_and_store_dir(
		&self,
		dir: impl AsRef<Path>,
		filter: GlobFilter,
	) -> Result<()> {
		let files = ReadDir::files_recursive(dir)?
			.into_iter()
			.filter(|file| filter.passes(file))
			.collect::<Vec<_>>();
		let num_files = files.len();
		tracing::info!("Loading {} files", num_files);
		for (i, file) in files.into_iter().enumerate() {
			tracing::info!("Loading file {}/{}", i + 1, num_files,);
			self.load_and_store_file(file).await?;
		}
		Ok(())
	}

	pub async fn load_and_store_file(
		&self,
		path: impl AsRef<Path>,
	) -> Result<()> {
		let path = path.as_ref();
		let content = tokio::fs::read_to_string(path).await?;
		self.split_and_store(path, &content).await
	}


	pub async fn split_and_store(
		&self,
		path: impl AsRef<Path>,
		content: &str,
	) -> Result<()> {
		let documents = SplitText::new(path.as_ref(), content)
			.split_to_documents()
			.into_iter()
			.map(|doc| Document {
				id: doc.id,
				content: doc.content,
			})
			.collect::<Vec<_>>();
		tracing::debug!(
			"Storing {} documents from {}",
			documents.len(),
			path.as_ref().to_string_lossy()
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
		let db = Database::connect(EmbedModel::all_minilm(), ":memory:")
			.await
			.unwrap();
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
		let results = db.query(&RagQuery::new("ancient", 1)).await.unwrap();
		assert_eq!(results.len(), 1);
		assert_eq!(results[0].id, "doc1");
	}
	#[tokio::test]
	async fn load_and_store() {
		let db = Database::connect(EmbedModel::all_minilm(), ":memory:")
			.await
			.unwrap();
		db.load_and_store_file("nexus_arcana.md").await.unwrap();

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
