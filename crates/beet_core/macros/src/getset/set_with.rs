//! Implementation of the `SetWith` derive macro.
extern crate alloc;

use alloc::vec::Vec;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::DeriveInput;
use syn::parse_macro_input;

use super::*;

/// Entry point for the `SetWith` derive macro.
pub fn impl_set_with(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	produce(&input, "set_with", generate_field)
}

fn generate_field(field: &syn::Field, config: &FieldConfig) -> TokenStream {
	let field_name = field.ident.as_ref().unwrap();
	let ty = &field.ty;
	let fn_name = format_ident!("with_{}", field_name);
	let vis = config.vis.to_tokens();
	let doc: Vec<_> = field
		.attrs
		.iter()
		.filter(|attr| attr.path().is_ident("doc"))
		.collect();

	// Check unwrap_option first
	if config.unwrap_option {
		if let Some(inner_ty) = option_inner_type(ty) {
			// Check unwrap_trait on the inner type
			if config.unwrap_trait {
				if let Some((kind, trait_ty)) = trait_wrapper_info(inner_ty) {
					if let Some(bounds) = trait_bounds_tokens(trait_ty) {
						let wrapper = match kind {
							TraitWrapperKind::Box => quote! { Box::new(val) },
							TraitWrapperKind::Arc => {
								quote! { alloc::sync::Arc::new(val) }
							}
						};
						return quote! {
							#(#doc)*
							#[inline(always)]
							#vis fn #fn_name(mut self, val: impl #bounds) -> Self {
								self.#field_name = Some(#wrapper);
								self
							}
						};
					}
				}
			}
			return quote! {
				#(#doc)*
				#[inline(always)]
				#vis fn #fn_name(mut self, val: #inner_ty) -> Self {
					self.#field_name = Some(val);
					self
				}
			};
		}
	}

	// Check unwrap_trait
	if config.unwrap_trait {
		if let Some((kind, trait_ty)) = trait_wrapper_info(ty) {
			if let Some(bounds) = trait_bounds_tokens(trait_ty) {
				let wrapper = match kind {
					TraitWrapperKind::Box => quote! { Box::new(val) },
					TraitWrapperKind::Arc => {
						quote! { alloc::sync::Arc::new(val) }
					}
				};
				return quote! {
					#(#doc)*
					#[inline(always)]
					#vis fn #fn_name(mut self, val: impl #bounds) -> Self {
						self.#field_name = #wrapper;
						self
					}
				};
			}
		}
	}

	// Normal builder-style setter
	quote! {
		#(#doc)*
		#[inline(always)]
		#vis fn #fn_name(mut self, val: #ty) -> Self {
			self.#field_name = val;
			self
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn basic_with_setter() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Foo {
				name: String,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn with_name"));
		assert!(result.contains("mut self"));
		assert!(result.contains("val : String"));
		assert!(result.contains("self . name = val"));
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
		assert!(result.contains("fn with_name"));
		assert!(result.contains("fn with_count"));
	}

	#[test]
	fn skipped_field() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Baz {
				#[set_with(skip)]
				secret: String,
				visible: u32,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(!result.contains("fn with_secret"));
		assert!(result.contains("fn with_visible"));
	}

	#[test]
	fn unwrap_option() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Config {
				#[set_with(unwrap_option)]
				label: Option<String>,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn with_label"));
		assert!(result.contains("val : String"));
		assert!(result.contains("Some (val)"));
	}

	#[test]
	fn unwrap_trait_box() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct HasTrait {
				#[set_with(unwrap_trait)]
				handler: Box<dyn Handler>,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn with_handler"));
		assert!(result.contains("impl Handler"));
		assert!(result.contains("Box :: new (val)"));
	}

	#[test]
	fn unwrap_trait_arc() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct HasArcTrait {
				#[set_with(unwrap_trait)]
				handler: Arc<dyn Handler>,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn with_handler"));
		assert!(result.contains("impl Handler"));
		assert!(result.contains("Arc :: new (val)"));
	}

	#[test]
	fn unwrap_option_and_trait() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Both {
				#[set_with(unwrap_option, unwrap_trait)]
				handler: Option<Box<dyn Handler>>,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn with_handler"));
		assert!(result.contains("impl Handler"));
		assert!(result.contains("Some (Box :: new (val))"));
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
		assert!(result.contains("fn with_inner"));
	}

	#[test]
	fn returns_self_not_ref() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Foo {
				name: String,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("-> Self"));
		// must not contain `&mut Self` return type
		assert!(!result.contains("& mut Self"));
	}
}
