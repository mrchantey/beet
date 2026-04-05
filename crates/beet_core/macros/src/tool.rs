extern crate alloc;

use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;
use beet_core_shared::prelude::*;
use heck::ToSnakeCase;
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
			FnArg::Typed(pat_type) => match pat_type.pat.as_ref() {
				syn::Pat::Ident(pat_ident) => {
					params.push((pat_ident.ident.clone(), pat_type.ty.clone()));
				}
				syn::Pat::Wild(_) => {
					let discard_ident = syn::Ident::new(
						&alloc::format!("__tool_discard_{}", params.len()),
						proc_macro2::Span::call_site(),
					);
					params.push((discard_ident, pat_type.ty.clone()));
				}
				_ => {
					synbail!(
						&pat_type.pat,
						"tool parameters must be simple identifiers or `_`",
					);
				}
			},
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
		// Check for async passthrough: first param is AsyncToolIn<T> or AsyncToolIn.
		if let Some(first_ty) = first_param_type {
			if let Some(inner) =
				extract_wrapper_type_or_unit(first_ty, "AsyncToolIn")
			{
				return parse_async_passthrough(
					&item, result_out, &inner, &params,
				);
			}
		}
		parse_async_tool(&item, result_out, &params)
	} else if let Some(first_ty) = first_param_type {
		if let Some(in_inner) = extract_wrapper_type(first_ty, "In") {
			// First param is In<T>, determine which sub-case.
			if let Some(inner) =
				extract_wrapper_type_or_unit(in_inner, "SystemToolIn")
			{
				// In<SystemToolIn<T>> or In<SystemToolIn> → system passthrough
				parse_system_passthrough(&item, result_out, &inner, &params)
			} else if let Some(inner) =
				extract_wrapper_type_or_unit(in_inner, "FuncToolIn")
			{
				// In<FuncToolIn<T>> or In<FuncToolIn> → func passthrough
				parse_func_passthrough(&item, result_out, &inner, &params)
			} else {
				// In<T> → system tool
				parse_system_tool(&item, result_out, in_inner, &params)
			}
		} else if let Some(inner) =
			extract_wrapper_type_or_unit(first_ty, "SystemToolIn")
		{
			// SystemToolIn<T> or SystemToolIn (no In<>) → system passthrough
			parse_system_passthrough(&item, result_out, &inner, &params)
		} else if let Some(inner) =
			extract_wrapper_type_or_unit(first_ty, "FuncToolIn")
		{
			// FuncToolIn<T> or FuncToolIn (no In<>) → func passthrough
			parse_func_passthrough(&item, result_out, &inner, &params)
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
	let generics = &item.sig.generics;
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	let fn_attrs = &item.attrs;

	let (in_type, out_type) = compute_in_out(params, item, result_out)?;

	let destructure = make_destructure(params);
	let body_wrap = make_body_wrap(body, item, result_out);

	let tool_fn = tool_fn_name(fn_name);
	let turbofish = make_turbofish(generics);

	let require_tool = quote! {
		#beet_tool::prelude::Tool<#in_type, #out_type> = #beet_tool::prelude::func_tool(#tool_fn #turbofish)
	};

	let struct_def =
		make_struct_def(vis, fn_name, generics, fn_attrs, Some(require_tool));

	Ok(quote! {
		#struct_def

		fn #tool_fn #impl_generics (input: #beet_tool::prelude::FuncToolIn<#in_type>) -> Result<#out_type> #where_clause {
			#destructure
			#body_wrap
		}

		impl #impl_generics #beet_tool::prelude::IntoTool<#fn_name #ty_generics> for #fn_name #ty_generics #where_clause {
			type In = #in_type;
			type Out = #out_type;

			fn into_tool(self) -> #beet_tool::prelude::Tool<Self::In, Self::Out> {
				#beet_tool::prelude::func_tool(#tool_fn #turbofish)
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
	let generics = &item.sig.generics;
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	let fn_attrs = &item.attrs;

	let in_type = quote! { #inner_type };
	let out_type = compute_out_type(item, result_out);
	let body_wrap = make_body_wrap(body, item, result_out);

	let tool_fn = tool_fn_name(fn_name);
	let turbofish = make_turbofish(generics);

	let require_tool = quote! {
		#beet_tool::prelude::Tool<#in_type, #out_type> = #beet_tool::prelude::func_tool(#tool_fn #turbofish)
	};

	let struct_def =
		make_struct_def(vis, fn_name, generics, fn_attrs, Some(require_tool));

	Ok(quote! {
		#struct_def

		fn #tool_fn #impl_generics (#param_name: #beet_tool::prelude::FuncToolIn<#in_type>) -> Result<#out_type> #where_clause {
			#body_wrap
		}

		impl #impl_generics #beet_tool::prelude::IntoTool<#fn_name #ty_generics> for #fn_name #ty_generics #where_clause {
			type In = #in_type;
			type Out = #out_type;

			fn into_tool(self) -> #beet_tool::prelude::Tool<Self::In, Self::Out> {
				#beet_tool::prelude::func_tool(#tool_fn #turbofish)
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
	let generics = &item.sig.generics;
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	let fn_attrs = &item.attrs;

	let (in_type, out_type) = compute_in_out(params, item, result_out)?;

	let destructure = make_destructure(params);
	let body_wrap = make_body_wrap(body, item, result_out);

	let tool_fn = tool_fn_name(fn_name);
	let turbofish = make_turbofish(generics);

	let require_tool = quote! {
		#beet_tool::prelude::Tool<#in_type, #out_type> = #beet_tool::prelude::async_tool(#tool_fn #turbofish)
	};

	let struct_def =
		make_struct_def(vis, fn_name, generics, fn_attrs, Some(require_tool));

	Ok(quote! {
		#struct_def

		async fn #tool_fn #impl_generics (input: #beet_tool::prelude::AsyncToolIn<#in_type>) -> Result<#out_type> #where_clause {
			#destructure
			#body_wrap
		}

		impl #impl_generics #beet_tool::prelude::IntoTool<#fn_name #ty_generics> for #fn_name #ty_generics #where_clause {
			type In = #in_type;
			type Out = #out_type;

			fn into_tool(self) -> #beet_tool::prelude::Tool<Self::In, Self::Out> {
				#beet_tool::prelude::async_tool(#tool_fn #turbofish)
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
	let generics = &item.sig.generics;
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	let fn_attrs = &item.attrs;

	let in_type = quote! { #inner_type };
	let out_type = compute_out_type(item, result_out);
	let body_wrap = make_body_wrap(body, item, result_out);

	let tool_fn = tool_fn_name(fn_name);
	let turbofish = make_turbofish(generics);

	let require_tool = quote! {
		#beet_tool::prelude::Tool<#in_type, #out_type> = #beet_tool::prelude::async_tool(#tool_fn #turbofish)
	};

	let struct_def =
		make_struct_def(vis, fn_name, generics, fn_attrs, Some(require_tool));

	Ok(quote! {
		#struct_def

		async fn #tool_fn #impl_generics (#param_name: #beet_tool::prelude::AsyncToolIn<#in_type>) -> Result<#out_type> #where_clause {
			#body_wrap
		}

		impl #impl_generics #beet_tool::prelude::IntoTool<#fn_name #ty_generics> for #fn_name #ty_generics #where_clause {
			type In = #in_type;
			type Out = #out_type;

			fn into_tool(self) -> #beet_tool::prelude::Tool<Self::In, Self::Out> {
				#beet_tool::prelude::async_tool(#tool_fn #turbofish)
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
	let generics = &item.sig.generics;
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	let fn_attrs = &item.attrs;

	let first_param_name = &params[0].0;
	let in_type = quote! { #in_inner };
	let out_type = compute_out_type(item, result_out);
	let body_wrap = make_body_wrap(body, item, result_out);

	// Collect remaining system params (everything after the first).
	let system_params = collect_system_params(item, 1);

	let tool_fn = tool_fn_name(fn_name);
	let turbofish = make_turbofish(generics);

	let require_tool = quote! {
		#beet_tool::prelude::Tool<#in_type, #out_type> = #beet_tool::prelude::system_tool(#tool_fn #turbofish)
	};

	let struct_def =
		make_struct_def(vis, fn_name, generics, fn_attrs, Some(require_tool));

	Ok(quote! {
		#struct_def

		fn #tool_fn #impl_generics (
			In(__tool_in): In<#beet_tool::prelude::SystemToolIn<#in_type>>
			#(, #system_params)*
		) -> Result<#out_type> #where_clause {
			let #first_param_name = In(__tool_in.input);
			#body_wrap
		}

		impl #impl_generics #beet_tool::prelude::IntoTool<#fn_name #ty_generics> for #fn_name #ty_generics #where_clause {
			type In = #in_type;
			type Out = #out_type;

			fn into_tool(self) -> #beet_tool::prelude::Tool<Self::In, Self::Out> {
				#beet_tool::prelude::system_tool(#tool_fn #turbofish)
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
	let generics = &item.sig.generics;
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	let fn_attrs = &item.attrs;

	let first_param_name = &params[0].0;
	let in_type = quote! { #inner_type };
	let out_type = compute_out_type(item, result_out);
	let body_wrap = make_body_wrap(body, item, result_out);

	let system_params = collect_system_params(item, 1);

	let tool_fn = tool_fn_name(fn_name);
	let turbofish = make_turbofish(generics);

	let require_tool = quote! {
		#beet_tool::prelude::Tool<#in_type, #out_type> = #beet_tool::prelude::system_tool(#tool_fn #turbofish)
	};

	let struct_def =
		make_struct_def(vis, fn_name, generics, fn_attrs, Some(require_tool));

	Ok(quote! {
		#struct_def

		fn #tool_fn #impl_generics (
			In(#first_param_name): In<#beet_tool::prelude::SystemToolIn<#in_type>>
			#(, #system_params)*
		) -> Result<#out_type> #where_clause {
			#body_wrap
		}

		impl #impl_generics #beet_tool::prelude::IntoTool<#fn_name #ty_generics> for #fn_name #ty_generics #where_clause {
			type In = #in_type;
			type Out = #out_type;

			fn into_tool(self) -> #beet_tool::prelude::Tool<Self::In, Self::Out> {
				#beet_tool::prelude::system_tool(#tool_fn #turbofish)
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

/// Generate a struct definition, forwarding function attributes to the
/// struct and optionally adding a `#[require(Tool<...>)]` attribute
/// when the derives include `Component`.
fn make_struct_def(
	vis: &syn::Visibility,
	fn_name: &syn::Ident,
	generics: &syn::Generics,
	fn_attrs: &[syn::Attribute],
	require_tool: Option<TokenStream>,
) -> TokenStream {
	let has_component = has_derive(fn_attrs, "Component");
	let require_attr = if has_component {
		if let Some(expr) = require_tool {
			quote! { #[require(#expr)] }
		} else {
			quote! {}
		}
	} else {
		quote! {}
	};

	let type_params: Vec<&syn::Ident> =
		generics.type_params().map(|tp| &tp.ident).collect();

	if type_params.is_empty() {
		quote! {
			#(#fn_attrs)*
			#require_attr
			#[allow(non_camel_case_types)]
			#vis struct #fn_name;
		}
	} else {
		let (impl_generics, _, where_clause) = generics.split_for_impl();
		let has_reflect = has_derive(fn_attrs, "Reflect");
		let phantom = if type_params.len() == 1 {
			let tp = type_params[0];
			if has_reflect {
				quote! { fn() -> #tp }
			} else {
				quote! { #tp }
			}
		} else {
			if has_reflect {
				quote! { fn() -> (#(#type_params),*) }
			} else {
				quote! { (#(#type_params),*) }
			}
		};
		let reflect_ignore = if has_reflect {
			quote! { #[reflect(ignore)] }
		} else {
			quote! {}
		};
		quote! {
			#(#fn_attrs)*
			#require_attr
			#[allow(non_camel_case_types)]
			#vis struct #fn_name #impl_generics (#reflect_ignore ::core::marker::PhantomData<#phantom>) #where_clause;
		}
	}
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

/// Extract inner type for wrappers with default generic unit:
/// `Wrapper<T>` -> `T`, `Wrapper` -> `()`.
fn extract_wrapper_type_or_unit(ty: &Type, name: &str) -> Option<Type> {
	if let Some(inner) = extract_wrapper_type(ty, name) {
		Some(inner.clone())
	} else if is_wrapper_without_args(ty, name) {
		Some(syn::parse_quote! { () })
	} else {
		None
	}
}

/// Whether a type path is `Wrapper` with no generic args.
fn is_wrapper_without_args(ty: &Type, name: &str) -> bool {
	if let Type::Path(type_path) = ty {
		if let Some(segment) = type_path.path.segments.last() {
			return segment.ident == name
				&& matches!(segment.arguments, syn::PathArguments::None);
		}
	}
	false
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


// ---------------------------------------------------------------------------
// Tool-specific helpers
// ---------------------------------------------------------------------------

/// Convert a function/struct name to a snake_case standalone function
/// name with a `_tool` suffix.
fn tool_fn_name(fn_name: &syn::Ident) -> syn::Ident {
	let snake = fn_name.to_string().to_snake_case();
	let name = alloc::format!("{}_tool", snake);
	syn::Ident::new(&name, fn_name.span())
}

/// Check whether any `#[derive(...)]` attribute contains `name`.
fn has_derive(attrs: &[syn::Attribute], name: &str) -> bool {
	attrs.iter().any(|attr| {
		if attr.path().is_ident("derive") {
			attr.parse_args_with(
				syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
			)
			.map(|meta_list| meta_list.iter().any(|path| path.is_ident(name)))
			.unwrap_or(false)
		} else {
			false
		}
	})
}

/// Generate turbofish syntax for generic type parameters,
/// or empty tokens when there are none.
fn make_turbofish(generics: &syn::Generics) -> TokenStream {
	let type_params: Vec<&syn::Ident> =
		generics.type_params().map(|tp| &tp.ident).collect();
	if type_params.is_empty() {
		quote! {}
	} else {
		quote! { ::<#(#type_params),*> }
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use alloc::string::String;
	use alloc::string::ToString;
	use quote::quote;

	// -----------------------------------------------------------------------
	// Generics propagation
	// -----------------------------------------------------------------------

	#[test]
	fn async_passthrough_with_generics() {
		let result = parse_str(quote!(), syn::parse_quote! {
			async fn my_tool<T>(input: AsyncToolIn<()>) -> ()
			where
				T: Send + Sync,
			{}
		});
		assert!(result.contains("struct my_tool"));
		assert!(result.contains("PhantomData"));
		assert!(result.contains("where T : Send + Sync"));
		assert!(result.contains("impl < T >"));
		assert!(
			result.contains("IntoTool < my_tool < T > > for my_tool < T >")
		);
	}

	#[test]
	fn func_tool_with_generics() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool<T>(val: i32) -> i32
			where
				T: Clone,
			{ val }
		});
		assert!(result.contains("PhantomData"));
		assert!(result.contains("where T : Clone"));
		assert!(result.contains("impl < T >"));
		assert!(
			result.contains("IntoTool < my_tool < T > > for my_tool < T >")
		);
	}

	#[test]
	fn multi_generic_struct() {
		let result = parse_str(quote!(), syn::parse_quote! {
			async fn my_tool<A, B>(input: AsyncToolIn<()>) -> ()
			where
				A: Send,
				B: Sync,
			{}
		});
		assert!(result.contains("PhantomData < (A , B) >"));
	}

	#[test]
	fn generic_with_reflect_uses_fn_phantom() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			#[reflect(Component)]
			async fn my_tool<T>(input: AsyncToolIn<()>) -> ()
			where
				T: Send + Sync,
			{}
		});
		assert!(result.contains("reflect (ignore)"));
		assert!(result.contains("fn () ->"));
	}

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
		// standalone function
		assert!(result.contains("fn my_tool_tool"));
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
		// standalone function
		assert!(result.contains("fn add_tool"));
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
		assert!(result.contains("fn double_tool"));
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
		// standalone function has the param name
		assert!(result.contains("fn my_tool_tool"));
		assert!(!result.contains("let cx = input"));
	}

	#[test]
	fn func_passthrough_in_func_tool_in() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(cx: In<FuncToolIn<String>>) -> String { cx.input.clone() }
		});
		assert!(result.contains("type In = String"));
		assert!(result.contains("func_tool"));
		assert!(result.contains("fn my_tool_tool"));
	}

	#[test]
	fn func_passthrough_default_unit() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(cx: FuncToolIn) -> i32 { 1 }
		});
		assert!(result.contains("type In = ()"));
		assert!(result.contains("func_tool"));
		assert!(result.contains("fn my_tool_tool"));
		assert!(!result.contains("let cx = input"));
	}

	#[test]
	fn func_passthrough_in_func_tool_in_default_unit() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(cx: In<FuncToolIn>) -> i32 { 1 }
		});
		assert!(result.contains("type In = ()"));
		assert!(result.contains("func_tool"));
		assert!(result.contains("fn my_tool_tool"));
		assert!(!result.contains("let cx = input"));
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
		// standalone async function instead of inline closure
		assert!(result.contains("async fn my_tool_tool"));
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
		// standalone async function
		assert!(result.contains("async fn my_tool_tool"));
		assert!(!result.contains("let cx = input"));
	}

	#[test]
	fn async_passthrough_default_unit() {
		let result = parse_str(quote!(), syn::parse_quote! {
			async fn my_tool(cx: AsyncToolIn) -> i32 { 42 }
		});
		assert!(result.contains("type In = ()"));
		assert!(result.contains("async_tool"));
		assert!(result.contains("async fn my_tool_tool"));
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
			fn my_tool(cx: In<SystemToolIn<i32>>) -> Entity { cx.caller }
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

	#[test]
	fn system_passthrough_default_unit() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(cx: In<SystemToolIn>) -> Entity { cx.caller }
		});
		assert!(result.contains("type In = ()"));
		assert!(result.contains("system_tool"));
		assert!(result.contains("In (cx)"));
		assert!(!result.contains("__tool_in"));
	}

	#[test]
	fn system_passthrough_direct() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(cx: SystemToolIn<i32>) -> Entity { cx.caller }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("system_tool"));
		assert!(result.contains("In (cx)"));
		assert!(!result.contains("__tool_in"));
	}

	#[test]
	fn system_passthrough_direct_default_unit() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(cx: SystemToolIn) -> Entity { cx.caller }
		});
		assert!(result.contains("type In = ()"));
		assert!(result.contains("system_tool"));
		assert!(result.contains("In (cx)"));
		assert!(!result.contains("__tool_in"));
	}

	// -----------------------------------------------------------------------
	// has_derive helper
	// -----------------------------------------------------------------------

	#[test]
	fn detect_component_derive() {
		let item: syn::ItemFn = syn::parse_quote! {
			#[derive(Debug, Clone, Component, Reflect)]
			fn Add() {}
		};
		assert!(!item.attrs.is_empty());
		assert!(has_derive(&item.attrs, "Component"));
	}

	#[test]
	fn detect_no_component_derive() {
		let item: syn::ItemFn = syn::parse_quote! {
			#[derive(Debug, Clone)]
			fn Add() {}
		};
		assert!(!item.attrs.is_empty());
		assert!(!has_derive(&item.attrs, "Component"));
	}

	#[test]
	fn detect_no_derives_at_all() {
		let item: syn::ItemFn = syn::parse_quote! {
			fn Add() {}
		};
		assert!(item.attrs.is_empty());
		assert!(!has_derive(&item.attrs, "Component"));
	}

	// -----------------------------------------------------------------------
	// Component derive + #[require] generation
	// -----------------------------------------------------------------------

	#[test]
	fn component_derive_adds_require() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Debug, Clone, Component, Reflect)]
			fn Add(a: i32, b: i32) -> i32 { a + b }
		});
		assert!(
			result.contains("derive (Debug , Clone , Component , Reflect)")
		);
		assert!(result.contains("# [require"));
		assert!(result.contains("Tool <"));
		assert!(result.contains("func_tool (add_tool"));
		assert!(result.contains("struct Add"));
		assert!(result.contains("fn add_tool"));
	}

	#[test]
	fn no_component_derive_no_require() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Debug, Clone)]
			fn my_tool(a: i32) -> i32 { a }
		});
		assert!(result.contains("derive (Debug , Clone)"));
		assert!(!result.contains("# [require"));
		assert!(result.contains("fn my_tool_tool"));
	}

	#[test]
	fn no_derives_no_require() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_tool(a: i32) -> i32 { a }
		});
		assert!(!result.contains("# [require"));
		assert!(result.contains("fn my_tool_tool"));
	}

	#[test]
	fn async_component_derive_adds_require() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Debug, Component, Reflect)]
			async fn HelpHandler(cx: AsyncToolIn<Request>) -> Result<Outcome<Response, Request>> {
				todo!()
			}
		});
		assert!(result.contains("derive (Debug , Component , Reflect)"));
		assert!(result.contains("# [require"));
		assert!(result.contains("async_tool (help_handler_tool"));
		assert!(result.contains("struct HelpHandler"));
		assert!(result.contains("async fn help_handler_tool"));
	}

	#[test]
	fn system_component_derive_adds_require() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Debug, Component, Reflect)]
			fn Increment(val: In<()>, query: Query<&Name>) -> Result<i64> {
				Ok(1)
			}
		});
		assert!(result.contains("derive (Debug , Component , Reflect)"));
		assert!(result.contains("# [require"));
		assert!(result.contains("system_tool (increment_tool"));
		assert!(result.contains("struct Increment"));
		assert!(result.contains("fn increment_tool"));
	}

	#[test]
	fn doc_attrs_forwarded() {
		let result = parse_str(quote!(), syn::parse_quote! {
			/// Does stuff.
			fn my_tool(a: i32) -> i32 { a }
		});
		assert!(result.contains("doc"));
		assert!(result.contains("struct my_tool"));
	}

	#[test]
	fn into_tool_uses_standalone_fn() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn add(a: i32, b: i32) -> i32 { a + b }
		});
		// IntoTool impl should reference the standalone function, not an inline closure
		assert!(result.contains("func_tool (add_tool)"));
	}

	#[test]
	fn generic_component_with_turbofish() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Debug, Component)]
			fn MyTool<T>(val: i32) -> i32
			where T: Clone,
			{ val }
		});
		assert!(result.contains("# [require"));
		assert!(result.contains("func_tool (my_tool_tool :: < T >"));
		assert!(result.contains("fn my_tool_tool < T >"));
	}
}
