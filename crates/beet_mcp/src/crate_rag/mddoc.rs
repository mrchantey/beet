use std::path::Path;

use super::ContentSource;
use anyhow::Result;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use rustdoc_md::rustdoc_json_to_markdown;
use rustdoc_md::rustdoc_json_types::Crate;
use sweet::prelude::*;

pub struct Mddoc<'a> {
	pub no_deps: bool,
	/// by default, any file that includes the crate name in the path,
	/// ie `bevy` will also build `bevy_ecs` docs.
	pub filter: GlobFilter,
	pub source: &'a ContentSource,
}

impl<'a> Mddoc<'a> {
	pub fn new(source: &'a ContentSource) -> Self {
		Self {
			no_deps: false,
			source,
			filter: GlobFilter::default()
				.with_include(&format!(
					"*target/doc/{}*.json",
					source.crate_meta.crate_name
				))
				.with_exclude("*.git*"),
		}
	}

	pub async fn build(&self) -> Result<()> {
		self.build_json().await?;
		self.build_matching_mdddocs().await?;
		Ok(())
	}

	/// build docs as json
	async fn build_json(&self) -> Result<()> {
		let target_path = self.source.target_path();
		// another commit may have some extra files lying around
		FsExt::remove(&target_path.join("doc")).ok();

		let mut cmd = tokio::process::Command::new("cargo");
		cmd.arg("doc")
			// .arg("--no-deps") cant do thie, ie bevy_ecs docs wouldnt be created
			.arg("--target-dir")
			.arg(&target_path.as_os_str())
			.arg("--manifest-path")
			.arg(&self.source.local_repo_path().join("Cargo.toml").as_os_str())
			.env("RUSTDOCFLAGS", "-Z unstable-options --output-format json");
		if self.no_deps {
			cmd.arg("--no-deps");
		}
		cmd.spawn()?.wait().await?;

		Ok(())
	}

	async fn build_matching_mdddocs(&self) -> Result<()> {
		ReadDir::files_recursive(&self.source.target_path().join("doc"))?
			.into_iter()
			.filter(|file| self.filter.passes(file))
			.par_bridge()
			.map(|path| self.build_mddoc(&path))
			.collect::<Result<Vec<_>>>()?;
		Ok(())
	}

	fn build_mddoc(&self, json_path: &Path) -> Result<()> {
		let file = ReadFile::to_string(&json_path)?;
		let data: Crate = serde_json::from_str(&file)?;

		let markdown = rustdoc_json_to_markdown(data);
		let file_stem = json_path.file_stem().ok_or_else(|| {
			anyhow::anyhow!(
				"Failed to get file stem for {}",
				json_path.display()
			)
		})?;

		let md_path = json_path
			.parent()
			.and_then(|p| p.parent())
			.ok_or_else(|| {
				anyhow::anyhow!(
					"Failed to get parent directory for {}",
					json_path.display()
				)
			})?
			.join("md")
			.join(file_stem)
			.with_extension("md");

		FsExt::write(&md_path, &markdown)?;

		tracing::trace!("Wrote markdown to {}", md_path.display());
		Ok(())
	}
}




#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[sweet::test]
	#[ignore = "takes long time but required for first run"]
	async fn build() {
		Mddoc::new(
			&KnownSources::get(&ContentSourceKey::bevy_16_docs()).unwrap(),
		)
		.build()
		.await
		.unwrap();
	}
	#[sweet::test]
	async fn mddoc_all() {
		Mddoc::new(
			&KnownSources::get(&ContentSourceKey::bevy_16_docs()).unwrap(),
		)
		.build_matching_mdddocs()
		.await
		.unwrap();
	}
}
