extern crate alloc;

use alloc::string::ToString;
use alloc::vec;
use beet_core_shared::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;

pub fn parse_test_attr(
	attr: proc_macro::TokenStream,
	input: proc_macro::TokenStream,
) -> syn::Result<TokenStream> {
	let func = syn::parse::<ItemFn>(input)?;

	// Convert proc_macro::TokenStream to proc_macro2::TokenStream
	let attr_tokens: TokenStream = attr.into();

	// Parse attributes using AttributeGroup
	let attrs = if attr_tokens.is_empty() {
		AttributeGroup { attributes: vec![] }
	} else {
		// Create a synthetic attribute to parse
		let synthetic_attr: syn::Attribute =
			syn::parse_quote!(#[beet(#attr_tokens)]);
		AttributeGroup::parse(&[synthetic_attr], "beet")?
	};

	attrs.validate_allowed_keys(&["timeout_ms", "tokio"])?;

	let timeout_ms = attrs.get_value_parsed::<syn::LitInt>("timeout_ms")?;
	let is_tokio = attrs.contains("tokio");

	// For inventory registration and generated code we need a path that
	// always resolves to `beet_core`, even from integration tests where
	// `crate` refers to the test binary root, not to the library.
	// `internal_or_beet` returns `crate` when the package name matches,
	// which breaks in integration tests. We therefore always use the
	// explicit crate name.
	let beet_core: syn::Path = if pkg_ext::is_internal() {
		syn::parse_str("beet_core").unwrap()
	} else {
		syn::parse_str("beet").unwrap()
	};

	// Build test params using the always-resolvable path
	let params_expr = if let Some(timeout_lit) = &timeout_ms {
		quote! {
			#beet_core::testing::TestCaseParams::new().with_timeout_ms(#timeout_lit)
		}
	} else {
		quote! {
			#beet_core::testing::TestCaseParams::new()
		}
	};

	let is_async = func.sig.asyncness.is_some();

	Ok(match (is_async, is_tokio) {
		(true, true) => {
			let non_tokio_attrs = attrs.attributes.iter().filter(|attr| {
				attr.name()
					.map(|name| name.to_string() != "tokio")
					.unwrap_or(true)
			});

			// wasm impl is recursive but without tokio marker
			quote! {
				#[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
				#[cfg_attr(target_arch = "wasm32", #beet_core::test(#(#non_tokio_attrs),*))]
				#func
			}
		}
		(true, false) => gen_async_test(&func, &beet_core, &params_expr),
		(false, _) => gen_sync_test(&func, &beet_core),
	})
}


/// Generate a sync test that works on both stable and nightly.
///
/// The decision between nightly and stable paths is made at macro compile
/// time via the `custom_test_framework` feature on `beet_core_macros`,
/// so no `cfg(feature = ...)` attributes are emitted into the consumer crate.
fn gen_sync_test(func: &ItemFn, beet_core: &syn::Path) -> TokenStream {
	let ident = &func.sig.ident;
	let vis = &func.vis;
	let block = &func.block;
	let attrs = &func.attrs;
	let sig_inputs = &func.sig.inputs;
	let sig_output = &func.sig.output;
	let name_str = ident.to_string();

	let inventory_entry = gen_inventory_entry(ident, &name_str, beet_core);

	quote! {
		#[test]
		#(#attrs)*
		#vis fn #ident(#sig_inputs) #sig_output {
			#block
		}

		#inventory_entry
	}
}


/// Generate an async test that works on both stable and nightly.
///
/// On nightly (`custom_test_framework` feature on `beet_core_macros`):
/// emits `#[test]` + `register_test` so the custom runner can drive
/// the async test through the ECS.
///
/// On stable: emits `#[test]` with `block_on` for the standard harness,
/// AND registers via `inventory` for the beet runner.
///
/// The choice is made at proc-macro compile time so no `cfg` attributes
/// leak into the consumer crate.
fn gen_async_test(
	func: &ItemFn,
	beet_core: &syn::Path,
	params_expr: &TokenStream,
) -> TokenStream {
	let ident = &func.sig.ident;
	let vis = &func.vis;
	let block = &func.block;
	let attrs = &func.attrs;
	let name_str = ident.to_string();

	let inventory_entry = gen_inventory_entry(ident, &name_str, beet_core);

	// Decide at macro compile time which path to emit.
	// When the `custom_test_framework` feature is active on beet_core_macros,
	// we emit nightly-style `register_test`. Otherwise we emit the stable
	// `block_on_async_test` fallback. This avoids emitting
	// `cfg(feature = "custom_test_framework")` into the consumer crate where
	// that feature is unknown, eliminating the `unexpected_cfgs` warnings.
	if cfg!(feature = "custom_test_framework") {
		// Nightly: register the async body for the custom test runner
		quote! {
			#[test]
			#(#attrs)*
			#vis fn #ident() {
				#beet_core::testing::register_test(
					#params_expr,
					async #block
				);
			}

			#inventory_entry
		}
	} else {
		// Stable: block on the async body directly
		quote! {
			#[test]
			#(#attrs)*
			#vis fn #ident() {
				#beet_core::testing::block_on_async_test(
					async #block
				);
			}

			#inventory_entry
		}
	}
}

/// Generate the `inventory::submit!` block that registers a test entry.
///
/// Uses the explicit crate name (never `crate::`) so this works from
/// both unit tests and integration tests.
fn gen_inventory_entry(
	ident: &syn::Ident,
	name_str: &str,
	beet_core: &syn::Path,
) -> TokenStream {
	quote! {
		#beet_core::inventory::submit! {
			#beet_core::testing::InventoryTestEntry {
				desc: #beet_core::testing::TestDesc {
					name: #beet_core::testing::TestName::StaticTestName(
						concat!(module_path!(), "::", #name_str)
					),
					ignore: false,
					ignore_message: None,
					source_file: file!(),
					start_line: line!() as usize,
					start_col: column!() as usize,
					end_line: line!() as usize,
					end_col: column!() as usize,
					compile_fail: false,
					no_run: false,
					should_panic: #beet_core::testing::ShouldPanic::No,
					test_type: #beet_core::testing::TestType::UnitTest,
				},
				func: {
					fn __beet_test_wrapper() -> Result<(), String> {
						#ident();
						Ok(())
					}
					__beet_test_wrapper
				},
			}
		}
	}
}
