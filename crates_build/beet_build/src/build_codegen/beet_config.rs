use crate::prelude::*;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
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
package_name = "beet_site"
src_path = "crates/beet_site/src"

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


	#[test]
	fn serde2() {
		let config: &str = r###"
# This is the config file for both the beet site
# and a test site used for internal testing.
#
# It contains some custom parts to handle the internal nature
# of the beet site, an external project will be significantly simpler.

package_name = "beet_site"
src_path = "crates/beet_site/src"
docs_route = "/docs"
wasm_imports = ["use beet::design as beet_design;", "use beet::prelude::*;"]
file_group_routes = ["beet_design_mockups"]

[[file_group]]
name = "test_site_pages"
path = "crates_rsx/beet_router/src/test_site/pages"
output = "crates_rsx/beet_router/src/test_site/codegen/pages.rs"
include = ["*.rs"]
exclude = ["*mod.rs"]
package_name = "beet_router"
import_tokens = ["use crate::as_beet::*;"]







[[file_group]]
name = "beet_design_mockups"
# preset = "mockup"
package_name = "beet_design"
path = "crates_rsx/beet_design/src"
output = "crates_rsx/beet_design/src/codegen/mockups.rs"
include = ["*.mockup.*"]
base_route = "/design"
replace_route = [{ from = ".mockup", to = "" }]
import_tokens = [
	"#[allow(unused_imports)]use beet::prelude::*;",
	"use beet_router::as_beet::*;",
]


# [wasm]
# import_tokens = ["use beet::design as beet_design;", "use beet::prelude::*;"]
"###;

		// let values = ValueDeserializer::deserialize(config).unwrap();
		let _config: BeetConfig = toml::de::from_str(config).unwrap();
	}
}
