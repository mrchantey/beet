use crate::prelude::ContentType;
use crate::prelude::Database;
use crate::prelude::Mddoc;
use crate::utils::BeetEmbedModel;
use anyhow::Result;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use rmcp::schemars;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;

use sweet::prelude::ReadDir;
use sweet::prelude::ReadFile;

use super::ContentSource;
use super::ContentSourceKey;
use super::KnownSources;


/// The key in a kvp of [`CrateMeta`] and [`RepoMeta`].
#[derive(
	Debug,
	Clone,
	Hash,
	PartialEq,
	Eq,
	Serialize,
	Deserialize,
	schemars::JsonSchema,
)]
pub struct CrateMeta {
	#[schemars(description = "The name of the crate, ie `bevy`")]
	pub crate_name: String,
	#[schemars(description = "The exact version of the crate, ie `0.16.0`")]
	pub crate_version: String,
}

impl Default for CrateMeta {
	fn default() -> Self { Self::bevy_0_16() }
}

impl CrateMeta {
	pub fn bevy_0_16() -> Self { Self::new("bevy", "0.16.0") }

	pub fn new(crate_name: &str, crate_version: &str) -> Self {
		Self {
			crate_name: crate_name.to_string(),
			crate_version: crate_version.to_string(),
		}
	}
}
impl std::fmt::Display for CrateMeta {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}@{}", self.crate_name, self.crate_version)
	}
}

pub struct IndexRepository<E: BeetEmbedModel> {
	embedding_model: E,
}

impl<E: BeetEmbedModel> IndexRepository<E> {
	pub fn new(embedding_model: E) -> Self { Self { embedding_model } }

	/// Yup, its a big one, if using a cloud embedding model this could result in
	/// $5-$100 dollars in charges.
	pub async fn try_index_all_known_crates(
		&self,
		filter: impl Fn(&(&ContentSourceKey, &ContentSource)) -> bool,
	) -> Result<()> {
		for (crate_meta, _) in KnownSources.iter().filter(filter) {
			self.try_index(crate_meta).await?;
		}
		Ok(())
	}

	/// indexes the repo if the database is empty
	pub async fn try_index(&self, key: &ContentSourceKey) -> Result<()> {
		let source = KnownSources::get(&key)?;
		let db_path = key.local_db_path(&self.embedding_model);
		let repo_path = source.local_repo_path();

		let db = Database::connect(
			self.embedding_model.clone(),
			&db_path.to_string_lossy(),
		)
		.await?;


		if !std::fs::exists(&repo_path)? {
			tokio::fs::create_dir_all(&repo_path).await?;
			// Clone the repository
			tokio::process::Command::new("git")
				.arg("clone")
				.arg(&source.git_url)
				.arg(&repo_path.as_os_str())
				.spawn()?
				.wait()
				.await?;
		}
		if db.is_empty().await? {
			tracing::info!("Indexing {}", key);

			let start = std::time::Instant::now();

			// Fetch the latest changes from remote without merging
			tokio::process::Command::new("git")
				.current_dir(&repo_path)
				.arg("fetch")
				.arg("origin")
				.spawn()?
				.wait()
				.await?;

			// Reset hard to the specific commit, discarding any local changes
			tokio::process::Command::new("git")
				.current_dir(&repo_path)
				.arg("reset")
				.arg("--hard")
				.arg(&source.git_hash)
				.spawn()?
				.wait()
				.await?;

			if key.content_type == ContentType::Docs {
				Mddoc::new(&source).build().await?;
			}

			self.load_and_store_dir(db, source, repo_path).await?;
			let elapsed = start.elapsed();

			let metadata = std::fs::metadata(db_path)?;
			let size_in_mb = metadata.len() as f64 / 1_048_576.0; // 1024*1024
			tracing::info!(
				"Success!\n \
				Vector Database size: {:.2} MB\n \
				Time elapsed: {}",
				size_in_mb,
				secs_to_str(elapsed.as_secs()),
			);
		}
		Ok(())
	}


	async fn load_and_store_dir(
		&self,
		db: Database<E>,
		source: &ContentSource,
		dir: impl AsRef<Path>,
	) -> Result<()> {
		let files = ReadDir::files_recursive(dir)?
			.into_iter()
			.filter(|file| source.filter.passes(file))
			.par_bridge()
			// .into_par_iter()
			.map(|path| {
				let content = ReadFile::to_string(&path)?;
				let documents =
					source.split_text.split_to_documents(path, &content);
				Ok(documents)
			})
			.collect::<Result<Vec<_>>>()?;
		let num_files = files.len();
		let chunks = files.into_iter().flatten().collect::<Vec<_>>();
		let num_chunks = chunks.len();
		tracing::info!(
			"Succesfully split {num_files} files into {num_chunks} chunks",
		);
		// Calculate estimated time based on 30 seconds per 1000 chunks
		let eta_seconds = (num_chunks as f64 * 0.03) as u64;
		tracing::info!(
			"generating embeddings.. eta: ~{}",
			secs_to_str(eta_seconds)
		);
		db.store(chunks).await?;

		Ok(())
	}
}

fn secs_to_str(secs: u64) -> String {
	let mins = secs / 60;
	let secs = secs % 60;
	format!("{:02}:{:02} minutes", mins, secs)
}
