//! Implementation of the `Get` derive macro for generating getter methods.
extern crate alloc;

use alloc::vec::Vec;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::parse_macro_input;

use super::*;

/// Entry point for the `Get` derive macro.
pub fn impl_get(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	produce(&input, "get", generate_field)
}

fn generate_field(field: &syn::Field, config: &FieldConfig) -> TokenStream {
	let field_name = field.ident.as_ref().unwrap();
	let ty = &field.ty;
	let vis = config.vis.to_tokens();
	let doc: Vec<_> = field
		.attrs
		.iter()
		.filter(|attr| attr.path().is_ident("doc"))
		.collect();

	if config.unwrap_trait {
		if let Some((_wrapper_kind, trait_ty)) = trait_wrapper_info(ty) {
			return quote! {
				#(#doc)*
				#[inline(always)]
				#vis fn #field_name(&self) -> &#trait_ty {
					&*self.#field_name
				}
			};
		}
	}

	match config.return_type {
		GetReturnType::Ref => quote! {
			#(#doc)*
			#[inline(always)]
			#vis fn #field_name(&self) -> &#ty {
				&self.#field_name
			}
		},
		GetReturnType::Clone => quote! {
			#(#doc)*
			#[inline(always)]
			#vis fn #field_name(&self) -> #ty {
				self.#field_name.clone()
			}
		},
		GetReturnType::Copy => quote! {
			#(#doc)*
			#[inline(always)]
			#vis fn #field_name(&self) -> #ty {
				self.#field_name
			}
		},
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn basic_ref_getter() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Foo {
				name: String,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn name"));
		assert!(result.contains("& self"));
		assert!(result.contains("& self . name"));
	}

	#[test]
	fn multiple_fields() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Bar {
				name: String,
				count: usize,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn name"));
		assert!(result.contains("fn count"));
	}

	#[test]
	fn skipped_field() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Baz {
				#[get(skip)]
				secret: String,
				visible: u32,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(!result.contains("fn secret"));
		assert!(result.contains("fn visible"));
	}

	#[test]
	fn copy_return_type() {
		let input: DeriveInput = syn::parse_quote! {
			#[get(copy)]
			pub struct Point {
				x_pos: f32,
				y_pos: f32,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn x_pos"));
		assert!(result.contains("self . x_pos"));
		// copy mode returns by value, no `&` before return type
		assert!(!result.contains("& f32"));
	}

	#[test]
	fn clone_return_type() {
		let input: DeriveInput = syn::parse_quote! {
			#[get(clone)]
			pub struct Names {
				first: String,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn first"));
		assert!(result.contains(". clone ()"));
	}

	#[test]
	fn preserves_generics() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Wrapper<T: Clone> {
				inner: T,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("impl < T : Clone >"));
		assert!(result.contains("fn inner"));
	}

	#[test]
	fn unwrap_trait_box() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct HasTrait {
				#[get(unwrap_trait)]
				handler: Box<dyn Handler>,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn handler"));
		assert!(result.contains("& dyn Handler"));
		assert!(result.contains("& * self . handler"));
	}
}
