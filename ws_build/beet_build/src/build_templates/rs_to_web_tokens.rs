use anyhow::Result;
use beet_common::prelude::*;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;
use sweet::prelude::ReadFile;
use sweet::prelude::WorkspacePathBuf;
use syn::spanned::Spanned;
use syn::visit::Visit;


/// For a given rust file, extract all `rsx!` macros and return a vector of
/// [`NodeSpan`] and [`WebTokens`] pairs.
pub struct RsToWebTokens;


impl Pipeline<WorkspacePathBuf, Result<Vec<(NodeSpan, WebTokens)>>>
	for RsToWebTokens
{
	fn apply(
		self,
		path: WorkspacePathBuf,
	) -> Result<Vec<(NodeSpan, WebTokens)>> {
		let file = ReadFile::to_string(path.into_abs_unchecked())?;
		let file = syn::parse_file(&file)?;
		let mac = syn::parse_quote!(rsx);
		// let path = WorkspacePathBuf::new_from_canonicalizable(path)?;
		let mut visitor = RsxSynVisitor::new(path, mac);

		visitor.visit_file(&file);
		Ok(visitor.templates)
	}
}




/// Visit a file, extracting an [`NodeSpan`] and [`RsxTemplateNode`] for each
/// `rsx!` macro in the file.
#[derive(Debug)]
struct RsxSynVisitor {
	/// Used for creating [`NodeSpan`] in several places.
	/// We must use workspace relative paths because locations are created
	/// via the `file!()` macro.
	file: WorkspacePathBuf,
	templates: Vec<(NodeSpan, WebTokens)>,
	mac: syn::Ident,
}
impl RsxSynVisitor {
	pub fn new(file: WorkspacePathBuf, mac: syn::Ident) -> Self {
		Self {
			file,
			templates: Default::default(),
			mac,
		}
	}
}

impl<'a> Visit<'a> for RsxSynVisitor {
	fn visit_macro(&mut self, mac: &syn::Macro) {
		if mac
			.path
			.segments
			.last()
			.map_or(false, |seg| seg.ident == self.mac)
		{
			// use the span of the inner tokens to match the behavior of
			// the rsx! macro
			let span = mac.tokens.span();
			let start = span.start();
			let loc = NodeSpan::new(
				self.file.clone(),
				start.line as u32,
				start.column as u32,
			);
			let web_tokens = mac
				.tokens
				.clone()
				.xpipe(TokensToRstml::default())
				.0
				.xpipe(RstmlToWebTokens::new())
				.0;

			self.templates.push((loc, web_tokens));
		}
	}
}
