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

fn parse(attr: TokenStream, item: ItemFn) -> syn::Result<TokenStream> {
	let attrs = AttributeMap::parse(attr)?;
	attrs.assert_types(&[], &["result_out"])?;
	let result_out = attrs.contains_key("result_out");

	let beet_tool = pkg_ext::internal_or_beet("beet_tool");

	let vis = &item.vis;
	let fn_name = &item.sig.ident;
	let body = &item.block;

	// Collect parameter names and types.
	let mut param_names: Vec<syn::Ident> = Vec::new();
	let mut param_types: Vec<Box<Type>> = Vec::new();

	for arg in &item.sig.inputs {
		match arg {
			FnArg::Typed(pat_type) => {
				// Extract the ident from the pattern.
				if let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
					param_names.push(pat_ident.ident.clone());
					param_types.push(pat_type.ty.clone());
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

	// Determine the raw return type (what the user wrote after `->`).
	let raw_return_type: Option<&Type> = match &item.sig.output {
		ReturnType::Default => None,
		ReturnType::Type(_, ty) => Some(ty.as_ref()),
	};

	// Determine whether the return is a Result, and compute In/Out types.
	let returns_result = raw_return_type
		.map(|ty| is_result_type(ty))
		.unwrap_or(false);

	let out_type: TokenStream = if let Some(ty) = raw_return_type {
		if returns_result && !result_out {
			// Unwrap Result<T> to just T.
			if let Some(inner) = extract_result_inner(ty) {
				quote! { #inner }
			} else {
				// Result with no generic args, fallback to ().
				quote! { () }
			}
		} else {
			quote! { #ty }
		}
	} else {
		quote! { () }
	};

	let in_type: TokenStream = match param_types.len() {
		0 => quote! { () },
		1 => {
			let ty = &param_types[0];
			quote! { #ty }
		}
		_ => {
			let types = &param_types;
			quote! { (#(#types),*) }
		}
	};

	// Input destructuring inside the closure.
	let destructure: TokenStream = match param_names.len() {
		0 => quote! { let _ = input.input; },
		1 => {
			let name = &param_names[0];
			quote! { let #name = input.input; }
		}
		_ => {
			let names = &param_names;
			quote! { let (#(#names),*) = input.input; }
		}
	};

	// Body evaluation: if the function returns Result (and not result_out),
	// append `?` to propagate the error.
	let body_wrap: TokenStream = if returns_result && !result_out {
		// return the body directly, it already returns a result
		quote! {
		#[allow(unused_braces)]
		#body }
	} else {
		// wrap the body in an Ok
		quote! { Ok(
			#[allow(unused_braces)]
			#body
			)
		}
	};

	Ok(quote! {
		#[allow(non_camel_case_types)]
		#vis struct #fn_name;

		impl #beet_tool::prelude::IntoToolHandler<#fn_name> for #fn_name {
			type In = #in_type;
			type Out = #out_type;

			fn into_tool_handler(self) -> #beet_tool::prelude::ToolHandler<Self::In, Self::Out> {
				#beet_tool::prelude::func_tool(|input: #beet_tool::prelude::FuncToolIn<#in_type>| {
					#destructure
					#body_wrap
				})
			}
		}
	})
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
	if let Type::Path(type_path) = ty {
		if let Some(segment) = type_path.path.segments.last() {
			if segment.ident == "Result" {
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


#[cfg(test)]
mod test {
	use super::*;
	use quote::quote;

	fn parse_str(attr: TokenStream, item: syn::ItemFn) -> String {
		parse(attr, item).unwrap().to_string()
	}

	#[test]
	fn no_args_no_return() {
		let result = parse_str(quote!(), syn::parse_quote! { fn my_tool() {} });
		assert!(result.contains("struct my_tool"));
		assert!(result.contains("type In = ()"));
		assert!(result.contains("type Out = ()"));
		assert!(result.contains("let _ = input . input"));
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
		// Should NOT have a `?` since the return is not Result.
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
	}

	#[test]
	fn result_return() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn fallible(a: i32, b: i32) -> Result<i32> { Ok(a + b) }
		});
		// Out should be i32, not Result<i32>.
		assert!(result.contains("type Out = i32"));
	}

	#[test]
	fn result_out_flag() {
		let result = parse_str(quote!(result_out), syn::parse_quote! {
			fn fallible(a: i32) -> Result<i32> { Ok(a) }
		});
		// Out should preserve the full Result type.
		assert!(result.contains("type Out = Result < i32 >"));
		// Body should NOT use `?`.
		assert!(!result.contains("} ?"));
	}

	#[test]
	fn visibility_preserved() {
		let result =
			parse_str(quote!(), syn::parse_quote! { pub fn public_tool() {} });
		assert!(result.contains("pub struct public_tool"));
	}
}
