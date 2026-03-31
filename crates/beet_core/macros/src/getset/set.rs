//! Implementation of the `Set` derive macro.
extern crate alloc;

use alloc::vec::Vec;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::DeriveInput;
use syn::parse_macro_input;

use super::*;

/// Entry point for the `Set` derive macro.
pub fn impl_set(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(input: DeriveInput) -> syn::Result<TokenStream> {
	produce(&input, "set", generate_field)
}

fn generate_field(field: &syn::Field, config: &FieldConfig) -> TokenStream {
	let field_name = field.ident.as_ref().unwrap();
	let ty = &field.ty;
	let fn_name = format_ident!("set_{}", field_name);
	let vis = config.vis.to_tokens();
	let doc: Vec<_> = field
		.attrs
		.iter()
		.filter(|attr| attr.path().is_ident("doc"))
		.collect();

	// When unwrap_option is set, unwrap the Option layer first
	if config.unwrap_option {
		if let Some(inner_ty) = option_inner_type(ty) {
			// Option<Box<dyn Trait>> or Option<Arc<dyn Trait>>
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
							#vis fn #fn_name(&mut self, val: impl #bounds) -> &mut Self {
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
				#vis fn #fn_name(&mut self, val: #inner_ty) -> &mut Self {
					self.#field_name = Some(val);
					self
				}
			};
		}
	}

	// Box<dyn Trait> or Arc<dyn Trait>
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
					#vis fn #fn_name(&mut self, val: impl #bounds) -> &mut Self {
						self.#field_name = #wrapper;
						self
					}
				};
			}
		}
	}

	// Normal setter
	quote! {
		#(#doc)*
		#[inline(always)]
		#vis fn #fn_name(&mut self, val: #ty) -> &mut Self {
			self.#field_name = val;
			self
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn basic_setter() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Foo {
				name: String,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn set_name"));
		assert!(result.contains("& mut self"));
		assert!(result.contains("& mut Self"));
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
		assert!(result.contains("fn set_name"));
		assert!(result.contains("fn set_count"));
	}

	#[test]
	fn skipped_field() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Baz {
				#[set(skip)]
				secret: String,
				visible: u32,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(!result.contains("fn set_secret"));
		assert!(result.contains("fn set_visible"));
	}

	#[test]
	fn unwrap_option() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Opt {
				#[set(unwrap_option)]
				label: Option<String>,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn set_label"));
		assert!(result.contains("val : String"));
		assert!(result.contains("Some (val)"));
	}

	#[test]
	fn unwrap_trait_box() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct HasTrait {
				#[set(unwrap_trait)]
				handler: Box<dyn Handler>,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn set_handler"));
		assert!(result.contains("impl Handler"));
		assert!(result.contains("Box :: new (val)"));
	}

	#[test]
	fn unwrap_trait_arc() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct HasArc {
				#[set(unwrap_trait)]
				handler: Arc<dyn Handler>,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn set_handler"));
		assert!(result.contains("impl Handler"));
		assert!(result.contains("Arc :: new (val)"));
	}

	#[test]
	fn unwrap_option_and_trait() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Combo {
				#[set(unwrap_option, unwrap_trait)]
				handler: Option<Box<dyn Handler>>,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn set_handler"));
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
		assert!(result.contains("fn set_inner"));
	}
}
