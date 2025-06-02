use anyhow::Result;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;
use sweet::prelude::ReadFile;
use sweet::prelude::WorkspacePathBuf;
use syn::visit::Visit;


/// For a given rust file, extract all `rsx!` macros and return a vector of
/// [`FileSpan`] and [`WebTokens`] pairs.
pub struct RsToWebTokens;


impl Pipeline<WorkspacePathBuf, Result<Vec<WebTokens>>> for RsToWebTokens {
	fn apply(self, path: WorkspacePathBuf) -> Result<Vec<WebTokens>> {
		let file = ReadFile::to_string(path.into_abs())?;
		let file = syn::parse_file(&file)?;
		let mac = syn::parse_quote!(rsx);
		// let path = WorkspacePathBuf::new_from_canonicalizable(path)?;
		let mut visitor = RsxSynVisitor::new(path, mac);

		visitor.visit_file(&file);
		Ok(visitor.templates)
	}
}




/// Visit a file, extracting an [`FileSpan`] and [`WebNodeTemplate`] for each
/// `rsx!` macro in the file.
#[derive(Debug)]
struct RsxSynVisitor {
	/// Used for creating [`FileSpan`] in several places.
	/// We must use workspace relative paths because locations are created
	/// via the `file!()` macro.
	file: WorkspacePathBuf,
	templates: Vec<WebTokens>,
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
			// let span = mac.tokens.span();
			// let loc = FileSpan::new_from_span(self.file.clone(), &span);

			let web_tokens = mac
				.tokens
				.clone()
				.xpipe(TokensToRstml::default())
				.0
				.xpipe(RstmlToWebTokens::new(self.file.clone()))
				.0;
			self.templates.push(web_tokens);
		}
	}
}
