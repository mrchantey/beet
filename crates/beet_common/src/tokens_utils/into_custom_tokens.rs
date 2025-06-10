use bevy::ecs::entity::Entity;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use send_wrapper::SendWrapper;
use std::marker::PhantomData;
use std::ops::Deref;
use sweet::prelude::WorkspacePathBuf;

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
		let inner = 
		self.deref().into_custom_token_stream();
		tokens.extend(quote! { SendWrapper::new(#inner) });
	}
}

impl IntoCustomTokens for () {
	fn into_custom_tokens(&self, tokens: &mut TokenStream) {
		tokens.extend(quote! { () });
	}
}
impl IntoCustomTokens for Entity {
	fn into_custom_tokens(&self, tokens: &mut TokenStream) {
		let bits = self.to_bits();
		tokens.extend(quote! { Entity::from_bits(#bits) });
	}
}

impl IntoCustomTokens for TokenStream {
	fn into_custom_tokens(&self, tokens: &mut TokenStream) {
		self.to_tokens(tokens);
	}
}
impl IntoCustomTokens for syn::Expr {
	fn into_custom_tokens(&self, tokens: &mut TokenStream) {
		let inner = self.to_token_stream();
		tokens.extend(quote! { syn::parse_quote!(#inner) });
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

impl<T> IntoCustomTokens for Vec<T>
where
	T: IntoCustomTokens,
{
	fn into_custom_tokens(&self, tokens: &mut TokenStream) {
		let items = self.iter().map(|item| item.into_custom_token_stream());
		tokens.extend(quote! { vec![#(#items),*] });
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