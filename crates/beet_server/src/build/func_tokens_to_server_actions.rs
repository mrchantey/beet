use beet_router::prelude::*;
use quote::format_ident;
use quote::quote;
use sweet::prelude::*;
use syn::Expr;
use syn::FnArg;
use syn::Ident;
use syn::ItemFn;
use syn::Member;
use syn::Pat;
use syn::PatType;
use syn::Token;
use syn::Type;
use syn::parse_quote;
use syn::punctuated::Punctuated;

/// Maps the [`FuncTokens::item_fn`] to a pair of
/// [`ItemFn`]s, one for the client and one for the server.
#[derive(Default)]
pub struct FuncTokensToServerActions;

impl<T: AsRef<FuncTokens>> Pipeline<T, (ItemFn, ItemFn)>
	for FuncTokensToServerActions
{
	fn apply(self, func_tokens: T) -> (ItemFn, ItemFn) {
		let func_tokens = func_tokens.as_ref();
		self.build(&func_tokens)
	}
}

impl FuncTokensToServerActions {
	pub fn build(&self, func_tokens: &FuncTokens) -> (ItemFn, ItemFn) {
		let client_func = self.client_func(func_tokens);
		let server_func = self.server_func(func_tokens);

		(client_func, server_func)
	}

	fn client_func(&self, func_tokens: &FuncTokens) -> ItemFn {
		let (inputs, param_names) = self.destructure_client_inputs(func_tokens);
		let output = &func_tokens.item_fn.sig.output;

		let fn_ident = &func_tokens.item_fn.sig.ident;
		let route_info = &func_tokens.route_info;

		let docs = func_tokens.item_fn.attrs.iter().filter_map(|attr| {
			if attr.path().is_ident("doc") {
				Some(attr.clone())
			} else {
				None
			}
		});

		let return_type = match output {
			syn::ReturnType::Default => quote! { () },
			syn::ReturnType::Type(_, ty) => quote! { #ty },
		};

		parse_quote! {
			#(#docs)*
			# [cfg(feature="client")]
			pub async fn #fn_ident(#inputs) -> Result<#return_type, ServerActionError> {
				CallServerAction::request(#route_info, #param_names).await
			}
		}
	}


	/// Map the original function inputs into a tuple of
	/// its first arg (destructured if tuple), and the param names for that arg.
	fn destructure_client_inputs(
		&self,
		func_tokens: &FuncTokens,
	) -> (Punctuated<FnArg, Token![,]>, Option<Expr>) {
		let pat_type = func_tokens
			.item_fn
			.sig
			.inputs
			.iter()
			.next()
			.map(|arg| match arg {
				FnArg::Typed(pat_type) => Some(pat_type),
				_ => None,
			})
			.flatten();

		match pat_type {
			Some(pat_ty) if let Type::Tuple(tuple) = &*pat_ty.ty => {
				let param_names = match &*pat_ty.pat {
					Pat::Tuple(pat_tuple) => pat_tuple
						.elems
						.iter()
						.map(|elem| match elem {
							Pat::Ident(ident) => ident.ident.clone(),
							_ => format_ident!("arg{}", 0u32),
						})
						.collect::<Vec<_>>(),
					_ => vec![],
				};
				let tuple_types = tuple.elems.iter().collect::<Vec<_>>();
				if param_names.len() == tuple_types.len() {
					let inputs = tuple_types
						.iter()
						.zip(param_names.iter())
						.map(|(ty, name)| -> FnArg {
							parse_quote! { #name: #ty }
						})
						.collect();
					(inputs, Some(parse_quote! { (#(#param_names),*) }))
				} else {
					let pat = pat_ty.pat.clone();
					(parse_quote!(#pat_ty), Some(parse_quote!(#pat)))
				}
			}
			Some(pat_ty) => {
				let pat = &pat_ty.pat;
				(parse_quote!(#pat_ty), Some(parse_quote!(#pat)))
			}
			None => {
				// No parameters
				(Punctuated::new(), None)
			}
		}
	}


	fn server_func(&self, func_tokens: &FuncTokens) -> ItemFn {
		let func_path = &func_tokens.func_path();
		let (inputs, param_names) = self.destructure_server_inputs(func_tokens);
		let maybe_await = if func_tokens.item_fn.sig.asyncness.is_some() {
			quote! { .await }
		} else {
			quote! {}
		};

		let fn_ident = &func_tokens.item_fn.sig.ident;
		// TODO handle if output is a result
		let output = &func_tokens.item_fn.sig.output;
		let return_type = match output {
			syn::ReturnType::Default => quote! { () },
			syn::ReturnType::Type(_, ty) => quote! { #ty },
		};


		parse_quote! {
			#[cfg(not(feature="client"))]
			async fn #fn_ident(#inputs) -> Json<#return_type> {
				Json(#func_path(#(#param_names),*) #maybe_await)
			}
		}
	}

	fn destructure_server_inputs(
		&self,
		func_tokens: &FuncTokens,
	) -> (Punctuated<FnArg, Token![,]>, Vec<Ident>) {
		let mut inputs = func_tokens.item_fn.sig.inputs.clone();
		// wrap the first arg in an extractor and destructure it
		if let Some(first_arg) = inputs.first_mut() {
			if let FnArg::Typed(pat_ty) = first_arg {
				let pat = &*pat_ty.pat;
				let ty = &*pat_ty.ty;
				if func_tokens.route_info.method.has_body() {
					*pat_ty.pat = parse_quote!(Json(#pat));
					*pat_ty.ty = parse_quote!(Json<#ty>)
				} else {
					*pat_ty.pat = parse_quote!(JsonQuery(#pat));
					*pat_ty.ty = parse_quote!(JsonQuery<#ty>)
				}
			}
		}

		/// recursively extracts the inner identifier from a `PatType`
		/// in the context of being used as a FnArg. Includes handling
		/// destructred tuples and structs.
		fn pat_ty_inner_idents(pat_ty: &PatType) -> Vec<&Ident> {
			match &*pat_ty.pat {
				Pat::Ident(pat_ident) => vec![&pat_ident.ident],
				Pat::Tuple(pat_tuple) => pat_tuple
					.elems
					.iter()
					.filter_map(|elem| {
						if let Pat::Ident(pat_ident) = elem {
							Some(&pat_ident.ident)
						} else {
							None
						}
					})
					.collect(),
				Pat::Struct(pat_struct) => pat_struct
					.fields
					.iter()
					.filter_map(|field| match &field.member {
						Member::Named(ident) => Some(ident),
						Member::Unnamed(_) => None,
					})
					.collect(),
				_ => vec![],
			}
		}

		let param_names = inputs
			.iter()
			.map(|arg| match arg {
				FnArg::Typed(pat_ty) => pat_ty_inner_idents(pat_ty),
				_ => vec![],
			})
			.flatten()
			.map(|ident| ident.clone())
			.collect::<Vec<_>>();

		(inputs, param_names)
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_router::prelude::*;
	use quote::ToTokens;
	use quote::quote;
	use sweet::prelude::*;
	use syn::parse_quote;

	fn build(func: syn::ItemFn) -> String {
		FuncTokens::simple_with_func("/add", func)
			.xpipe(FuncTokensToServerActions)
			.xmap(|(client, server)| {
				quote! {
					#client
					#server
				}
			})
			.to_token_stream()
			.to_string()
	}

	#[test]
	fn get_no_args() {
		expect(build(parse_quote! {
			fn get() {
				1 + 1
			}
		}))
		.to_be(
			quote! {
				#[cfg(feature = "client")]
				pub async fn get() -> Result<i32, ServerActionError> {
					CallServerAction::request(RouteInfo::new("/add", HttpMethod::Get), ).await
				}
				#[cfg(not(feature = "client"))]
				async fn get() -> Json<i32> {
					Json(file0::get())
				}
			}
			.to_string(),
		);
	}
	// #[test]
	// fn one_arg() {
	// 	expect(build(parse_quote! {
	// 		fn get(val:u32) -> i32 {
	// 			val + 1
	// 		}
	// 	}))
	// 	.to_be(
	// 		quote! {
	// 			#[cfg(feature = "client")]
	// 			pub async fn get(val:u32) -> Result<i32, ServerActionError> {
	// 				CallServerAction::request(RouteInfo::new("/add", HttpMethod::Get), ).await
	// 			}
	// 			#[cfg(not(feature = "client"))]
	// 			async fn get(Json(val):Json<i32>) -> Json<i32> {
	// 				Json(file0::get())
	// 			}
	// 		}
	// 		.to_string(),
	// 	);
	// }
}
