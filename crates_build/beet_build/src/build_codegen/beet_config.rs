use crate::prelude::*;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeetConfig {
	pub file_group: Vec<FileGroupConfig>,
}


impl BeetConfig {
	pub fn apply_codegen(self) -> Result<Vec<CodegenFile>> {
		self.file_group
			.into_iter()
			.map(|file_group| file_group.apply_codegen())
			.collect()
	}
	/// Build and write all codegen files
	pub fn build_and_write(self) -> Result<()> {
		let codegens = self.apply_codegen()?;
		for codegen in codegens {
			codegen.build_and_write()?;
		}
		Ok(())
	}
}


#[derive(Debug, Clone, Serialize, Deserialize)]
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

	const CONFIG: &str = r#"
[[file_group]]
name = "test_site_pages"
path = "crates_rsx/beet_router/src/test_site/pages"
include = ["*.rs"]
exclude = ["*mod.rs"]
package_name = "beet_router"
output = "crates_rsx/beet_router/src/test_site/codegen/pages.rs"
import_tokens = ["use crate::as_beet::*;"]
"#;

	#[test]
	fn works() {
		let config: BeetConfig = toml::de::from_str(CONFIG).unwrap();
		expect(config.file_group.len()).to_be(1);
	}

	#[test]
	fn apply_codegen() {
		let config: BeetConfig = toml::de::from_str(CONFIG).unwrap();
		let file_group = config.file_group[0].clone();
		let codegen = file_group.apply_codegen().unwrap();
		let str = codegen
			.build_output()
			.unwrap()
			.xmap(|file| prettyplease::unparse(&file));
		// println!("{}", str);
		expect(str).to_contain(
			"RouteFunc::new(RouteInfo::new(\"/docs\", HttpMethod::Get), file0::get)",
		);
	}
}
