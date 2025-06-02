use anyhow::Result;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;
use sweet::prelude::ReadFile;
use sweet::prelude::WorkspacePathBuf;

use crate::utils::ParseMarkdown;



/// For a given markdown file, parse into a
/// [`FileSpan`] and [`WebTokens`] pairs.
pub struct MdToWebTokens;


impl Pipeline<WorkspacePathBuf, Result<WebTokens>> for MdToWebTokens {
	fn apply(self, path: WorkspacePathBuf) -> Result<WebTokens> {
		let file = ReadFile::to_string(path.into_abs())?;
		let web_tokens = ParseMarkdown::markdown_to_rsx_str(&file)
			.xpipe(StringToWebTokens::new(path.clone()))
			.map_err(|e| {
				anyhow::anyhow!(
					"Failed to parse Markdown HTML\nPath: {}\nError: {}",
					path.display(),
					e
				)
			})?;
		Ok(web_tokens)
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		WorkspacePathBuf::new("README.md")
			.xpipe(MdToWebTokens)
			.unwrap();
	}
}
