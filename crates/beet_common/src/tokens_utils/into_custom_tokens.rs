use std::marker::PhantomData;
use std::ops::Deref;

use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use quote::quote_spanned;
use send_wrapper::SendWrapper;
use sweet::prelude::WorkspacePathBuf;
use syn::Lit;

/// Trait for converting a type into a [`TokenStream`],
/// usually derived using the [`ToTokens`] macro.
pub trait IntoCustomTokens {
	/// Append the type to a [`TokenStream`].
	fn into_custom_tokens(&self, tokens: &mut TokenStream);
	/// Create a new [`TokenStream`] from the type.
	fn into_custom_token_stream(&self) -> TokenStream {
		let mut tokens = TokenStream::new();
		self.into_custom_tokens(&mut tokens);
		tokens
	}
}

impl<T> IntoCustomTokens for SendWrapper<T>
where
	T: IntoCustomTokens,
{
	fn into_custom_tokens(&self, tokens: &mut TokenStream) {
		self.deref().into_custom_tokens(tokens);
	}
}

impl IntoCustomTokens for () {
	fn into_custom_tokens(&self, tokens: &mut TokenStream) {
		tokens.extend(quote! { () });
	}
}

impl IntoCustomTokens for TokenStream {
	fn into_custom_tokens(&self, tokens: &mut TokenStream) {
		self.to_tokens(tokens);
	}
}
impl IntoCustomTokens for syn::Expr {
	fn into_custom_tokens(&self, tokens: &mut TokenStream) {
		self.to_tokens(tokens);
	}
}

impl IntoCustomTokens for WorkspacePathBuf {
	fn into_custom_tokens(&self, tokens: &mut TokenStream) {
		let path = self.to_string_lossy();
		tokens.extend(quote! { WorkspacePathBuf::new(#path) });
	}
}

impl<T> IntoCustomTokens for PhantomData<T> {
	fn into_custom_tokens(&self, tokens: &mut TokenStream) {
		let type_name =
			syn::parse_str::<syn::Path>(std::any::type_name::<T>()).unwrap();
		tokens.extend(quote! { std::marker::PhantomData::<#type_name> });
	}
}

impl IntoCustomTokens for String {
	fn into_custom_tokens(&self, tokens: &mut TokenStream) {
		tokens.extend(quote! { String::from(#self) });
	}
}

impl<T: IntoCustomTokens> IntoCustomTokens for Option<T> {
	fn into_custom_tokens(&self, tokens: &mut TokenStream) {
		match self {
			Some(value) => {
				let value = value.into_custom_token_stream();
				tokens.extend(quote! { Some(#value) });
			}
			None => {
				tokens.extend(quote! { None });
			}
		}
	}
}

macro_rules! impl_into_custom_tokens {
	($($t:ty),*) => {
		$(
			impl IntoCustomTokens for $t {
				fn into_custom_tokens(&self, tokens: &mut TokenStream) {
					tokens.extend(quote! { #self });
				}
			}
		)*
	};
}

// Implement for all primitive types
impl_into_custom_tokens!(
	i8,
	i16,
	i32,
	i64,
	i128,
	isize,
	u8,
	u16,
	u32,
	u64,
	u128,
	usize,
	f32,
	f64,
	bool,
	char,
	&'static str
);


/// A trait for types that can be tokenized into
/// a Rust representation.
pub trait RustTokens {
	/// Convert the type into a rust [`TokenStream`] for use in
	/// rust files like macros, ie `MyStruct::new(7)`.
	/// It assumes the type is in scope.
	fn into_rust_tokens(&self) -> TokenStream;
}

impl RustTokens for String {
	/// Create a token stream of the constructor for a [`String`]
	fn into_rust_tokens(&self) -> TokenStream {
		let value = self.to_string();
		quote! { #value.to_string() }
	}
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
