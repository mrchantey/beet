use proc_macro2::Literal;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;

/// Hash a span based on the start location
pub fn span_to_line_col(span: &Span) -> TokenStream {
	let line = span.start().line as u32;
	let column = span.start().column as u32;
	quote! {LineColumn::new(#line, #column)}
}
/// ron version of [span_to_line_col]
pub fn span_to_line_col_ron(span: &Span) -> TokenStream {
	let line = Literal::u32_unsuffixed(span.start().line as u32);
	let column = Literal::u32_unsuffixed(span.start().column as u32);
	quote! {
		LineColumn(
			line: #line,
			column: #column
		)
	}
}


// pub fn effect_tokens()
