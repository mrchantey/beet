use std::path::Path;

use crate::prelude::*;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeetConfig {
	#[serde(rename = "file_group")]
	pub file_groups: Vec<FileGroupConfig>,
}


impl BeetConfig {
	pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
		Ok(toml::de::from_str(&ReadFile::to_string(path)?)?)
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
	#[serde(flatten)]
	pub file_group: FileGroup,
	#[serde(flatten)]
	pub codegen: CodegenFile,
}

impl FileGroupConfig {
	pub fn apply_codegen(self) -> Result<CodegenFile> {
		self.file_group
			.xpipe(FileGroupToFuncTokens::default())?
			.xpipe(FuncTokensToRsxRoutes::new(self.codegen.clone()))?
			.xmap(|(_, codegen)| codegen)
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
