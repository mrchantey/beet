use anyhow::Result;
use beet_common::prelude::*;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;
use sweet::prelude::ReadFile;
use sweet::prelude::WorkspacePathBuf;

use crate::utils::ParseMarkdown;



/// For a given markdown file, parse into a
/// [`FileSpan`] and [`WebTokens`] pairs.
pub struct MdToWebTokens;


impl Pipeline<WorkspacePathBuf, Result<(FileSpan, WebTokens)>>
	for MdToWebTokens
{
	fn apply(self, path: WorkspacePathBuf) -> Result<(FileSpan, WebTokens)> {
		let span = FileSpan::new_for_file(&path);
		let file = ReadFile::to_string(path.into_abs_unchecked())?;
		let web_tokens = ParseMarkdown::markdown_to_rsx_str(&file)
			.xpipe(StringToWebTokens::new(Some(span.clone())))
			.map_err(|e| {
				anyhow::anyhow!(
					"Failed to parse Markdown HTML\nPath: {}\nError: {}",
					span.file().display(),
					e
				)
			})?;
		Ok((span, web_tokens))
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
