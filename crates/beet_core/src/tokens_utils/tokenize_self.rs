use beet_utils::prelude::*;
use bevy::ecs::entity::Entity;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use send_wrapper::SendWrapper;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::PathBuf;

/// Trait for converting a type into a [`TokenStream`],
/// usually derived using the [`ToTokens`] macro.
pub trait TokenizeSelf<M = Self> {
	/// Append the type to a [`TokenStream`].
	fn self_tokens(&self, tokens: &mut TokenStream);
	/// Create a new [`TokenStream`] from the type.
	fn self_token_stream(&self) -> TokenStream {
		let mut tokens = TokenStream::new();
		self.self_tokens(&mut tokens);
		tokens
	}
}



/// Returns the past part of an [`std::any::type_name`] as a [`syn::Path`],
/// the user is expected to bring the type into scope.
/// Where the typename is `"std::option::Option<std::vec::Vec<usize>>"`,
/// the output is `Option<Vec<usize>>`
pub fn short_type_path<T>() -> syn::Path {
	let type_name = std::any::type_name::<T>();
	let result = shorten_generic_type_name(type_name);
	syn::parse_str::<syn::Path>(&result).expect(&format!(
		"Failed to parse type name {result} into syn::Path"
	))
}

fn shorten_generic_type_name(type_name: &str) -> String {
	let mut result = String::new();
	let mut chars = type_name.chars().peekable();

	while let Some(ch) = chars.next() {
		match ch {
			'<' => {
				// We hit a generic start, collect everything until the matching '>'
				result.push(ch);
				let mut depth = 1;
				let mut inner_content = String::new();

				while let Some(inner_ch) = chars.next() {
					match inner_ch {
						'<' => {
							depth += 1;
							inner_content.push(inner_ch);
						}
						'>' => {
							depth -= 1;
							if depth == 0 {
								// We found the matching closing bracket
								// Process the inner content and add it
								if !inner_content.is_empty() {
									let shortened_inner =
										shorten_generic_type_name(
											&inner_content,
										);
									result.push_str(&shortened_inner);
								}
								result.push(inner_ch);
								break;
							} else {
								inner_content.push(inner_ch);
							}
						}
						',' if depth == 1 => {
							// Comma at the top level of this generic
							if !inner_content.is_empty() {
								let shortened_inner = shorten_generic_type_name(
									&inner_content.trim(),
								);
								result.push_str(&shortened_inner);
								inner_content.clear();
							}
							result.push_str(", ");
						}
						_ => {
							inner_content.push(inner_ch);
						}
					}
				}
			}
			':' => {
				// Check if this is part of ::
				if chars.peek() == Some(&':') {
					chars.next(); // consume the second :
					// Skip everything up to this point and start fresh
					result.clear();
				} else {
					result.push(ch);
				}
			}
			_ => {
				result.push(ch);
			}
		}
	}

	result
}


impl<T> TokenizeSelf for SendWrapper<T>
where
	T: TokenizeSelf,
{
	fn self_tokens(&self, tokens: &mut TokenStream) {
		let inner = self.deref().self_token_stream();
		tokens.extend(quote! { SendWrapper::new(#inner) });
	}
}

impl TokenizeSelf for () {
	fn self_tokens(&self, tokens: &mut TokenStream) {
		tokens.extend(quote! { () });
	}
}
impl TokenizeSelf for Entity {
	fn self_tokens(&self, tokens: &mut TokenStream) {
		let bits = self.to_bits();
		tokens.extend(quote! { Entity::from_bits(#bits) });
	}
}

impl TokenizeSelf for TokenStream {
	fn self_tokens(&self, tokens: &mut TokenStream) { self.to_tokens(tokens); }
}
impl TokenizeSelf for syn::Expr {
	fn self_tokens(&self, tokens: &mut TokenStream) {
		let inner = self.to_token_stream();
		tokens.extend(quote! { syn::parse_quote!(#inner) });
	}
}

impl TokenizeSelf for WsPathBuf {
	fn self_tokens(&self, tokens: &mut TokenStream) {
		let path = self.to_string_lossy();
		tokens.extend(quote! { WsPathBuf::new(#path) });
	}
}
impl TokenizeSelf for PathBuf {
	fn self_tokens(&self, tokens: &mut TokenStream) {
		let path = self.to_string_lossy();
		tokens.extend(quote! { std::path::PathBuf::from(#path) });
	}
}

impl<T> TokenizeSelf for PhantomData<T> {
	fn self_tokens(&self, tokens: &mut TokenStream) {
		let type_name = short_type_path::<T>();
		tokens.extend(quote! { std::marker::PhantomData::<#type_name> });
	}
}

impl<T> TokenizeSelf for Vec<T>
where
	T: TokenizeSelf,
{
	fn self_tokens(&self, tokens: &mut TokenStream) {
		let items = self.iter().map(|item| item.self_token_stream());
		tokens.extend(quote! { vec![#(#items),*] });
	}
}
pub struct TokenizeSelfRefMarker;

impl<T> TokenizeSelf<TokenizeSelfRefMarker> for Vec<&T>
where
	T: TokenizeSelf,
{
	fn self_tokens(&self, tokens: &mut TokenStream) {
		let items = self.iter().map(|item| item.self_token_stream());
		tokens.extend(quote! { vec![#(#items),*] });
	}
}

impl TokenizeSelf for String {
	fn self_tokens(&self, tokens: &mut TokenStream) {
		tokens.extend(quote! { String::from(#self) });
	}
}
impl TokenizeSelf for Span {
	fn self_tokens(&self, tokens: &mut TokenStream) {
		tokens.extend(quote! { proc_macro2::Span::call_site() });
	}
}

impl<T: TokenizeSelf> TokenizeSelf for Option<T> {
	fn self_tokens(&self, tokens: &mut TokenStream) {
		match self {
			Some(value) => {
				let value = value.self_token_stream();
				tokens.extend(quote! { Some(#value) });
			}
			None => {
				tokens.extend(quote! { None });
			}
		}
	}
}

macro_rules! impl_self_tokens {
	($($t:ty),*) => {
		$(
			impl TokenizeSelf for $t {
				fn self_tokens(&self, tokens: &mut TokenStream) {
					tokens.extend(quote! { #self });
				}
			}
		)*
	};
}

// Implement for all primitive types
impl_self_tokens!(
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


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use quote::ToTokens;
	use sweet::prelude::*;

	#[test]
	fn works() {
		short_type_path::<Option<Vec<Matcher<u32>>>>()
			.to_token_stream()
			.to_string()
			.replace(" ", "")
			.xpect()
			.to_be("Option<Vec<Matcher<u32>>>");
	}
}
