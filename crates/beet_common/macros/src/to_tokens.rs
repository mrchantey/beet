use proc_macro2::TokenStream;
use quote::quote;
use beet_utils::prelude::*;
use crate::utils::pound_token;
use syn;
use syn::DeriveInput;
use syn::parse_macro_input;
use syn::WherePredicate;

pub fn impl_derive_to_tokens(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}



fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	let ident = &input.ident;
	let pound_token = pound_token();

	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();

	let generic_idents = input
		.generics
		.params
		.iter()
		.filter_map(|param| match param {
			syn::GenericParam::Type(ty) => Some(ty),
			syn::GenericParam::Lifetime(_) => None,
			syn::GenericParam::Const(_) => None,
		})
		.enumerate()
		.map(|(i, ty)| {
			let ident = &ty.ident;
			let generic_ident =
				syn::Ident::new(&format!("generic{}", i), ty.ident.span());
			(ident, generic_ident)
		});

	let qualified_name = if input.generics.params.is_empty() {
		quote! { #ident }
	} else {
		let qualified = generic_idents.clone().map(|(_, generic_ident)| {
			quote! { #pound_token #generic_ident }
		});
		quote! { #ident::<#(#qualified),*> }
	};

	let content = match &input.data {
		syn::Data::Struct(data_struct) => {
			let fields = &data_struct.fields;

			match fields {
				syn::Fields::Named(fields_named) => {
					let field_defs = fields_named.named.iter().map(|field| {
						let field_name = &field.ident;
						quote! {
							let #field_name = self.#field_name.self_token_stream();
						}
					});

					let field_tokens = fields_named.named.iter().map(|field| {
						let field_name = &field.ident;
						quote! {
							#field_name: #pound_token #field_name
						}
					});

					quote! {
						#(#field_defs)*
						tokens.extend(quote::quote! { #qualified_name {
							#(#field_tokens),*
						} });
					}
				}
				syn::Fields::Unnamed(fields_unnamed) => {
					let field_defs =
						fields_unnamed.unnamed.iter().enumerate().map(
							|(i, _)| {
								let index = syn::Index::from(i);
								let field_var = syn::Ident::new(
									&format!("field{}", i),
									proc_macro2::Span::call_site(),
								);
								quote! {
									let #field_var = self.#index.self_token_stream();
								}
							},
						);

					let field_vars = (0..fields_unnamed.unnamed.len())
						.map(|i| {
							syn::Ident::new(
								&format!("field{}", i),
								proc_macro2::Span::call_site(),
							)
						})
						.collect::<Vec<_>>();

					quote! {
						#(#field_defs)*
						tokens.extend(quote::quote! { #qualified_name(
							#(#pound_token #field_vars),*
						) });
					}
				}
				syn::Fields::Unit => {
					quote! {
						tokens.extend(quote::quote! { #qualified_name });
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
							Self::#variant_name { #(#field_names),* } => {
								#(let #field_names = #field_names.self_token_stream();)*
								tokens.extend(quote::quote! { #qualified_name::#variant_name {
									#(#field_names: #pound_token #field_names),*
								} });
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
							Self::#variant_name(#(#field_vars),*) => {
								#(let #field_vars = #field_vars.self_token_stream();)*
								tokens.extend(quote::quote! { #qualified_name::#variant_name(
									#(#pound_token #field_vars),*
								) });
							}
						}
					}
					syn::Fields::Unit => {
						quote! {
							Self::#variant_name => {
								tokens.extend(quote::quote! { #qualified_name::#variant_name });
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
				"Union types are not supported by TokenizeSelf derive",
			));
		}
	};
	let generic_defs = generic_idents.clone().map(|(ident, generic_ident)| {
		quote! {
			let #generic_ident = syn::parse_str::<syn::Path>(
				std::any::type_name::<#ident>()
			).expect("failed to parse generic type from std::any::type_name");
		}
	});

	let mut where_clause = where_clause
		.cloned()
		.unwrap_or_else(|| syn::parse_quote!(where));
	where_clause.predicates.extend(generic_idents.map(|(ident, _)| {
		let predicate: WherePredicate = syn::parse_quote! {
			#ident: beet::prelude::TokenizeSelf
		};
		predicate
	}));


	quote! {
		impl #impl_generics beet::prelude::TokenizeSelf for #ident #type_generics #where_clause {
			fn self_tokens(&self, tokens: &mut beet::exports::proc_macro2::TokenStream) {
				use beet::exports::quote;
				use beet::exports::proc_macro2;
				#(#generic_defs)*
				#content
			}
		}
	}
	.xok()
}


#[cfg(test)]
mod test {
	use beet_utils::prelude::*;
	use super::parse;
	use super::pound_token;
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
		let pound_token = pound_token();


		input.xmap(parse).unwrap().to_string().xpect().to_be(
			quote! {
					impl beet::prelude::TokenizeSelf for Test {
						fn self_tokens(&self, tokens: &mut beet::exports::proc_macro2::TokenStream) {
							use beet::exports::quote;
							use beet::exports::proc_macro2;
							let inner = self.inner.self_token_stream();
							let value = self.value.self_token_stream();
							tokens.extend(quote::quote! { Test {
								inner: #pound_token inner,
								value: #pound_token value
							} });
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
		let pound_token = pound_token();

		input.xmap(parse).unwrap().to_string().xpect().to_be(
			quote! {
					impl beet::prelude::TokenizeSelf for TupleTest {
						fn self_tokens(&self, tokens: &mut beet::exports::proc_macro2::TokenStream) {
							use beet::exports::quote;
							use beet::exports::proc_macro2;
							let field0 = self.0.self_token_stream();
							let field1 = self.1.self_token_stream();
							tokens.extend(quote::quote! { TupleTest(
								#pound_token field0,
								#pound_token field1
							) });
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
		let pound_token = pound_token();

		input.xmap(parse).unwrap().to_string().xpect().to_be(
			quote! {
					impl beet::prelude::TokenizeSelf for TestEnum {
						fn self_tokens(&self, tokens: &mut beet::exports::proc_macro2::TokenStream) {
							use beet::exports::quote;
							use beet::exports::proc_macro2;
							match self {
								Self::A => {
									tokens.extend(quote::quote! { TestEnum::A });
								},
								Self::B(field0) => {
									let field0 = field0.self_token_stream();
									tokens.extend(quote::quote! { TestEnum::B(
										#pound_token field0
									) });
								},
								Self::C { value } => {
									let value = value.self_token_stream();
									tokens.extend(quote::quote! { TestEnum::C {
										value: #pound_token value
									} });
								}
							}
						}
					}
				}
			.to_string(),
		);
	}
	#[test]
	fn test_generics() {
		let input: DeriveInput = syn::parse_quote! {
			struct Foo<U:Clone>{}
		};
		let pound_token = pound_token();


		input.xmap(parse).unwrap().to_string().xpect().to_be(
			quote! {
				impl<U: Clone> beet::prelude::TokenizeSelf for Foo<U> 
				where 
					U: beet::prelude::TokenizeSelf
				{
					fn self_tokens(&self, tokens: &mut beet::exports::proc_macro2::TokenStream) {
						use beet::exports::quote;
						use beet::exports::proc_macro2;
						
						let generic0 = syn::parse_str::<syn::Path>(std::any::type_name::<U>())
							.expect("failed to parse generic type from std::any::type_name");
						tokens.extend(quote::quote! { Foo::<#pound_token generic0>{ } });
					}
				}
			}
			.to_string(),
		);
	}
}
