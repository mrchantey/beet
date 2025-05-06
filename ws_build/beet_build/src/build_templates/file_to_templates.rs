use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;
use sweet::prelude::WorkspacePathBuf;
use sweet::prelude::*;




pub struct FileToTemplates;



impl Pipeline<WorkspacePathBuf, Result<Vec<FileTemplates>>>
	for FileToTemplates
{
	fn apply(self, path: WorkspacePathBuf) -> Result<Vec<FileTemplates>> {
		match path.extension() {
			Some(ex) if ex == "rs" => path.xpipe(RsToHtmlTokens),
			Some(ex) if ex == "md" || ex == "mdx" => {
				path.xpipe(MdToHtmlTokens).map(|v| vec![v])
			}
			_ => Ok(Default::default()),
		}?
		.xmap_each(|(location, html_tokens)| {
			let rsx_ron = html_tokens
				.xpipe(ParseHtmlTokens::default())?
				.xpipe(HtmlTokensToRon::new(
					&location.file,
					location.line,
					location.col,
				))
				.to_string();
			let template_node =
				ron::de::from_str::<RsxTemplateNode>(rsx_ron.trim())
					.map_err(|e| ron_cx_err(e, &rsx_ron))?;

			FileTemplates {
				location,
				template_node,
				// style_ron: Default::default(),
			}
			.xok()
		})
		.into_iter()
		.collect::<Result<Vec<_>>>()
	}
}


/// how many leading and trailing characters to show in the context of the error
const CX_SIZE: usize = 8;
/// A ron deserialization error with the context of the file and line
fn ron_cx_err(e: ron::error::SpannedError, str: &str) -> anyhow::Error {
	let start = e.position.col.saturating_sub(CX_SIZE);
	let end = e.position.col.saturating_add(CX_SIZE);
	let cx = if e.position.line == 1 {
		str[start..end].to_string()
	} else {
		let lines = str.lines().collect::<Vec<_>>();
		let str = lines.get(e.position.line - 1).unwrap_or(&"");
		str[start..end].to_string()
	};
	return anyhow::anyhow!(
		"Failed to parse RsxTemplate from string\nError: {}\nContext: {}\nFull: {}",
		e.code,
		cx,
		str
	);
}



pub struct FileTemplates {
	/// The location of the rsx
	pub location: RsxMacroLocation,
	/// A [`TokenStream`] representing a [`ron`] representation of a [`RsxTemplateNode`].
	pub template_node: RsxTemplateNode,
	// /// A [`TokenStream`] representing styles extracted from the file.
	// pub style_ron: TokenStream,
}
