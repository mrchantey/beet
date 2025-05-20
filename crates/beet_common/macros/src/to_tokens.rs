use proc_macro2::TokenStream;
use quote::quote;
use sweet::prelude::*;
use syn;
use syn::DeriveInput;

use syn::parse_macro_input;

pub fn impl_derive_to_tokens(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}



fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	let name = &input.ident;
	let content = match &input.data {
		syn::Data::Struct(data_struct) => {
			let fields = &data_struct.fields;

			match fields {
				syn::Fields::Named(fields_named) => {
					let field_tokens = fields_named.named.iter().map(|field| {
						let field_name = &field.ident;
						quote! {
							tokens.extend(quote::quote! { #field_name: });
							self.#field_name.into_custom_tokens(tokens);
							tokens.extend(quote::quote! { , });
						}
					});

					quote! {
						tokens.extend(quote::quote! { #name });
						tokens.extend(proc_macro2::TokenStream::from_str("{").unwrap());
						#(#field_tokens)*
						tokens.extend(proc_macro2::TokenStream::from_str("}").unwrap());
					}
				}
				syn::Fields::Unnamed(fields_unnamed) => {
					let field_tokens =
						fields_unnamed.unnamed.iter().enumerate().map(
							|(i, _)| {
								let index = syn::Index::from(i);
								quote! {
									self.#index.into_custom_tokens(tokens);
									tokens.extend(quote::quote! { , });
								}
							},
						);

					quote! {
						tokens.extend(quote::quote! { #name });
						tokens.extend(proc_macro2::TokenStream::from_str("(").unwrap());
						#(#field_tokens)*
						tokens.extend(proc_macro2::TokenStream::from_str(")").unwrap());
					}
				}
				syn::Fields::Unit => {
					quote! {
						tokens.extend(quote::quote! { #name });
					}
				}
			}
		}
		syn::Data::Enum(data_enum) => {
			let match_arms = data_enum.variants.iter().map(|variant| {
				let variant_name = &variant.ident;

				match &variant.fields {
					syn::Fields::Named(fields_named) => {
						let field_names = fields_named
							.named
							.iter()
							.map(|field| field.ident.as_ref().unwrap())
							.collect::<Vec<_>>();

						quote! {
							#name::#variant_name { #(#field_names),* } => {
								tokens.extend(quote::quote! { #name::#variant_name });
								tokens.extend(proc_macro2::TokenStream::from_str("{").unwrap());
								#(
									tokens.extend(quote::quote! { #field_names: });
									#field_names.into_custom_tokens(tokens);
									tokens.extend(quote::quote! { , });
								)*
								tokens.extend(proc_macro2::TokenStream::from_str("}").unwrap());
							}
						}
					}
					syn::Fields::Unnamed(fields_unnamed) => {
						let field_vars = (0..fields_unnamed.unnamed.len())
							.map(|i| {
								syn::Ident::new(
									&format!("field{}", i),
									proc_macro2::Span::call_site(),
								)
							})
							.collect::<Vec<_>>();

						quote! {
							#name::#variant_name(#(#field_vars),*) => {
								tokens.extend(quote::quote! { #name::#variant_name });
								tokens.extend(proc_macro2::TokenStream::from_str("(").unwrap());
								#(
									#field_vars.into_custom_tokens(tokens);
									tokens.extend(quote::quote! { , });
								)*
								tokens.extend(proc_macro2::TokenStream::from_str(")").unwrap());
							}
						}
					}
					syn::Fields::Unit => {
						quote! {
							#name::#variant_name => {
								tokens.extend(quote::quote! { #name::#variant_name });
							}
						}
					}
				}
			});

			quote! {
				match self {
					#(#match_arms),*
				}
			}
		}
		syn::Data::Union(data_union) => {
			return Err(syn::Error::new_spanned(
				&data_union.union_token,
				"Union types are not supported by IntoCustomTokens derive",
			));
		}
	};

	quote! {
		impl beet::prelude::IntoCustomTokens for #name {
			fn into_custom_tokens(&self, tokens: &mut beet::exports::proc_macro2::TokenStream) {
				use std::str::FromStr;
				use beet::exports::quote;
				use beet::exports::proc_macro2;
				#content
			}
		}
	}
	.xok()
}


#[cfg(test)]
mod test {
	// use crate::prelude::*;
	use super::parse;
	use quote::quote;
	use sweet::prelude::*;
	use syn::DeriveInput;

	#[test]
	fn test_struct_named_fields() {
		let input: DeriveInput = syn::parse_quote! {
			struct Test {
				inner: u32,
				value: String,
			}
		};
		input.xmap(parse).unwrap().to_string().xpect().to_be(
			quote! {
					impl beet::prelude::IntoCustomTokens for Test {
						fn into_custom_tokens(&self, tokens: &mut beet::exports::proc_macro2::TokenStream) {
							use std::str::FromStr;
							use beet::exports::quote;
							use beet::exports::proc_macro2;
							tokens.extend(quote::quote! { Test });
							tokens.extend(proc_macro2::TokenStream::from_str("{").unwrap());
							tokens.extend(quote::quote! { inner: });
							self.inner.into_custom_tokens(tokens);
							tokens.extend(quote::quote! { , });
							tokens.extend(quote::quote! { value: });
							self.value.into_custom_tokens(tokens);
							tokens.extend(quote::quote! { , });
							tokens.extend(proc_macro2::TokenStream::from_str("}").unwrap());
						}
					}
				}
			.to_string(),
		);
	}

	#[test]
	fn test_struct_tuple() {
		let input: DeriveInput = syn::parse_quote! {
			struct TupleTest(u32, String);
		};
		input.xmap(parse).unwrap().to_string().xpect().to_be(
			quote! {
					impl beet::prelude::IntoCustomTokens for TupleTest {
						fn into_custom_tokens(&self, tokens: &mut beet::exports::proc_macro2::TokenStream) {
							use std::str::FromStr;
							use beet::exports::quote;
							use beet::exports::proc_macro2;
							tokens.extend(quote::quote! { TupleTest });
							tokens.extend(proc_macro2::TokenStream::from_str("(").unwrap());
							self.0.into_custom_tokens(tokens);
							tokens.extend(quote::quote! { , });
							self.1.into_custom_tokens(tokens);
							tokens.extend(quote::quote! { , });
							tokens.extend(proc_macro2::TokenStream::from_str(")").unwrap());
						}
					}
				}
			.to_string(),
		);
	}

	#[test]
	fn test_enum() {
		let input: DeriveInput = syn::parse_quote! {
			enum TestEnum {
				A,
				B(u32),
				C { value: String },
			}
		};
		input.xmap(parse).unwrap().to_string().xpect().to_be(
			quote! {
					impl beet::prelude::IntoCustomTokens for TestEnum {
						fn into_custom_tokens(&self, tokens: &mut beet::exports::proc_macro2::TokenStream) {
							use std::str::FromStr;
							use beet::exports::quote;
							use beet::exports::proc_macro2;
							match self {
								TestEnum::A => {
									tokens.extend(quote::quote! { TestEnum::A });
								},
								TestEnum::B(field0) => {
									tokens.extend(quote::quote! { TestEnum::B });
									tokens.extend(proc_macro2::TokenStream::from_str("(").unwrap());
									field0.into_custom_tokens(tokens);
									tokens.extend(quote::quote! { , });
									tokens.extend(proc_macro2::TokenStream::from_str(")").unwrap());
								},
								TestEnum::C { value } => {
									tokens.extend(quote::quote! { TestEnum::C });
									tokens.extend(proc_macro2::TokenStream::from_str("{").unwrap());
									tokens.extend(quote::quote! { value: });
									value.into_custom_tokens(tokens);
									tokens.extend(quote::quote! { , });
									tokens.extend(proc_macro2::TokenStream::from_str("}").unwrap());
								}
							}
						}
					}
				}
			.to_string(),
		);
	}
}
