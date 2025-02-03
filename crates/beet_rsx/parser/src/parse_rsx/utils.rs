use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;

/// Hash a span based on the start location
pub fn span_to_line_col(span: &Span) -> TokenStream {
	let line = span.start().line as u32;
	let column = span.start().column as u32;
	quote! {LineColumn::new(#line, #column)}
}


// pub fn effect_tokens()
