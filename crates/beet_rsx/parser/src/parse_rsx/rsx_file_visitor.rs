use crate::prelude::*;
use syn::visit_mut;
use syn::visit_mut::VisitMut;


/// The rsx visitor is used by file (preprocessor) parsers.
pub struct RsxFileVisitor<'a> {
	parser: &'a mut ParseRsx,
	/// The rsx macros found in the function
	macros: Vec<RstmlToRsx>,
	/// Errors that occurred while parsing the rsx macro
	errors: Vec<syn::Error>,
}

/// Output from a fully parsed file with multiple rsx macros.
pub struct RsxFileVisitorOut {
	/// the transformed rust macros
	pub macros: Vec<RstmlToRsx>,
	pub errors: Vec<syn::Error>,
}

impl<'a> Into<RsxFileVisitorOut> for RsxFileVisitor<'a> {
	fn into(self) -> RsxFileVisitorOut {
		RsxFileVisitorOut {
			macros: self.macros,
			errors: self.errors,
		}
	}
}

impl<'a> RsxFileVisitor<'a> {
	pub fn new(parser: &'a mut ParseRsx) -> Self {
		Self {
			parser,
			macros: Vec::new(),
			errors: Vec::new(),
		}
	}
	pub fn extend_result(
		&mut self,
		result: Result<RsxFileVisitorOut, syn::Error>,
	) {
		match result {
			Ok(out) => {
				self.macros.extend(out.macros);
				self.errors.extend(out.errors);
			}
			Err(e) => self.errors.push(e),
		}
	}
}

impl<'a> VisitMut for RsxFileVisitor<'a> {
	fn visit_macro_mut(&mut self, item: &mut syn::Macro) {
		if self.parser.path_matches(&item.path) {
			let parts = self.parser.parse_rsx(&mut item.tokens);
			self.macros.push(parts);
			// place path::to::rsx! with noop!
			item.path = syn::parse_quote!(sweet::noop)
		}
		// visit nested
		visit_mut::visit_macro_mut(self, item);
	}
}
