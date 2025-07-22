use beet_core::prelude::*;
use bevy::prelude::*;
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
use crate::prelude::*;

/// For a given [`RouteFileMethod::item_fn`],
/// create an equivelent client side function to call it.
///
#[derive(Default)]
pub struct ParseClientAction;

impl ParseClientAction {
	pub fn client_func(&self, RouteFileMethod{route_info,item}: &RouteFileMethod) -> ItemFn {
		let parsed_inputs = Self::parse_inputs(item);
		let (return_type, error_type) = Self::parse_output(item);

		let fn_ident = &item.sig.ident;
		let route_info = route_info.self_token_stream();

		let docs = item.attrs.iter().filter_map(|attr| {
			if attr.path().is_ident("doc") {
				Some(attr.clone())
			} else {
				None
			}
		});


		match parsed_inputs {
			Some((fn_args, param_names)) => parse_quote! {
				#(#docs)*
				pub async fn #fn_ident(#fn_args) -> ServerActionResult<#return_type, #error_type> {
					CallServerAction::request(#route_info, #param_names).await
				}
			},
			None => parse_quote! {
				#(#docs)*
				pub async fn #fn_ident() -> ServerActionResult<#return_type, #error_type> {
					CallServerAction::request_no_data(#route_info).await
				}
			},
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
	/// |`fn foo(a: Json<i32>)` 								| `Some([a: i32], a)`								|
	/// |`fn foo(Json(a): Json<i32>)` 					| `Some([a: i32], a)`								|
	/// |`fn foo(args: Json<(i32,i32)>)` 				| `Some([args: (i32, i32)], args])`	|
	/// |`fn foo(Json((a,b)): Json<(i32,i32)>)` | `Some([a: i32, b: i32], (a, b))`	|
	fn parse_inputs(
		func: &ItemFn,
	) -> Option<(Punctuated<FnArg, Token![,]>, Pat)> {
		// Get the type of the first argument if it is an In<T>
		let Some(extractor_arg) = func.sig.inputs.iter().next().and_then(|arg| {
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
		}) else {
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

	/// For given function output, return the output and error types for the client function, unwrapping
	/// whatever was inside the extractor, if any.
	///
	/// ## Examples:
	/// |Input                                                                                  | Output                                 |
	/// |---                                                                                    | ---                                    |
	/// |`fn foo()`                                                                            | `((), ())`                             |
	/// |`fn foo() -> Bar`                                                                     | `(Bar, ())`                            |
	/// |`fn foo() -> ActionResult<Foo, Bar>`                                                  | `(Foo, Bar)`                           |
	/// |`fn foo() -> ActionResult<Json<Foo>, Json<Bar>>`                                      | `(Foo, Bar)`                           |
	/// |`fn foo() -> ActionResult<Json<Result<u32, u32>>, Json<ActionError<u32>>>`            | `(Result<u32, u32>, ActionError<u32>)` |
	/// |`fn foo() -> ActionError<Bar>`                                                        | `((), Bar)`                            |
	/// |`fn foo() -> ActionError<Json<Bar>>`                                                  | `((), Bar)`                            |
	/// |`fn foo() -> Json<u32>`                                                               | `(u32, ())`                            |
	/// |`fn foo() -> Result<Json<u64>>`                                                       | `(u64, ())`                            |
	/// |`fn foo() -> Result<Json<i32>, Bar>`                                                  | `(i32, Bar)`                           |
	/// |`fn foo() -> Result<Json<Result<u32, u32>>>`                                          | `(Result<u32, u32>, ())`               |
	fn parse_output(func: &ItemFn) -> (Type, Type) {
		// recursively unwraps the extractor type,
		// ie Json<ActionError<Json<u32>>> becomes u32
		fn is_action_error(ty: &Type) -> bool {
			if let Type::Path(TypePath { path, .. }) = ty {
				if let Some(seg) = path.segments.last() {
					if seg.ident == "ActionError" {
						return true;
					}
				}
			}
			false
		}


		fn unwrap_extractors(ty: &Type) -> &Type {
			if let Type::Path(TypePath { path, .. }) = ty {
				if let Some(seg) = path.segments.last() {
					if seg.ident == "ActionError" {
						if let syn::PathArguments::AngleBracketed(args) =
							&seg.arguments
						{
							if let Some(syn::GenericArgument::Type(inner_ty)) =
								args.args.first()
							{
								return unwrap_extractors(inner_ty);
							}
						}
					}
				}
			}
			ty
		}

		/// Unwraps a `Result<T,E>` or `ActionResult<T,E>`
		fn unwrap_result_like(ty: &Type) -> Option<(Type, Type)> {
			if let Type::Path(TypePath { path, .. }) = ty {
				if let Some(seg) = path.segments.last() {
					if seg.ident == "Result" || seg.ident == "ActionResult" {
						if let syn::PathArguments::AngleBracketed(args) =
							&seg.arguments
						{
							let mut args_iter =
								args.args.iter().filter_map(|a| match a {
									syn::GenericArgument::Type(t) => Some(t),
									_ => None,
								});
							let t = args_iter
								.next()
								.map(unwrap_extractors)
								.cloned()
								.unwrap_or_else(|| parse_quote! { () });
							let e = args_iter
								.next()
								.map(unwrap_extractors)
								.cloned()
								// the default E type of ActionResult is String
								.unwrap_or_else(|| parse_quote! { String });
							return Some((t, e));
						}
					}
				}
			}
			None
		}

		match &func.sig.output {
			syn::ReturnType::Default => {
				(parse_quote! { () }, parse_quote! { () })
			}
			syn::ReturnType::Type(_, ty) => {
				if let Some((t, e)) = unwrap_result_like(ty) {
					(t, e)
				} else if is_action_error(ty) {
					(parse_quote! { () }, unwrap_extractors(ty).clone())
				} else {
					(unwrap_extractors(ty).clone(), parse_quote! { () })
				}
			}
		}
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
		fn parse(inputs: &str) -> Option<(String,String)> {
			let inputs: TokenStream = syn::parse_str(&inputs).unwrap();
			ParseClientAction::parse_inputs(
				&syn::parse_quote! {
					fn post(#inputs){}
				},
			)
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
		parse("foo: In<u32>").unwrap().xpect().to_be(("foo : u32".into(), "foo".into()));
		parse("In(foo): In<u32>").unwrap().xpect().to_be(("foo : u32".into(), "foo".into()));
		parse("foo: In<(u32)>").unwrap().xpect().to_be(("foo : (u32)".into(), "foo".into()));
		parse("foo: In<(u32,u32)>").unwrap().xpect().to_be(("foo : (u32 , u32)".into(), "foo".into()));
		parse("In((foo,bar)): In<(u32,u32)>").unwrap().xpect().to_be(("foo : u32 , bar : u32".into(), "(foo , bar)".into()));

	}
	#[test]
	fn parse_output() {
		fn parse(output: &str)-> (String,String) {
			let output: TokenStream = syn::parse_str(output).unwrap();
			let (ty, err) = ParseClientAction::parse_output(&parse_quote! {
				fn post() -> #output{}
			});
			(
				ty.to_token_stream().to_string(),
				err.to_token_stream().to_string(),
			)
		}
		// No output
		let (ty, err) = ParseClientAction::parse_output(&parse_quote! {
			fn post(){}
		});
		expect((
			ty.to_token_stream().to_string(),
			err.to_token_stream().to_string(),
		))
		.to_be(("()".to_string(), "()".to_string()));

		parse("Bar").xpect().to_be(("Bar".into(), "()".into()));
		parse("Foo<Bar>").xpect().to_be(("Foo < Bar >".into(), "()".into()));
		parse("Result<Foo, Bar>").xpect().to_be(("Foo".into(), "Bar".into()));
		parse("Result<Foo<Bar>>").xpect().to_be(("Foo < Bar >".into(), "String".into()));
		parse("Result<Foo<Bar>,Bazz>").xpect().to_be(("Foo < Bar >".into(), "Bazz".into()));
		parse("Result<Result<Foo, Bar>>").xpect().to_be(("Result < Foo , Bar >".into(), "String".into()));
		parse("ActionResult<Foo>").xpect().to_be(("Foo".into(), "String".into()));
		parse("ActionResult<Foo,Bar>").xpect().to_be(("Foo".into(), "Bar".into()));
		parse("Result<Foo, ActionError<Bar>>").xpect().to_be(("Foo".into(), "Bar".into()));
		parse("ActionResult<Result<Foo , Bar>, ActionError<Bazz>>").xpect().to_be(("Result < Foo , Bar >".into(), "Bazz".into()));
		parse("ActionError<Bar>").xpect().to_be(("()".into(), "Bar".into()));
	}




	#[test]
	fn get() {
		fn assert(
			func: syn::ItemFn,
			expected: syn::ItemFn,
		) -> (String, String) {
			let received = ParseClientAction
				.client_func(&RouteFileMethod::new_with("/add",&func))
				.to_token_stream()
				.to_string();
			(received, expected.to_token_stream().to_string())
		}
		assert(parse_quote! {
			fn get() {
				1 + 1
			}
		},parse_quote! {
			pub async fn get() -> ServerActionResult<(), ()> {
				CallServerAction::request_no_data(RouteInfo { path: RoutePath(std::path::PathBuf::from("/add")), method: HttpMethod::Get }).await
			}
		}).xmap(|(received, expected)| {
			expect(received).to_be(expected);
		});

		assert(parse_quote! {
			fn get(In((a,b)):In<(i32,i64)>)->Result<Result<u32>> {
				Ok(Ok(1 + 1))
			}
		},parse_quote! {
			pub async fn get(a: i32, b: i64) -> ServerActionResult<Result<u32>, String> {
				CallServerAction::request(RouteInfo { 
					path: RoutePath(std::path::PathBuf::from("/add")), 
					method: HttpMethod::Get 
				}, (a, b)).await
			}
		}).xmap(|(received, expected)| {
			expect(received).to_be(expected);
		});
	}
}
