use anyhow::Result;
use rig::embeddings::EmbeddingModel;
use sweet::prelude::GlobFilter;

use crate::prelude::Database;



///
pub struct IndexRepository {
	name: String,
	/// url to the git repository
	/// ie `https://github.com/mrchantey/beet`
	git_url: String,
}


impl IndexRepository {
	pub fn new(name: &str, path: &str) -> Self {
		Self {
			name: name.to_string(),
			git_url: path.to_string(),
		}
	}


	pub async fn index_repo<E: EmbeddingModel>(
		&self,
		db: &Database<E>,
		filter: GlobFilter,
	) -> Result<()> {
		let repo_dir = format!("vector_stores/repo_cache/{}", self.name);
		if !std::fs::exists(&repo_dir)? {
			tokio::fs::create_dir_all(&repo_dir).await?;
			tokio::process::Command::new("git")
				.arg("clone")
				.arg("--depth=1")
				.arg(&self.git_url)
				.arg(&repo_dir)
				.spawn()?
				.wait()
				.await?;
		}

		// tokio::fs::remove_dir_all(&repo_dir).await.ok(); // ignore error if it doesn't exist


		db.load_and_store_dir(repo_dir, filter).await?;

		Ok(())
	}
}
