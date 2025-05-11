use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::Lit;




/// A trait for types that can be tokenized into
/// a Rust representation.
pub trait RustTokens {
	/// Convert the type into a rust [`TokenStream`] for use in
	/// rust files like macros, ie `MyStruct::new(7)`.
	/// It assumes the type is in scope.
	fn into_rust_tokens(&self) -> TokenStream;
}


impl RustTokens for syn::Lit {
	/// Create a token stream of the constructor for a [`syn::Lit`]
	fn into_rust_tokens(&self) -> TokenStream {
		match self {
			Lit::Str(str) => {
				let span = str.span();
				let value = str.value();
				quote_spanned! {span=> syn::Lit::Str(
					syn::LitStr::new(#value, proc_macro2::Span::call_site())
				)}
			}
			Lit::CStr(lit_cstr) => {
				let span = lit_cstr.span();
				let value = lit_cstr.value();
				quote_spanned! {span=> syn::Lit::CStr(
					syn::LitCStr::new(#value, proc_macro2::Span::call_site())
				)}
			}
			Lit::Float(lit_float) => {
				let span = lit_float.span();
				let value = lit_float.base10_digits();
				quote_spanned! {span=> syn::Lit::Float(
					syn::LitFloat::new(#value, proc_macro2::Span::call_site())
				)}
			}
			Lit::Bool(lit_bool) => {
				let span = lit_bool.span();
				let value = lit_bool.value;
				quote_spanned! {span=> syn::Lit::Bool(
					syn::LitBool::new(#value, proc_macro2::Span::call_site())
				)}
			}
			Lit::Byte(lit_byte) => {
				let span = lit_byte.span();
				let value = lit_byte.value();
				quote_spanned! {span=> syn::Lit::Byte(
					syn::LitByte::new(#value, proc_macro2::Span::call_site())
				)}
			}
			Lit::Char(lit_char) => {
				let span = lit_char.span();
				let value = lit_char.value();
				quote_spanned! {span=> syn::Lit::Char(
					syn::LitChar::new(#value, proc_macro2::Span::call_site())
				)}
			}
			Lit::Int(lit_int) => {
				let span = lit_int.span();
				let value = lit_int.base10_digits();
				quote_spanned! {span=> syn::Lit::Int(
					syn::LitInt::new(#value, proc_macro2::Span::call_site())
				)}
			}
			Lit::ByteStr(lit_byte_str) => {
				let span = lit_byte_str.span();
				let value = lit_byte_str.value();
				quote_spanned! {span=> syn::Lit::ByteStr(
					syn::LitByteStr::new(vec![#(#value),*], proc_macro2::Span::call_site())
				)}
			}
			Lit::Verbatim(literal) => {
				let span = literal.span();
				quote_spanned! {span=> syn::Lit::Verbatim(#literal)}
			}
			_ => unimplemented!(),
		}
	}
}
