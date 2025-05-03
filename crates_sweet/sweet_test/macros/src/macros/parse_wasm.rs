use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;
use syn::ReturnType;
use syn::Type;

pub fn parse_wasm(func: &ItemFn) -> syn::Result<TokenStream> {
	let is_async = func.sig.asyncness.is_some();

	// non async tests are just #[test]
	if !is_async {
		let out = quote! {
			#[test]
			#func
		}
		.into();
		return Ok(out);
	}

	let func_inner = wrap_func_inner(&func)?;

	let attrs = &func.attrs;
	let vis = &func.vis;
	let sig = &func.sig;
	let name = &sig.ident;
	let out = quote! {
		#[cfg(target_arch = "wasm32")]
			#[test]
			#(#attrs)*
			#vis fn #name() {
					#func_inner
					sweet::prelude::SweetTestCollector::register(inner);
			}
	};

	Ok(out)
}


fn wrap_func_inner(func: &ItemFn) -> syn::Result<TokenStream> {
	let body = &func.block;

	match &func.sig.output {
		ReturnType::Default => Ok(quote! {
			async fn inner() -> sweet::exports::Result<(),String> {
				async #body.await;
				Ok(())
			}
		}),
		ReturnType::Type(_, ty) => {
			if !returns_result(ty) {
				return Err(syn::Error::new_spanned(
					ty,
					"async test functions must return Unit or Result",
				));
			}
			Ok(quote! {
				async fn inner() -> sweet::exports::Result<(),String> {
					let result:#ty = async #body.await;
					match result {
						Ok(_) => Ok(()),
						Err(err)=> Err(err.to_string()),
					}
				}
			})
		}
	}
}

fn returns_result(ty: &Box<Type>) -> bool {
	match &**ty {
		syn::Type::Path(type_path) => {
			let segments = &type_path.path.segments;
			if let Some(last_segment) = segments.last() {
				last_segment.ident == "Result"
			} else {
				false
			}
		}
		_ => false,
	}
}
