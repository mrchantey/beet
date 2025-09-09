use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use quote::quote;
use syn::FnArg;
use syn::ItemFn;
use syn::Pat;
use syn::PatIdent;
use syn::PatTupleStruct;
use syn::Token;
use syn::Type;
use syn::TypePath;
use syn::parse_quote;
use syn::punctuated::Punctuated;

/// For a given [`RouteFileMethod::item_fn`],
/// create an equivelent client side function to call it.
///
#[derive(Default)]
pub struct ParseClientAction;

impl ParseClientAction {
	pub fn client_func(&self, action: &RouteFileMethod) -> ItemFn {
		let parsed_inputs = Self::parse_inputs(&action.item);
		// let (return_type, error_type) = Self::parse_output(item);

		let fn_ident = &action.item.sig.ident;
		let fn_return_type = match &action.item.sig.output {
			syn::ReturnType::Default => parse_quote! { () },
			syn::ReturnType::Type(_, ty) => ty.clone(),
		};
		let method = &action.route_info.method.self_token_stream();
		let path = &action.route_info.path.to_string_lossy();
		// let route_info = route_info.self_token_stream();

		let docs = &action
			.item
			.attrs
			.iter()
			.filter_map(|attr| {
				if attr.path().is_ident("doc") {
					Some(attr.clone())
				} else {
					None
				}
			})
			.collect::<Vec<_>>();

		let dot_send = match action.returns_result() {
			true => quote! { .send_fallible() },
			false => quote! { .send() },
		};

		let (fn_args, body) = match parsed_inputs {
			Some((fn_args, param_names)) => {
				(fn_args, quote! { .with_body(#param_names) })
			}
			None => (Punctuated::new(), Default::default()),
		};

		parse_quote! {
			#(#docs)*
			#[allow(unused)]
			pub async fn #fn_ident(#fn_args) -> Result<#fn_return_type> {
				ServerActionRequest::new(#method, #path)
					#body
					#dot_send
					.await
			}
		}
	}
	/// For given function inputs, return the inputs for the client function
	/// as well as the 'restructured' version to be pased to the server.
	/// If there are no inputs to be passed, this will be [`None`].
	///
	/// ## Examples:
	/// |Input 																	| Output 														|
	/// |---																		|	---																|
	/// |`fn foo()` 														| `None`														|
	/// |`fn foo(some_extractor: SomeExtractor)`| `None`														|
	/// |`fn foo(a: In<i32>)` 									| `Some([a: i32], a)`								|
	/// |`fn foo(In(a): In<i32>)` 							| `Some([a: i32], a)`								|
	/// |`fn foo(args: In<(i32,i32)>)` 					| `Some([args: (i32, i32)], args])`	|
	/// |`fn foo(In((a,b)): In<(i32,i32)>)` 		| `Some([a: i32, b: i32], (a, b))`	|
	fn parse_inputs(
		func: &ItemFn,
	) -> Option<(Punctuated<FnArg, Token![,]>, Pat)> {
		// Get the type of the first argument if it is an In<T>
		let Some(extractor_arg) =
			func.sig.inputs.iter().next().and_then(|arg| {
				if let FnArg::Typed(pat_type) = arg {
					if let Type::Path(type_path) = &*pat_type.ty {
						if let Some(last) = type_path.path.segments.last() {
							if last.ident == "In" {
								return Some(pat_type);
							}
						}
					}
				}
				None
			})
		else {
			return None;
		};

		// Extract the pattern and the inner type
		match &*extractor_arg.pat {
			// ie a: Json<i32>
			Pat::Ident(PatIdent { ident, .. }) => {
				// Type is Json<T>
				if let Type::Path(TypePath { path, .. }) = &*extractor_arg.ty {
					if let Some(seg) = path.segments.last() {
						if let syn::PathArguments::AngleBracketed(args) =
							&seg.arguments
						{
							if let Some(syn::GenericArgument::Type(inner_ty)) =
								args.args.first()
							{
								// Pattern is just the ident
								return Some((
									{
										let mut punct = Punctuated::new();
										punct.push(
											syn::parse_quote! { #ident: #inner_ty },
										);
										punct
									},
									syn::parse_quote! { #ident },
								));
							}
						}
					}
				}
				return None;
			}
			// ie Json(a): Json<i32>
			// or Json((a,b)): Json<(i32,i32)>
			Pat::TupleStruct(PatTupleStruct { elems, .. }) => {
				// Pattern is Json(a) or Json((a, b))
				if let Type::Path(TypePath { path, .. }) = &*extractor_arg.ty {
					if let Some(seg) = path.segments.last() {
						if let syn::PathArguments::AngleBracketed(args) =
							&seg.arguments
						{
							if let Some(syn::GenericArgument::Type(inner_ty)) =
								args.args.first()
							{
								if let Type::Tuple(tuple) = inner_ty {
									// Handle Json((a, b)): Json<(u32, u32)>
									if elems.len() == 1 {
										if let Pat::Tuple(inner_tuple) =
											&elems[0]
										{
											let mut fn_args = Punctuated::new();
											let mut pat_idents = Vec::new();
											for (pat_elem, ty_elem) in
												inner_tuple
													.elems
													.iter()
													.zip(tuple.elems.iter())
											{
												if let Pat::Ident(PatIdent {
													ident,
													..
												}) = pat_elem
												{
													fn_args.push(
														syn::parse_quote! { #ident: #ty_elem },
													);
													pat_idents
														.push(ident.clone());
												}
											}
											let tuple_pat = syn::parse_quote! { (#(#pat_idents),*) };
											return Some((fn_args, tuple_pat));
										}
									}
									// Fallback: e.g. Json(a, b): Json<(u32, u32)> (not typical, but for completeness)
									let mut fn_args = Punctuated::new();
									let mut pat_idents = Vec::new();
									for (pat_elem, ty_elem) in
										elems.iter().zip(tuple.elems.iter())
									{
										if let Pat::Ident(PatIdent {
											ident,
											..
										}) = pat_elem
										{
											fn_args.push(
												syn::parse_quote! { #ident: #ty_elem },
											);
											pat_idents.push(ident.clone());
										}
									}
									let tuple_pat = syn::parse_quote! { (#(#pat_idents),*) };
									return Some((fn_args, tuple_pat));
								} else {
									// e.g. Json(a): Json<i32>
									if let Some(Pat::Ident(PatIdent {
										ident,
										..
									})) = elems.first()
									{
										let mut fn_args = Punctuated::new();
										fn_args.push(
											syn::parse_quote! { #ident: #inner_ty },
										);
										return Some((
											fn_args,
											syn::parse_quote! { #ident },
										));
									}
								}
							}
						}
					}
				}
				return None;
			}
			_ => return None,
		};
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::utils::PipelineTarget;
	use proc_macro2::TokenStream;
	use quote::ToTokens;
	use sweet::prelude::*;
	use syn::parse_quote;

	#[test]
	fn parse_inputs() {
		fn parse(inputs: &str) -> Option<(String, String)> {
			let inputs: TokenStream = syn::parse_str(&inputs).unwrap();
			ParseClientAction::parse_inputs(&syn::parse_quote! {
				fn post(#inputs){}
			})
			.xmap(|idents| {
				idents.map(|(a, b)| {
					(
						a.to_token_stream().to_string(),
						b.to_token_stream().to_string(),
					)
				})
			})
		}
		parse("").xpect().to_be_none();
		parse("foo: Bar").xpect().to_be_none();
		parse("foo: In<u32>")
			.unwrap()
			.xpect_eq(("foo : u32".into(), "foo".into()));
		parse("In(foo): In<u32>")
			.unwrap()
			.xpect_eq(("foo : u32".into(), "foo".into()));
		parse("foo: In<(u32)>")
			.unwrap()
			.xpect_eq(("foo : (u32)".into(), "foo".into()));
		parse("foo: In<(u32,u32)>")
			.unwrap()
			.xpect_eq(("foo : (u32 , u32)".into(), "foo".into()));
		parse("In((foo,bar)): In<(u32,u32)>")
			.unwrap()
			.xpect_eq(("foo : u32 , bar : u32".into(), "(foo , bar)".into()));
	}

	#[test]
	fn get() {
		ParseClientAction
			.client_func(&RouteFileMethod::new_with("/add", &parse_quote! {
				fn get() {
					1 + 1
				}
			}))
			.to_token_stream()
			.xpect()
			.to_be_snapshot();
	}
	#[test]
	fn get_with_result() {
		ParseClientAction
			.client_func(&RouteFileMethod::new_with("/add", &parse_quote! {
				fn get(In((a,b)): In<(i32,i64)>) -> Result<u32, String> {
					Ok(Ok(1 + 1))
				}
			}))
			.to_token_stream()
			.xpect()
			.to_be_snapshot();
	}
}
