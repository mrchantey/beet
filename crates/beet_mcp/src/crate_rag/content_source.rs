use super::ContentType;
use super::CrateMeta;
use crate::prelude::SplitText;
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
	pub fn bevy_16_guides() -> Self {
		Self::new("bevy", "0.16.0", ContentType::Guides)
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
	/// ie the connection string to the database. Each crate has a seperate
	/// database for each of the scopes.
	pub fn local_db_path(&self) -> AbsPathBuf {
		WorkspacePathBuf::default()
			.into_abs()
			.unwrap()
			.join(format!(".cache/repo-dbs/{self}.db"))
	}
}

impl std::fmt::Display for ContentSourceKey {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}@{}/{}",
			self.crate_meta.crate_name,
			self.crate_meta.crate_version,
			self.content_type
		)
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
			.join(format!(".cache/repos/{}/{}", author, repo_name))
	}
}
