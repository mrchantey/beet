use super::GetSpan;

/// A location in a source file, the line is 1 indexed and the column is 0 indexed.
/// The Default implementation is `1:0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineCol {
	/// The 1 indexed line in the source file, reflecting the behavior of `line!()` and
	/// `proc_macro2::Span`
	line: u32,
	/// The 0 indexed column in the source file, reflecting the behavior of `column!()`
	/// and `proc_macro2::Span`. This is not the same as proc_macro::Span which
	/// is 1 indexed.
	col: u32,
}

impl LineCol {
	pub fn new(line: u32, col: u32) -> Self {
		// id like to assert this but it seems rust-analyzer uses 0 based line numbers?

		// assert_ne!(line, 0, "Line number must be greater than 0");
		Self { line, col }
	}
	pub fn line(&self) -> u32 { self.line }
	pub fn col(&self) -> u32 { self.col }

	/// Find the start of the first element and the end of the last element,
	/// or default.
	pub fn iter_to_spans(vec: &[impl GetSpan]) -> (LineCol, LineCol) {
		let start = vec.first().map(|n| n.span().start()).unwrap_or_default();
		let end = vec.last().map(|n| n.span().end()).unwrap_or_default();
		(start, end)
	}

	// #[cfg(feature = "tokens")]
	// pub fn syn_iter_to_spans(
	// 	vec: &[impl syn::spanned::Spanned],
	// ) -> (LineCol, LineCol) {
	// 	let start = vec
	// 		.first()
	// 		.map(|n| n.span().start().into())
	// 		.unwrap_or_default();
	// 	let end = vec
	// 		.last()
	// 		.map(|n| n.span().end().into())
	// 		.unwrap_or_default();
	// 	(start, end)
	// }
}

impl Default for LineCol {
	fn default() -> Self { Self { line: 1, col: 0 } }
}

impl std::fmt::Display for LineCol {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.line, self.col)
	}
}


#[cfg(feature = "tokens")]
impl From<proc_macro2::LineColumn> for LineCol {
	fn from(line_col: proc_macro2::LineColumn) -> Self {
		Self {
			line: line_col.line as u32,
			col: line_col.column as u32,
		}
	}
}

#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for LineCol {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		let line = proc_macro2::Literal::u32_unsuffixed(self.line);
		let col = proc_macro2::Literal::u32_unsuffixed(self.col);
		quote::quote! {
			LineCol::new(#line, #col)
		}
	}
}
