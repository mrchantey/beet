//! Implementation of the `GetMut` derive macro.
extern crate alloc;

use alloc::vec::Vec;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::DeriveInput;
use syn::parse_macro_input;

use super::*;

/// Entry point for the `GetMut` derive macro.
pub fn impl_get_mut(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	produce(&input, "get_mut", generate_field)
}

fn generate_field(field: &syn::Field, config: &FieldConfig) -> TokenStream {
	let field_name = field.ident.as_ref().unwrap();
	let ty = &field.ty;
	let fn_name = format_ident!("{}_mut", field_name);
	let vis = config.vis.to_tokens();
	let doc: Vec<_> = field
		.attrs
		.iter()
		.filter(|attr| attr.path().is_ident("doc"))
		.collect();

	quote! {
		#(#doc)*
		#[inline(always)]
		#vis fn #fn_name(&mut self) -> &mut #ty {
			&mut self.#field_name
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn basic_get_mut() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Foo {
				field: String,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn field_mut"));
		assert!(result.contains("& mut self"));
		assert!(result.contains("& mut self . field"));
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
		assert!(result.contains("fn name_mut"));
		assert!(result.contains("fn count_mut"));
	}

	#[test]
	fn skipped_field() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Baz {
				#[get_mut(skip)]
				secret: String,
				visible: u32,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(!result.contains("fn secret_mut"));
		assert!(result.contains("fn visible_mut"));
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
		assert!(result.contains("fn inner_mut"));
	}
}
