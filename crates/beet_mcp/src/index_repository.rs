use anyhow::Result;
use rig::embeddings::EmbeddingModel;
use rmcp::schemars;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::GlobFilter;

use crate::prelude::Database;
use std::collections::HashMap;
use std::sync::LazyLock;

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

impl CrateMeta {
	pub fn bevy_0_16_0() -> Self {
		Self::new("bevy", "0.16.0")
	}

	pub fn new(crate_name: &str, crate_version: &str) -> Self {
		Self {
			crate_name: crate_name.to_string(),
			crate_version: crate_version.to_string(),
		}
	}
	pub fn local_repo_path(&self) -> String {
		format!(".cache/repos/{}", self.crate_name)
	}
	/// ie the connection string to the database.
	pub fn local_db_path(&self) -> String {
		format!(
			".cache/repo-dbs/{}-{}.db",
			self.crate_name, self.crate_version
		)
	}
}


/// The value in a kvp of [`CrateMeta`] and [`RepoMeta`].
#[derive(Debug, Clone, Hash)]
pub struct RepoMeta {
	/// The git url of the repository.
	/// ie `https://github.com/BevyEngine/bevy.git`
	pub git_url: String,
	pub commit_hash: String,
}

pub static KNOWN_CRATES: LazyLock<HashMap<CrateMeta, RepoMeta>> =
	LazyLock::new(|| {
		[
			(
				CrateMeta {
					crate_name: "bevy".to_string(),
					crate_version: "0.16.0".to_string(),
				},
				RepoMeta {
					git_url: "https://github.com/BevyEngine/bevy.git"
						.to_string(),
					commit_hash: "e9418b3845c1ffc9624a3a4003bde66a2ad6566a"
						.to_string(),
				},
			),
			(
				CrateMeta {
					crate_name: "bevy".to_string(),
					crate_version: "0.8.0".to_string(),
				},
				RepoMeta {
					git_url: "https://github.com/BevyEngine/bevy.git"
						.to_string(),
					commit_hash: "0149c4145f0f398e9fba85c2584d0481a260f57c"
						.to_string(),
				},
			),
		]
		.into_iter()
		.collect()
	});

pub struct IndexRepository;

impl IndexRepository {
	/// Yup, its a big one, if using a cloud embedding model this could result in
	/// $5-$100 dollars in charges.
	pub async fn try_index_all<E: 'static + EmbeddingModel>(
		embed_model: E,
	) -> Result<()> {
		for (crate_meta, _) in KNOWN_CRATES.iter() {
			Self::try_index(embed_model.clone(), crate_meta).await?;
		}
		Ok(())
	}

	/// indexes the repo if the database is empty
	pub async fn try_index<E: 'static + EmbeddingModel>(
		embed_model: E,
		crate_meta: &CrateMeta,
	) -> Result<()> {
		let Some(repo_meta) = KNOWN_CRATES.get(&crate_meta) else {
			anyhow::bail!(
				"The git url for {}@{} is not known, please create a PR adding it to the `KNOWN_CRATES` map.",
				crate_meta.crate_name,
				crate_meta.crate_version
			);
		};
		let db_path = crate_meta.local_db_path();
		let repo_path = crate_meta.local_repo_path();

		let db = Database::connect(embed_model, &db_path).await?;


		if !std::fs::exists(&repo_path)? {
			tokio::fs::create_dir_all(&repo_path).await?;
			// Clone the repository
			tokio::process::Command::new("git")
				.arg("clone")
				.arg(&repo_meta.git_url)
				.arg(&repo_path)
				.spawn()?
				.wait()
				.await?;
		}
		if db.is_empty().await? {
			let start = std::time::Instant::now();
			// Pull the latest changes
			tokio::process::Command::new("git")
				.current_dir(&repo_path)
				.arg("pull")
				.spawn()?
				.wait()
				.await?;

			// Checkout the specific commit
			tokio::process::Command::new("git")
				.current_dir(&repo_path)
				.arg("checkout")
				.arg(&repo_meta.commit_hash)
				.spawn()?
				.wait()
				.await?;
			let filter = GlobFilter::default()
				.with_exclude("*.git*")
				.with_include("*.rs")
				.with_include("*.md");
			db.load_and_store_dir(repo_path, filter).await?;
			let elapsed = start.elapsed();

			let metadata = std::fs::metadata(db_path)?;
			let size_in_mb = metadata.len() as f64 / 1_048_576.0; // 1024*1024
			println!(
				"Success!\n \
				Vector Database size: {:.2} MB\n \
				Time elapsed: {:.2} minutes",
				size_in_mb,
				elapsed.as_secs_f64() / 60.0
			);
		}
		// tokio::fs::remove_dir_all(&repo_dir).await.ok();

		Ok(())
	}
}
