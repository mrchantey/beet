use crate::prelude::*;
use syn::visit_mut;
use syn::visit_mut::VisitMut;


/// The rsx visitor is used by file (preprocessor) parsers.
pub struct RsxFileVisitor<'a, T> {
	parser: &'a mut RsxParser<T>,
	/// The rsx macros found in the function
	macros: Vec<RstmlToRsx<T>>,
	/// Errors that occurred while parsing the rsx macro
	errors: Vec<syn::Error>,
}

/// Output from a fully parsed file with multiple rsx macros.
pub struct RsxFileVisitorOut<T> {
	/// the transformed rust macros
	pub macros: Vec<RstmlToRsx<T>>,
	pub errors: Vec<syn::Error>,
}

impl<'a, T> Into<RsxFileVisitorOut<T>> for RsxFileVisitor<'a, T> {
	fn into(self) -> RsxFileVisitorOut<T> {
		RsxFileVisitorOut {
			macros: self.macros,
			errors: self.errors,
		}
	}
}

impl<'a, T> RsxFileVisitor<'a, T> {
	pub fn new(parser: &'a mut RsxParser<T>) -> Self {
		Self {
			parser,
			macros: Vec::new(),
			errors: Vec::new(),
		}
	}
	pub fn extend_result(
		&mut self,
		result: Result<RsxFileVisitorOut<T>, syn::Error>,
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

impl<'a, T: RsxRustTokens> VisitMut for RsxFileVisitor<'a, T> {
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
