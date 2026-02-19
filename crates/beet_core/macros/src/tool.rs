use beet_core_shared::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::FnArg;
use syn::ItemFn;
use syn::ReturnType;
use syn::Type;
use syn::parse_macro_input;


pub fn impl_tool(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	parse(attr.into(), parse_macro_input!(item as ItemFn))
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

/// Collect all parameters from a function signature as (name, type) pairs.
fn collect_params(item: &ItemFn) -> syn::Result<Vec<(syn::Ident, Box<Type>)>> {
	let mut params = Vec::new();
	for arg in &item.sig.inputs {
		match arg {
			FnArg::Typed(pat_type) => {
				if let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
					params.push((pat_ident.ident.clone(), pat_type.ty.clone()));
				} else {
					synbail!(
						&pat_type.pat,
						"tool parameters must be simple identifiers",
					);
				}
			}
			FnArg::Receiver(recv) => {
				synbail!(
					recv,
					"`self` parameters are not supported in tool functions",
				);
			}
		}
	}
	Ok(params)
}

fn parse(attr: TokenStream, item: ItemFn) -> syn::Result<TokenStream> {
	let attrs = AttributeMap::parse(attr)?;
	attrs.assert_types(&[], &["result_out"])?;
	let result_out = attrs.contains_key("result_out");

	let is_async = item.sig.asyncness.is_some();

	let params = collect_params(&item)?;

	// Detect tool kind and passthrough.
	let first_param_type = params.first().map(|(_, ty)| ty.as_ref());

	if is_async {
		// Check for async passthrough: first param is AsyncToolIn<T>.
		if let Some(first_ty) = first_param_type {
			if let Some(inner) = extract_wrapper_type(first_ty, "AsyncToolIn") {
				return parse_async_passthrough(
					&item, result_out, inner, &params,
				);
			}
		}
		parse_async_tool(&item, result_out, &params)
	} else if let Some(first_ty) = first_param_type {
		if let Some(in_inner) = extract_wrapper_type(first_ty, "In") {
			// First param is In<T>, determine which sub-case.
			if let Some(inner) = extract_wrapper_type(in_inner, "SystemToolIn")
			{
				// In<SystemToolIn<T>> → system passthrough
				parse_system_passthrough(&item, result_out, inner, &params)
			} else if let Some(inner) =
				extract_wrapper_type(in_inner, "FuncToolIn")
			{
				// In<FuncToolIn<T>> → func passthrough
				parse_func_passthrough(&item, result_out, inner, &params)
			} else {
				// In<T> → system tool
				parse_system_tool(&item, result_out, in_inner, &params)
			}
		} else if let Some(inner) = extract_wrapper_type(first_ty, "FuncToolIn")
		{
			// FuncToolIn<T> (no In<>) → func passthrough
			parse_func_passthrough(&item, result_out, inner, &params)
		} else {
			// No special first param → func tool (current behavior)
			parse_func_tool(&item, result_out, &params)
		}
	} else {
		// No params → func tool
		parse_func_tool(&item, result_out, &params)
	}
}


// ---------------------------------------------------------------------------
// Func tool (the original behavior)
// ---------------------------------------------------------------------------

fn parse_func_tool(
	item: &ItemFn,
	result_out: bool,
	params: &[(syn::Ident, Box<Type>)],
) -> syn::Result<TokenStream> {
	let beet_tool = pkg_ext::internal_or_beet("beet_tool");
	let vis = &item.vis;
	let fn_name = &item.sig.ident;
	let body = &item.block;

	let (in_type, out_type) = compute_in_out(params, item, result_out)?;

	let destructure = make_destructure(params);
	let body_wrap = make_body_wrap(body, item, result_out);

	Ok(quote! {
		#[allow(non_camel_case_types)]
		#vis struct #fn_name;

		impl #beet_tool::prelude::IntoTool<#fn_name> for #fn_name {
			type In = #in_type;
			type Out = #out_type;

			fn into_tool(self) -> #beet_tool::prelude::Tool<Self::In, Self::Out> {
				#beet_tool::prelude::func_tool(|input: #beet_tool::prelude::FuncToolIn<#in_type>| {
					#destructure
					#body_wrap
				})
			}
		}
	})
}

/// Func passthrough: first param is `FuncToolIn<T>` or `In<FuncToolIn<T>>`.
/// The user gets the full [`FuncToolIn`] context.
fn parse_func_passthrough(
	item: &ItemFn,
	result_out: bool,
	inner_type: &Type,
	params: &[(syn::Ident, Box<Type>)],
) -> syn::Result<TokenStream> {
	if params.len() != 1 {
		synbail!(
			&item.sig,
			"FuncToolIn passthrough expects exactly one parameter"
		);
	}

	let beet_tool = pkg_ext::internal_or_beet("beet_tool");
	let vis = &item.vis;
	let fn_name = &item.sig.ident;
	let body = &item.block;
	let param_name = &params[0].0;

	let in_type = quote! { #inner_type };
	let out_type = compute_out_type(item, result_out);
	let body_wrap = make_body_wrap(body, item, result_out);

	Ok(quote! {
		#[allow(non_camel_case_types)]
		#vis struct #fn_name;

		impl #beet_tool::prelude::IntoTool<#fn_name> for #fn_name {
			type In = #in_type;
			type Out = #out_type;

			fn into_tool(self) -> #beet_tool::prelude::Tool<Self::In, Self::Out> {
				#beet_tool::prelude::func_tool(|#param_name: #beet_tool::prelude::FuncToolIn<#in_type>| {
					#body_wrap
				})
			}
		}
	})
}


// ---------------------------------------------------------------------------
// Async tool
// ---------------------------------------------------------------------------

fn parse_async_tool(
	item: &ItemFn,
	result_out: bool,
	params: &[(syn::Ident, Box<Type>)],
) -> syn::Result<TokenStream> {
	let beet_tool = pkg_ext::internal_or_beet("beet_tool");
	let vis = &item.vis;
	let fn_name = &item.sig.ident;
	let body = &item.block;

	let (in_type, out_type) = compute_in_out(params, item, result_out)?;

	let destructure = make_destructure(params);
	let body_wrap = make_body_wrap(body, item, result_out);

	Ok(quote! {
		#[allow(non_camel_case_types)]
		#vis struct #fn_name;

		impl #beet_tool::prelude::IntoTool<#fn_name> for #fn_name {
			type In = #in_type;
			type Out = #out_type;

			fn into_tool(self) -> #beet_tool::prelude::Tool<Self::In, Self::Out> {
				#beet_tool::prelude::async_tool(|input: #beet_tool::prelude::AsyncToolIn<#in_type>| async move {
					#destructure
					#body_wrap
				})
			}
		}
	})
}

/// Async passthrough: first param is `AsyncToolIn<T>`.
fn parse_async_passthrough(
	item: &ItemFn,
	result_out: bool,
	inner_type: &Type,
	params: &[(syn::Ident, Box<Type>)],
) -> syn::Result<TokenStream> {
	if params.len() != 1 {
		synbail!(
			&item.sig,
			"AsyncToolIn passthrough expects exactly one parameter"
		);
	}

	let beet_tool = pkg_ext::internal_or_beet("beet_tool");
	let vis = &item.vis;
	let fn_name = &item.sig.ident;
	let body = &item.block;
	let param_name = &params[0].0;

	let in_type = quote! { #inner_type };
	let out_type = compute_out_type(item, result_out);
	let body_wrap = make_body_wrap(body, item, result_out);

	Ok(quote! {
		#[allow(non_camel_case_types)]
		#vis struct #fn_name;

		impl #beet_tool::prelude::IntoTool<#fn_name> for #fn_name {
			type In = #in_type;
			type Out = #out_type;

			fn into_tool(self) -> #beet_tool::prelude::Tool<Self::In, Self::Out> {
				#beet_tool::prelude::async_tool(|#param_name: #beet_tool::prelude::AsyncToolIn<#in_type>| async move {
					#body_wrap
				})
			}
		}
	})
}


// ---------------------------------------------------------------------------
// System tool
// ---------------------------------------------------------------------------

/// System tool: first param is `In<T>`, remaining are system params.
fn parse_system_tool(
	item: &ItemFn,
	result_out: bool,
	in_inner: &Type,
	params: &[(syn::Ident, Box<Type>)],
) -> syn::Result<TokenStream> {
	let beet_tool = pkg_ext::internal_or_beet("beet_tool");
	let vis = &item.vis;
	let fn_name = &item.sig.ident;
	let body = &item.block;

	let first_param_name = &params[0].0;
	let in_type = quote! { #in_inner };
	let out_type = compute_out_type(item, result_out);
	let body_wrap = make_body_wrap(body, item, result_out);

	// Collect remaining system params (everything after the first).
	let system_params = collect_system_params(item, 1);

	Ok(quote! {
		#[allow(non_camel_case_types)]
		#vis struct #fn_name;

		impl #beet_tool::prelude::IntoTool<#fn_name> for #fn_name {
			type In = #in_type;
			type Out = #out_type;

			fn into_tool(self) -> #beet_tool::prelude::Tool<Self::In, Self::Out> {
				#beet_tool::prelude::system_tool(|
					In(__tool_in): In<#beet_tool::prelude::SystemToolIn<#in_type>>
					#(, #system_params)*
				| -> Result<#out_type> {
					let #first_param_name = In(__tool_in.input);
					#body_wrap
				})
			}
		}
	})
}

/// System passthrough: first param is `In<SystemToolIn<T>>`.
fn parse_system_passthrough(
	item: &ItemFn,
	result_out: bool,
	inner_type: &Type,
	params: &[(syn::Ident, Box<Type>)],
) -> syn::Result<TokenStream> {
	let beet_tool = pkg_ext::internal_or_beet("beet_tool");
	let vis = &item.vis;
	let fn_name = &item.sig.ident;
	let body = &item.block;

	let first_param_name = &params[0].0;
	let in_type = quote! { #inner_type };
	let out_type = compute_out_type(item, result_out);
	let body_wrap = make_body_wrap(body, item, result_out);

	let system_params = collect_system_params(item, 1);

	Ok(quote! {
		#[allow(non_camel_case_types)]
		#vis struct #fn_name;

		impl #beet_tool::prelude::IntoTool<#fn_name> for #fn_name {
			type In = #in_type;
			type Out = #out_type;

			fn into_tool(self) -> #beet_tool::prelude::Tool<Self::In, Self::Out> {
				#beet_tool::prelude::system_tool(|
					In(#first_param_name): In<#beet_tool::prelude::SystemToolIn<#in_type>>
					#(, #system_params)*
				| -> Result<#out_type> {
					#body_wrap
				})
			}
		}
	})
}


// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Compute the `In` and `Out` types for func and async tools (all params
/// contribute to the input type).
fn compute_in_out(
	params: &[(syn::Ident, Box<Type>)],
	item: &ItemFn,
	result_out: bool,
) -> syn::Result<(TokenStream, TokenStream)> {
	let param_types: Vec<&Type> =
		params.iter().map(|(_, ty)| ty.as_ref()).collect();

	let in_type: TokenStream = match param_types.len() {
		0 => quote! { () },
		1 => {
			let ty = param_types[0];
			quote! { #ty }
		}
		_ => {
			let types = &param_types;
			quote! { (#(#types),*) }
		}
	};

	let out_type = compute_out_type(item, result_out);
	Ok((in_type, out_type))
}

/// Compute the output type from the function signature.
fn compute_out_type(item: &ItemFn, result_out: bool) -> TokenStream {
	let raw_return_type: Option<&Type> = match &item.sig.output {
		ReturnType::Default => None,
		ReturnType::Type(_, ty) => Some(ty.as_ref()),
	};

	let returns_result = raw_return_type
		.map(|ty| is_result_type(ty))
		.unwrap_or(false);

	if let Some(ty) = raw_return_type {
		if returns_result && !result_out {
			if let Some(inner) = extract_result_inner(ty) {
				quote! { #inner }
			} else {
				quote! { () }
			}
		} else {
			quote! { #ty }
		}
	} else {
		quote! { () }
	}
}

/// Generate the `let ... = input.input;` destructuring for func/async tools.
fn make_destructure(params: &[(syn::Ident, Box<Type>)]) -> TokenStream {
	match params.len() {
		0 => quote! { let _ = input.input; },
		1 => {
			let name = &params[0].0;
			quote! { let #name = input.input; }
		}
		_ => {
			let names: Vec<&syn::Ident> =
				params.iter().map(|(name, _)| name).collect();
			quote! { let (#(#names),*) = input.input; }
		}
	}
}

/// Wrap the function body, adding `Ok(...)` if needed or passing through
/// if the function already returns [`Result`].
fn make_body_wrap(
	body: &syn::Block,
	item: &ItemFn,
	result_out: bool,
) -> TokenStream {
	let raw_return_type: Option<&Type> = match &item.sig.output {
		ReturnType::Default => None,
		ReturnType::Type(_, ty) => Some(ty.as_ref()),
	};

	let returns_result = raw_return_type
		.map(|ty| is_result_type(ty))
		.unwrap_or(false);

	if returns_result && !result_out {
		quote! {
			#[allow(unused_braces)]
			#body
		}
	} else {
		quote! { Ok(
			#[allow(unused_braces)]
			#body
			)
		}
	}
}

/// Collect system params from the function signature, skipping the first
/// `skip` params. Returns the raw [`FnArg`] tokens.
fn collect_system_params(item: &ItemFn, skip: usize) -> Vec<&FnArg> {
	item.sig.inputs.iter().skip(skip).collect()
}


// ---------------------------------------------------------------------------
// Type extraction helpers
// ---------------------------------------------------------------------------

/// Extract the inner type `T` from a wrapper type `Wrapper<T>`, matching
/// only on the last path segment name.
fn extract_wrapper_type<'a>(ty: &'a Type, name: &str) -> Option<&'a Type> {
	if let Type::Path(type_path) = ty {
		if let Some(segment) = type_path.path.segments.last() {
			if segment.ident == name {
				if let syn::PathArguments::AngleBracketed(args) =
					&segment.arguments
				{
					if let Some(syn::GenericArgument::Type(inner)) =
						args.args.first()
					{
						return Some(inner);
					}
				}
			}
		}
	}
	None
}

/// Whether the return type path ends with `Result`.
fn is_result_type(ty: &Type) -> bool {
	if let Type::Path(type_path) = ty {
		if let Some(segment) = type_path.path.segments.last() {
			return segment.ident == "Result";
		}
	}
	false
}

/// Extract the inner `T` from `Result<T>` or `Result<T, E>`.
fn extract_result_inner(ty: &Type) -> Option<&Type> {
	extract_wrapper_type(ty, "Result")
}


#[cfg(test)]
mod test {
	use super::*;
	use quote::quote;

	fn parse_str(attr: TokenStream, item: syn::ItemFn) -> String {
		parse(attr, item).unwrap().to_string()
	}

	// -----------------------------------------------------------------------
	// Func tool tests (original behavior)
	// -----------------------------------------------------------------------

	#[test]
	fn no_args_no_return() {
		let result = parse_str(quote!(), syn::parse_quote! { fn my_tool() {} });
		assert!(result.contains("struct my_tool"));
		assert!(result.contains("type In = ()"));
		assert!(result.contains("type Out = ()"));
		assert!(result.contains("let _ = input . input"));
		assert!(result.contains("func_tool"));
	}

	#[test]
	fn args_with_return() {
		let result = parse_str(
			quote!(),
			syn::parse_quote! { fn add(a: i32, b: i32) -> i32 { a + b } },
		);
		assert!(result.contains("struct add"));
		assert!(result.contains("type In = (i32 , i32)"));
		assert!(result.contains("type Out = i32"));
		assert!(result.contains("let (a , b) = input . input"));
		assert!(result.contains("func_tool"));
		assert!(!result.contains("} ?"));
	}

	#[test]
	fn single_arg() {
		let result = parse_str(
			quote!(),
			syn::parse_quote! { fn double(val: i32) -> i32 { val * 2 } },
		);
		assert!(result.contains("type In = i32"));
		assert!(result.contains("let val = input . input"));
		assert!(result.contains("func_tool"));
	}

	#[test]
	fn result_return() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn fallible(a: i32, b: i32) -> Result<i32> { Ok(a + b) }
		});
		assert!(result.contains("type Out = i32"));
		assert!(result.contains("func_tool"));
	}

	#[test]
	fn result_out_flag() {
		let result = parse_str(quote!(result_out), syn::parse_quote! {
			fn fallible(a: i32) -> Result<i32> { Ok(a) }
		});
		assert!(result.contains("type Out = Result < i32 >"));
		assert!(!result.contains("} ?"));
	}

	#[test]
	fn visibility_preserved() {
		let result =
			parse_str(quote!(), syn::parse_quote! { pub fn public_tool() {} });
		assert!(result.contains("pub struct public_tool"));
	}

	// -----------------------------------------------------------------------
	// Func passthrough
	// -----------------------------------------------------------------------

	#[test]
	fn func_passthrough_func_tool_in() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(cx: FuncToolIn<i32>) -> i32 { *cx }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("func_tool"));
		// The param name should be forwarded directly, no destructuring.
		assert!(result.contains("| cx : "));
		assert!(!result.contains("let cx = input"));
	}

	#[test]
	fn func_passthrough_in_func_tool_in() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(cx: In<FuncToolIn<String>>) -> String { cx.input.clone() }
		});
		assert!(result.contains("type In = String"));
		assert!(result.contains("func_tool"));
		assert!(result.contains("| cx : "));
	}

	// -----------------------------------------------------------------------
	// Async tool tests
	// -----------------------------------------------------------------------

	#[test]
	fn async_no_args() {
		let result = parse_str(
			quote!(),
			syn::parse_quote! { async fn my_tool() -> i32 { 42 } },
		);
		assert!(result.contains("struct my_tool"));
		assert!(result.contains("type In = ()"));
		assert!(result.contains("type Out = i32"));
		assert!(result.contains("async_tool"));
		assert!(result.contains("async move"));
	}

	#[test]
	fn async_with_args() {
		let result = parse_str(quote!(), syn::parse_quote! {
			async fn fetch(url: String, timeout: u32) -> String { url }
		});
		assert!(result.contains("type In = (String , u32)"));
		assert!(result.contains("type Out = String"));
		assert!(result.contains("async_tool"));
		assert!(result.contains("let (url , timeout) = input . input"));
	}

	#[test]
	fn async_single_arg() {
		let result = parse_str(quote!(), syn::parse_quote! {
			async fn negate(val: i32) -> i32 { -val }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("async_tool"));
		assert!(result.contains("let val = input . input"));
	}

	#[test]
	fn async_result_return() {
		let result = parse_str(quote!(), syn::parse_quote! {
			async fn fallible(val: i32) -> Result<i32> { Ok(val) }
		});
		assert!(result.contains("type Out = i32"));
		assert!(result.contains("async_tool"));
	}

	#[test]
	fn async_result_out() {
		let result = parse_str(quote!(result_out), syn::parse_quote! {
			async fn fallible(val: i32) -> Result<i32> { Ok(val) }
		});
		assert!(result.contains("type Out = Result < i32 >"));
		assert!(result.contains("async_tool"));
	}

	// -----------------------------------------------------------------------
	// Async passthrough
	// -----------------------------------------------------------------------

	#[test]
	fn async_passthrough() {
		let result = parse_str(quote!(), syn::parse_quote! {
			async fn my_tool(cx: AsyncToolIn<i32>) -> i32 { *cx }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("async_tool"));
		// The param name should be forwarded directly.
		assert!(result.contains("| cx : "));
		assert!(!result.contains("let cx = input"));
	}

	// -----------------------------------------------------------------------
	// System tool tests
	// -----------------------------------------------------------------------

	#[test]
	fn system_basic() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(val: In<i32>) -> i32 { val * 2 }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("type Out = i32"));
		assert!(result.contains("system_tool"));
		assert!(result.contains("SystemToolIn"));
		assert!(result.contains("let val = In (__tool_in . input)"));
	}

	#[test]
	fn system_with_system_params() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(val: In<i32>, time: Res<Time>) -> f32 { val as f32 }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("system_tool"));
		assert!(result.contains("time : Res < Time >"));
		assert!(result.contains("let val = In (__tool_in . input)"));
	}

	#[test]
	fn system_result() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(val: In<i32>) -> Result<i32> { Ok(val) }
		});
		assert!(result.contains("type Out = i32"));
		assert!(result.contains("system_tool"));
	}

	#[test]
	fn system_unit_in_unit_out() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(val: In<()>) {}
		});
		assert!(result.contains("type In = ()"));
		assert!(result.contains("type Out = ()"));
		assert!(result.contains("system_tool"));
	}

	// -----------------------------------------------------------------------
	// System passthrough
	// -----------------------------------------------------------------------

	#[test]
	fn system_passthrough() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(cx: In<SystemToolIn<i32>>) -> Entity { cx.tool }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("system_tool"));
		// The param should be bound directly, no __tool_in indirection.
		assert!(result.contains("In (cx)"));
		assert!(!result.contains("__tool_in"));
	}

	#[test]
	fn system_passthrough_with_params() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(cx: In<SystemToolIn<i32>>, time: Res<Time>) -> f32 { 0.0 }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("system_tool"));
		assert!(result.contains("time : Res < Time >"));
		assert!(result.contains("In (cx)"));
	}
}
