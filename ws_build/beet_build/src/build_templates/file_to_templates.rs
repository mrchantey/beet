use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;
use proc_macro2::TokenStream;
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
			let rsx_ron = html_tokens.xpipe(ParseHtmlTokens::default())?.xpipe(
				HtmlTokensToRon::new(
					&location.file,
					location.line,
					location.col,
				),
			);

			FileTemplates {
				location,
				rsx_ron,
				style_ron: Default::default(),
			}
			.xok()
		})
		.into_iter()
		.collect::<Result<Vec<_>>>()
	}
}




pub struct FileTemplates {
	/// The location of the rsx
	pub location: RsxMacroLocation,
	/// A [`TokenStream`] representing a [`ron`] representation of a [`RsxTemplateNode`].
	pub rsx_ron: TokenStream,
	/// A [`TokenStream`] representing styles extracted from the file.
	pub style_ron: TokenStream,
}
