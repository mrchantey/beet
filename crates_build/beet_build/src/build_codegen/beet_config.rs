use serde::Deserialize;
use serde::Serialize;

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeetConfig {
	pub file_group: Vec<FileGroupConfig>,
}


impl BeetConfig {}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileGroupConfig {
	#[serde(flatten)]
	pub file_group: FileGroup,
	#[serde(flatten)]
	pub codegen: CodegenFile,
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
}
