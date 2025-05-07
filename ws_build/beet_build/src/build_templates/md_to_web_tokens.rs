use anyhow::Result;
use beet_common::prelude::*;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;
use sweet::prelude::ReadFile;
use sweet::prelude::WorkspacePathBuf;

use crate::utils::ParseMarkdown;



/// For a given markdown file, parse into a
/// [`NodeSpan`] and [`WebTokens`] pairs.
pub struct MdToWebTokens;


impl Pipeline<WorkspacePathBuf, Result<(NodeSpan, WebTokens)>>
	for MdToWebTokens
{
	fn apply(self, path: WorkspacePathBuf) -> Result<(NodeSpan, WebTokens)> {
		let location = NodeSpan::new_for_file(&path);
		let file = ReadFile::to_string(path.into_abs_unchecked())?;
		let web_tokens = ParseMarkdown::markdown_to_rsx_str(&file)
			.xpipe(StringToWebTokens::default())
			.map_err(|e| {
				anyhow::anyhow!(
					"Failed to parse Markdown HTML\nPath: {}\nError: {}",
					location.file.display(),
					e
				)
			})?;
		Ok((location, web_tokens))
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
