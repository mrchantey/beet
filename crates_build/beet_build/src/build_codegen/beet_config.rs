use std::path::Path;
use crate::prelude::*;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeetConfig {
	/// The [`FileGroupConfig::name`] for each group that should be
	/// collected into the route tree. Usually this includes pages
	/// but excludes actions.
	#[serde(default)]
	pub file_group_routes: Vec<String>,
	#[serde(rename = "file_group")]
	pub file_groups: Vec<FileGroupConfig>,
/// Configuration for a default site configuration.
	#[serde(flatten)]
	pub default_site_config: DefaultSiteConfig,
}


impl BeetConfig {
	pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
		Ok(toml::de::from_str(&ReadFile::to_string(path)?)?)
	}

	pub fn routes(&self) -> Result<Vec<FuncTokens>> {
		self.file_group_routes
			.iter()
			.map(|name| {
				self.file_groups
					.iter()
					.find(|group| group.name == *name)
					.ok_or_else(|| {
						anyhow::anyhow!("File group {} not found", name)
					})
					.map(|group| {
						group.to_func_tokens().map(|group| group.funcs)
					})
					.flatten()
			})
			.collect::<Result<Vec<_>>>()
			.map(|vecs| vecs.into_iter().flatten().collect())
	}


	#[cfg(test)]
	pub fn test_config() -> Self {
		let config = r#"
[[file_group]]
name = "test_site_pages"
path = "crates_rsx/beet_router/src/test_site/pages"
include = ["*.rs"]
exclude = ["*mod.rs"]
package_name = "beet_router"
output = "crates_rsx/beet_router/src/test_site/codegen/pages.rs"
import_tokens = ["use crate::as_beet::*;"]
"#;
		toml::de::from_str(config).unwrap()
	}
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileGroupConfig {
	name: String,
	#[serde(flatten)]
	pub file_group: FileGroup,
	#[serde(flatten)]
	pub codegen: CodegenFile,
	#[serde(flatten)]
	pub map_tokens: MapFuncTokens,
}

impl FileGroupConfig {
	/// Convert this config to a [`FuncTokensGroup`]
	pub fn to_func_tokens(&self) -> Result<FuncTokensGroup> {
		self.file_group
			.clone()
			.xpipe(FileGroupToFuncTokens::default())?
			.xpipe(self.map_tokens.clone())
			.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn serde() {
		let config = BeetConfig::test_config();
		let str = toml::ser::to_string(&config).unwrap();
		let config2: BeetConfig = toml::de::from_str(&str).unwrap();
		expect(config).to_be(config2);
	}
}
