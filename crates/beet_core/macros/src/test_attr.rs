extern crate alloc;

use alloc::vec;
use beet_core_shared::prelude::*;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::ItemFn;

/// Parsed `#[ignore]` / `#[should_panic]` metadata from the test fn attrs.
struct StdTestAttrs {
	ignore: bool,
	ignore_message: Option<syn::LitStr>,
	should_panic: TokenStream,
	has_should_panic: bool,
}

fn parse_std_attrs(
	attrs: &[syn::Attribute],
	beet_core: &syn::Path,
) -> syn::Result<StdTestAttrs> {
	let mut ignore = false;
	let mut ignore_message = None;
	let mut should_panic = quote! { #beet_core::testing::ShouldPanic::No };
	let mut has_should_panic = false;

	for attr in attrs {
		if attr.path().is_ident("should_panic") {
			has_should_panic = true;
		}
		if attr.path().is_ident("ignore") {
			ignore = true;
			if let syn::Meta::NameValue(nv) = &attr.meta {
				if let syn::Expr::Lit(syn::ExprLit {
					lit: syn::Lit::Str(s),
					..
				}) = &nv.value
				{
					ignore_message = Some(s.clone());
				}
			}
		} else if attr.path().is_ident("should_panic") {
			match &attr.meta {
				syn::Meta::Path(_) => {
					should_panic =
						quote! { #beet_core::testing::ShouldPanic::Yes };
				}
				syn::Meta::List(_) => {
					// `#[should_panic(expected = "msg")]`
					let nv: syn::MetaNameValue = attr.parse_args()?;
					if !nv.path.is_ident("expected") {
						return Err(syn::Error::new_spanned(
							&nv.path,
							"expected `expected = \"...\"`",
						));
					}
					let msg = nv.value;
					should_panic = quote! {
						#beet_core::testing::ShouldPanic::YesWithMessage(#msg)
					};
				}
				syn::Meta::NameValue(nv) => {
					let msg = &nv.value;
					should_panic = quote! {
						#beet_core::testing::ShouldPanic::YesWithMessage(#msg)
					};
				}
			}
		}
	}

	Ok(StdTestAttrs {
		ignore,
		ignore_message,
		should_panic,
		has_should_panic,
	})
}

pub fn impl_test_attr(
	attr: proc_macro::TokenStream,
	input: proc_macro::TokenStream,
) -> syn::Result<TokenStream> {
	let func = syn::parse::<ItemFn>(input)?;

	let attr_tokens: TokenStream = attr.into();

	let attrs = if attr_tokens.is_empty() {
		AttributeGroup { attributes: vec![] }
	} else {
		let synthetic_attr: syn::Attribute =
			syn::parse_quote!(#[beet(#attr_tokens)]);
		AttributeGroup::parse(&[synthetic_attr], "beet")?
	};

	attrs.validate_allowed_keys(&["timeout_ms"])?;

	let timeout_ms = attrs.get_value_parsed::<syn::LitInt>("timeout_ms")?;
	let beet_core = pkg_ext::internal_or_beet("beet_core");

	let params_expr = if let Some(timeout_lit) = timeout_ms {
		quote! {
			#beet_core::testing::TestCaseParams::new().with_timeout_ms(#timeout_lit)
		}
	} else {
		quote! {
			#beet_core::testing::TestCaseParams::new()
		}
	};

	let ident = &func.sig.ident;
	#[cfg(feature = "custom_test_frameworks")]
	let vis = &func.vis;
	let func_attrs = &func.attrs;
	let block = &func.block;
	let output = &func.sig.output;
	let is_async = func.sig.asyncness.is_some();

	let StdTestAttrs {
		ignore,
		ignore_message,
		should_panic,
		has_should_panic,
	} = parse_std_attrs(func_attrs, &beet_core)?;
	#[cfg(not(feature = "custom_test_frameworks"))]
	let _ = has_should_panic;

	let ignore_message = match ignore_message {
		Some(lit) => quote! { ::core::option::Option::Some(#lit) },
		None => quote! { ::core::option::Option::None },
	};

	let run_ident = format_ident!("__beet_run_{}", ident);

	// Body that executes the test and yields `Result<(), String>`.
	let run_body = if is_async {
		let async_block = match output {
			syn::ReturnType::Default => quote! { async #block },
			syn::ReturnType::Type(_, ty) => quote! {
				async {
					let out: #ty = async #block.await;
					out
				}
			},
		};
		quote! {
			#beet_core::testing::register_test(#params_expr, #async_block);
			::core::result::Result::Ok(())
		}
	} else {
		quote! {
			fn #ident() #output #block
			#beet_core::testing::IntoTestResult::into_test_result(#ident())
		}
	};

	let run_fn = quote! {
		#[allow(dead_code)]
		// The return type is named through the `testing` re-export (not `_alloc`
		// or `std`) so the macro resolves the same way `BeetTestCase` does:
		// integration tests / downstream crates import `beet_core::testing` but
		// not `_alloc`, and `std::string::String` is absent on the no_std device.
		fn #run_ident() -> ::core::result::Result<(), #beet_core::testing::TestError> {
			#run_body
		}
	};

	// The unique static name linkme needs for the embedded registration; the
	// inventory path ignores it. Derived from the fn ident like `__beet_run_*`.
	let case_ident = format_ident!("__beet_test_case_{}", ident);
	let registration = quote! {
		#beet_core::testing::submit! {
			#case_ident,
			#beet_core::testing::BeetTestCase::new(
				concat!(module_path!(), "::", stringify!(#ident)),
				file!(),
				line!(),
				column!(),
				#should_panic,
				#ignore,
				#ignore_message,
				#run_ident,
			)
		}
	};

	// Nightly libtest path: also emit a `#[test]` fn so the
	// `custom_test_frameworks` harness can collect it. Inert on
	// `harness = false` targets.
	// libtest forbids `#[should_panic]` on fns that return `Result`, so
	// those return `()` (the panic is what libtest / the beet runner checks).
	#[cfg(feature = "custom_test_frameworks")]
	let libtest_fn = if has_should_panic {
		quote! {
			#[test]
			#[allow(dead_code)]
			#(#func_attrs)*
			#vis fn #ident() {
				let _ = #run_ident();
			}
		}
	} else {
		quote! {
			#[test]
			#[allow(dead_code)]
			#(#func_attrs)*
			#vis fn #ident() -> ::core::result::Result<(), ::std::string::String> {
				#run_ident()
			}
		}
	};
	#[cfg(not(feature = "custom_test_frameworks"))]
	let libtest_fn = quote! {};

	Ok(quote! {
		#run_fn
		#registration
		#libtest_fn
	})
}
