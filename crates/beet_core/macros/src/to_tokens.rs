use crate::shared_utils::pkg_ext;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn;
use syn::DeriveInput;
use syn::WherePredicate;
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
	let ident = &input.ident;
	let pound_token = syn::Token![#](Span::call_site());

	// extract the to_tokens attribute if it exists
	let constructor = input
		.attrs
		.iter()
		.find(|attr| attr.path().is_ident("to_tokens"))
		.map(|attr| attr.parse_args::<syn::Expr>().ok())
		.flatten();

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
		syn::Data::Struct(data_struct) => match &data_struct.fields {
			syn::Fields::Named(fields_named) => {
				let field_names = fields_named
					.named
					.iter()
					.map(|field| &field.ident)
					.collect::<Vec<_>>();

				let field_defs = quote!(#(let #field_names = self.#field_names.self_token_stream();)*);

				if let Some(constructor) = &constructor {
					quote! {
						#field_defs
						tokens.extend(quote::quote! {
							#constructor( #(#pound_token #field_names),* )
						});
					}
				} else {
					let field_tokens = fields_named.named.iter().map(|field| {
						let field_name = &field.ident;
						quote! {
							#field_name: #pound_token #field_name
						}
					});
					quote! {
						#field_defs
						tokens.extend(quote::quote! {
							#qualified_name{ #(#field_tokens),* }
						});
					}
				}
			}
			syn::Fields::Unnamed(fields_unnamed) => {
				let field_names = (0..fields_unnamed.unnamed.len())
					.map(|i| {
						syn::Ident::new(
							&format!("field{}", i),
							proc_macro2::Span::call_site(),
						)
					})
					.collect::<Vec<_>>();
				let field_defs =
					field_names.iter().enumerate().map(|(i, name)| {
						let index = syn::Index::from(i);
						quote! {
							let #name = self.#index.self_token_stream();
						}
					});

				if let Some(constructor) = &constructor {
					quote! {
						#(#field_defs)*
						tokens.extend(quote::quote! {
							#constructor(#(#pound_token #field_names),*)
						});
					}
				} else {
					quote! {
						#(#field_defs)*
						tokens.extend(quote::quote! {
							#qualified_name( #(#pound_token #field_names),*)
						});
					}
				}
			}
			syn::Fields::Unit => {
				quote! {
					tokens.extend(quote::quote! { #qualified_name });
				}
			}
		},
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
			let #generic_ident = short_type_path::<#ident>();
		}
	});

	let beet_core = pkg_ext::internal_or_beet("beet_core");

	let mut where_clause = where_clause
		.cloned()
		.unwrap_or_else(|| syn::parse_quote!(where));
	where_clause
		.predicates
		.extend(generic_idents.map(|(ident, _)| {
			let predicate: WherePredicate = syn::parse_quote! {
				#ident: #beet_core::prelude::TokenizeSelf
			};
			predicate
		}));


	Ok(quote! {
		impl #impl_generics #beet_core::prelude::TokenizeSelf for #ident #type_generics #where_clause {
			fn self_tokens(&self, tokens: &mut #beet_core::exports::proc_macro2::TokenStream) {
				use #beet_core::exports::quote;
				use #beet_core::exports::proc_macro2;
				#(#generic_defs)*
				#content
			}
		}
	})
}


#[cfg(test)]
mod test {
	use super::parse;
	use syn::DeriveInput;

	#[test]
	fn named_struct() {
		let input: DeriveInput = syn::parse_quote! {
			struct MyNamedStruct {
				field1: u32,
				field2: String,
			}
		};
		let result = parse(input).unwrap().to_string();
		let expected = "impl beet_core :: prelude :: TokenizeSelf for MyNamedStruct { fn self_tokens (& self , tokens : & mut beet_core :: exports :: proc_macro2 :: TokenStream) { use beet_core :: exports :: quote ; use beet_core :: exports :: proc_macro2 ; let field1 = self . field1 . self_token_stream () ; let field2 = self . field2 . self_token_stream () ; tokens . extend (quote :: quote ! { MyNamedStruct { field1 : # field1 , field2 : # field2 } }) ; } }";
		assert_eq!(expected, result);
	}

	#[test]
	fn named_struct_constructor() {
		let input: DeriveInput = syn::parse_quote! {
			#[to_tokens(Self::new)]
			struct MyNamedStruct {
				field1: u32,
				field2: String,
			}
		};
		let result = parse(input).unwrap().to_string();
		let expected = "impl beet_core :: prelude :: TokenizeSelf for MyNamedStruct { fn self_tokens (& self , tokens : & mut beet_core :: exports :: proc_macro2 :: TokenStream) { use beet_core :: exports :: quote ; use beet_core :: exports :: proc_macro2 ; let field1 = self . field1 . self_token_stream () ; let field2 = self . field2 . self_token_stream () ; tokens . extend (quote :: quote ! { Self :: new (# field1 , # field2) }) ; } }";
		assert_eq!(expected, result);
	}

	#[test]
	fn tuple_struct() {
		let input: DeriveInput = syn::parse_quote! {
			struct MyTupleStruct(u32, String);
		};

		let result = parse(input).unwrap().to_string();
		let expected = "impl beet_core :: prelude :: TokenizeSelf for MyTupleStruct { fn self_tokens (& self , tokens : & mut beet_core :: exports :: proc_macro2 :: TokenStream) { use beet_core :: exports :: quote ; use beet_core :: exports :: proc_macro2 ; let field0 = self . 0 . self_token_stream () ; let field1 = self . 1 . self_token_stream () ; tokens . extend (quote :: quote ! { MyTupleStruct (# field0 , # field1) }) ; } }";
		assert_eq!(expected, result);
	}

	#[test]
	fn tuple_struct_constructor() {
		let input: DeriveInput = syn::parse_quote! {
			#[to_tokens(Self::new)]
			struct MyTupleStruct(u32, String);
		};

		let result = parse(input).unwrap().to_string();
		let expected = "impl beet_core :: prelude :: TokenizeSelf for MyTupleStruct { fn self_tokens (& self , tokens : & mut beet_core :: exports :: proc_macro2 :: TokenStream) { use beet_core :: exports :: quote ; use beet_core :: exports :: proc_macro2 ; let field0 = self . 0 . self_token_stream () ; let field1 = self . 1 . self_token_stream () ; tokens . extend (quote :: quote ! { Self :: new (# field0 , # field1) }) ; } }";
		assert_eq!(expected, result);
	}

	#[test]
	fn r#enum() {
		let input: DeriveInput = syn::parse_quote! {
			enum MyEnum {
				A,
				B(u32),
				C { value: String },
			}
		};

		let result = parse(input).unwrap().to_string();
		let expected = "impl beet_core :: prelude :: TokenizeSelf for MyEnum { fn self_tokens (& self , tokens : & mut beet_core :: exports :: proc_macro2 :: TokenStream) { use beet_core :: exports :: quote ; use beet_core :: exports :: proc_macro2 ; match self { Self :: A => { tokens . extend (quote :: quote ! { MyEnum :: A }) ; } , Self :: B (field0) => { let field0 = field0 . self_token_stream () ; tokens . extend (quote :: quote ! { MyEnum :: B (# field0) }) ; } , Self :: C { value } => { let value = value . self_token_stream () ; tokens . extend (quote :: quote ! { MyEnum :: C { value : # value } }) ; } } } }";
		assert_eq!(expected, result);
	}

	#[test]
	fn generics() {
		let input: DeriveInput = syn::parse_quote! {
			struct MyGenericStruct<U:Clone>{}
		};

		let result = parse(input).unwrap().to_string();
		let expected = "impl < U : Clone > beet_core :: prelude :: TokenizeSelf for MyGenericStruct < U > where U : beet_core :: prelude :: TokenizeSelf { fn self_tokens (& self , tokens : & mut beet_core :: exports :: proc_macro2 :: TokenStream) { use beet_core :: exports :: quote ; use beet_core :: exports :: proc_macro2 ; let generic0 = short_type_path :: < U > () ; tokens . extend (quote :: quote ! { MyGenericStruct :: < # generic0 > { } }) ; } }";
		assert_eq!(expected, result);
	}
}
