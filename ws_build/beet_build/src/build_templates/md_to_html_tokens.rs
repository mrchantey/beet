use anyhow::Result;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;
use sweet::prelude::ReadFile;
use sweet::prelude::WorkspacePathBuf;

use crate::utils::ParseMarkdown;



/// For a given markdown file, parse into a
/// [`RsxMacroLocation`] and [`HtmlTokens`] pairs.
pub struct MdToHtmlTokens;


impl Pipeline<WorkspacePathBuf, Result<(RsxMacroLocation, HtmlTokens)>>
	for MdToHtmlTokens
{
	fn apply(
		self,
		path: WorkspacePathBuf,
	) -> Result<(RsxMacroLocation, HtmlTokens)> {
		let location = RsxMacroLocation::new_for_file(&path);
		let file = ReadFile::to_string(path.into_abs_unchecked())?;
		let html_tokens = ParseMarkdown::markdown_to_rsx_str(&file)
			.xtap(|val| println!("Parsed Markdown: {}", val))
			.xpipe(StringToHtmlTokens::default())
			.map_err(|e| {
				anyhow::anyhow!(
					"Failed to parse Markdown HTML\nPath: {}\nError: {}",
					location.file.display(),
					e
				)
			})?;
		Ok((location, html_tokens))
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		WorkspacePathBuf::new("README.md")
			.xpipe(MdToHtmlTokens)
			.unwrap();
	}
}
