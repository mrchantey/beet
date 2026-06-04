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

	// unwrap Box<dyn Trait> / Arc<dyn Trait> → &dyn Trait
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
		GetReturnType::Ref => {
			// Auto-copy for primitives: bool, char, numeric types
			if is_primitive_copy_type(ty) {
				return quote! {
					#(#doc)*
					#[inline(always)]
					#vis fn #field_name(&self) -> #ty {
						self.#field_name
					}
				};
			}
			// Auto-deref for owned string-like types: String → &str, PathBuf → &Path, etc.
			if let Some((ret_ty, accessor)) = str_like_return(ty) {
				return quote! {
					#(#doc)*
					#[inline(always)]
					#vis fn #field_name(&self) -> #ret_ty {
						self.#field_name.#accessor
					}
				};
			}
			// Default: return by reference
			quote! {
				#(#doc)*
				#[inline(always)]
				#vis fn #field_name(&self) -> &#ty {
					&self.#field_name
				}
			}
		}
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
		// String auto-deref: returns &str via as_str()
		assert!(result.contains("-> & str"));
		assert!(result.contains("as_str ()"));
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

	// -- Auto-copy for primitives --

	#[test]
	fn auto_copy_bool() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Flags {
				enabled: bool,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn enabled"));
		// returns by value, not reference
		assert!(result.contains("-> bool"));
		assert!(!result.contains("-> & bool"));
		assert!(result.contains("self . enabled"));
	}

	#[test]
	fn auto_copy_integer() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Counter {
				count: i32,
				index: usize,
			}
		};
		let result = parse(input).unwrap().to_string();
		// both numeric fields return by value
		assert!(result.contains("-> i32"));
		assert!(result.contains("-> usize"));
		assert!(!result.contains("-> & i32"));
		assert!(!result.contains("-> & usize"));
	}

	#[test]
	fn auto_copy_float() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Point {
				x: f32,
				y: f64,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("-> f32"));
		assert!(result.contains("-> f64"));
		assert!(!result.contains("-> & f32"));
		assert!(!result.contains("-> & f64"));
	}

	#[test]
	fn auto_copy_char() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Token {
				ch: char,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("-> char"));
		assert!(!result.contains("-> & char"));
	}

	/// Explicit `#[get(clone)]` on a primitive is respected over auto-copy.
	#[test]
	fn explicit_clone_overrides_auto_copy() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Foo {
				#[get(clone)]
				count: u32,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains(". clone ()"));
	}

	// -- Auto-deref for String-like types --

	#[test]
	fn auto_str_string() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Named {
				label: String,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn label"));
		// returns &str, not &String
		assert!(result.contains("-> & str"));
		assert!(!result.contains("-> & String"));
		assert!(result.contains("as_str ()"));
	}

	#[test]
	fn auto_deref_pathbuf() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Config {
				path: PathBuf,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn path"));
		// returns &::std::path::Path
		assert!(result.contains("std :: path :: Path"));
		assert!(!result.contains("-> & PathBuf"));
		assert!(result.contains("as_path ()"));
	}

	#[test]
	fn auto_deref_osstring() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct SysName {
				name: OsString,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn name"));
		// returns &::std::ffi::OsStr
		assert!(result.contains("std :: ffi :: OsStr"));
		assert!(!result.contains("-> & OsString"));
		assert!(result.contains("as_os_str ()"));
	}

	/// Explicit `#[get(clone)]` on a String is respected over auto-deref.
	#[test]
	fn explicit_clone_overrides_auto_str() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Foo {
				#[get(clone)]
				label: String,
			}
		};
		let result = parse(input).unwrap().to_string();
		// clone mode returns owned String
		assert!(result.contains(". clone ()"));
		assert!(!result.contains("as_str ()"));
	}

	/// `#[get(ref)]` does not exist; non-primitive non-String types still return by ref.
	#[test]
	fn non_primitive_non_string_returns_ref() {
		let input: DeriveInput = syn::parse_quote! {
			pub struct Wrapper {
				inner: Vec<u8>,
			}
		};
		let result = parse(input).unwrap().to_string();
		assert!(result.contains("fn inner"));
		assert!(result.contains("-> & Vec < u8 >"));
		assert!(result.contains("& self . inner"));
	}
}
