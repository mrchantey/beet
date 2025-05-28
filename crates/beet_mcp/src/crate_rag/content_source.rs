use super::ContentType;
use super::CrateMeta;
use crate::prelude::SplitText;
use crate::utils::Model;
use rmcp::schemars;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use sweet::prelude::*;


/*
that also gets me thinking about the difference between the big expensive censored models and these local libre models, like in the 90s we got local 3d graphics which spawned the demo scene, what does that look like for ml
*/

#[derive(
	Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct ContentSourceKey {
	pub crate_meta: CrateMeta,
	pub content_type: ContentType,
}

impl ContentSourceKey {
	pub fn bevy_16_docs() -> Self {
		Self::new("bevy", "0.16.0", ContentType::Docs)
	}

	pub fn new(
		crate_name: &str,
		crate_version: &str,
		content_type: ContentType,
	) -> Self {
		Self {
			crate_meta: CrateMeta::new(crate_name, crate_version),
			content_type,
		}
	}
	/// ie the connection string to the vector database. Its important that
	/// the same model is used for each db, so the model must be provided.
	///
	/// the path uses several factors:
	/// - [`Model::model_name`]
	/// - [`CrateMeta::crate_name`]
	/// - [`CrateMeta::crate_version`]
	/// - [`ContentSourceKey::content_type`]
	///
	/// ### Example
	///
	/// An example path may look like:
	/// ```sh
	/// .cache/databases/mxbai-embed-large/bevy@0.16.0/docs.db
	/// ```
	///
	pub fn local_db_path<E: Model>(&self, embedding_model: &E) -> AbsPathBuf {
		WorkspacePathBuf::default()
			.into_abs()
			.unwrap()
			.join(format!(
				".cache/databases/{}/{}.db",
				embedding_model.model_name(),
				self,
			))
	}
}

impl std::fmt::Display for ContentSourceKey {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}/{}", self.crate_meta, self.content_type)
	}
}

#[derive(Debug, Clone)]
pub struct ContentSource {
	/// crate metadata
	pub crate_meta: CrateMeta,
	/// file filter to apply to the content source
	pub filter: GlobFilter,
	/// url to the git repository
	/// ie `https://github.com/BevyEngine/bevy.git`
	pub git_url: String,
	/// git commit hash of the source
	pub git_hash: String,
	/// the branch to git pull from
	pub git_branch: String,
	/// strategy for splitting text into chunks
	pub split_text: SplitText,
}


impl ContentSource {
	pub fn local_repo_path(&self) -> AbsPathBuf {
		let path = Path::new(&self.git_url);
		let author = path
			.parent()
			.and_then(|p| p.file_name())
			.and_then(|s| s.to_str())
			.unwrap_or("unknown_author");

		let repo_name = Path::new(&self.git_url)
			.file_stem()
			.and_then(|s| s.to_str())
			.unwrap_or("unknown_repo");

		WorkspacePathBuf::default()
			.into_abs()
			.unwrap()
			.join(format!(".cache/repositories/{}/{}", author, repo_name))
	}

	pub fn target_path(&self) -> AbsPathBuf {
		self.local_repo_path().join("target")
	}
}
