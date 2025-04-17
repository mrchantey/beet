use beet_router::prelude::*;
use sweet::prelude::*;
use syn::FnArg;
use syn::Ident;
use syn::ItemFn;
use syn::Pat;
use syn::PatIdent;
use syn::PatTupleStruct;
use syn::Token;
use syn::Type;
use syn::TypePath;
use syn::parse_quote;
use syn::punctuated::Punctuated;

/// For a given [`FuncTokens::item_fn`] which is a valid [`axum::handler::Handler`],
/// create an equivelent client side function to call it.
///
///
/// ## Syntax Sugar
///
/// Destructuring any valid
///
#[derive(Default)]
pub struct FuncTokensToServerActions;

impl<T: AsRef<FuncTokens>> Pipeline<T, ItemFn> for FuncTokensToServerActions {
	fn apply(self, func_tokens: T) -> ItemFn {
		let func_tokens = func_tokens.as_ref();
		self.client_func(&func_tokens)
	}
}

impl FuncTokensToServerActions {
	fn client_func(&self, func_tokens: &FuncTokens) -> ItemFn {
		let parsed_inputs = Self::parse_inputs(func_tokens);
		let return_type = Self::parse_output(func_tokens);

		let fn_ident = &func_tokens.item_fn.sig.ident;
		let route_info = &func_tokens.route_info;

		let docs = func_tokens.item_fn.attrs.iter().filter_map(|attr| {
			if attr.path().is_ident("doc") {
				Some(attr.clone())
			} else {
				None
			}
		});

		match parsed_inputs {
			Some((fn_args, param_names)) => parse_quote! {
				#(#docs)*
				pub async fn #fn_ident(#fn_args) -> Result<#return_type, ServerActionError> {
					CallServerAction::request(#route_info, #param_names).await
				}
			},
			None => parse_quote! {
				#(#docs)*
				pub async fn #fn_ident() -> Result<#return_type, ServerActionError> {
					CallServerAction::request_no_data(#route_info).await
				}
			},
		}
	}

	/// Extractors that can be mapped to client side.
	/// This will be an extractor that either works with the url or the body,
	/// depending on the method.
	fn input_extractors(method: HttpMethod) -> Vec<Ident> {
		match method.has_body() {
			true => vec![parse_quote! { Json }],
			false => vec![parse_quote! { JsonQuery }],
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
		func: &FuncTokens,
	) -> Option<(Punctuated<FnArg, Token![,]>, Pat)> {
		// Find the first input that matches an extractor
		let Some(extractor_arg) =
			func.item_fn.sig.inputs.iter().find_map(|arg| {
				if let FnArg::Typed(pat_type) = arg {
					if let Type::Path(type_path) = &*pat_type.ty {
						if let Some(last) = type_path.path.segments.last() {
							if Self::input_extractors(func.route_info.method)
								.iter()
								.any(|extractor| last.ident == *extractor)
							{
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

	/// For given function output, return the output for the client function, uwrapping
	/// whatever was inside the extractor, if any.
	///
	/// ## Examples:
	/// |Input 																				| Output 														|
	/// |---																					|	---																|
	/// |`fn foo()` 																	| `()`															|
	/// |`fn foo() -> Bar`														| `()`															|
	/// |`fn foo() -> Result<Foo, Bar>`	 							| `()`															|
	/// |`fn foo() -> Json<u32>`											| `u32`															|
	/// |`fn foo() -> Result<Json<u32>>`							| `u32`															|
	/// |`fn foo() -> Result<Json<u32>, Bar>`					| `u32`															|
	/// |`fn foo() -> Json<Result<u32,u32>>`					| `Result<u32, u32>`								|
	/// |`fn foo() -> Result<Json<Result<u32,u32>>>`	| `Result<u32, u32>`								|
	fn parse_output(func: &FuncTokens) -> Type {
		/// Return type that can be mapped to client side.
		fn output_extractors() -> Vec<Ident> { vec![parse_quote! { Json }] }

		// Helper to check if a type is an output extractor (e.g., Json),
		// and if so return the first generic argument
		fn output_extractor_inner(ty: &Type) -> Option<&Type> {
			if let Type::Path(type_path) = ty {
				if let Some(seg) = type_path.path.segments.last() {
					if output_extractors()
						.iter()
						.any(|extractor| seg.ident == *extractor)
					{
						// Extract the inner type: Json<T>
						if let syn::PathArguments::AngleBracketed(args) =
							&seg.arguments
						{
							if let Some(syn::GenericArgument::Type(inner_ty)) =
								args.args.first()
							{
								return Some(inner_ty);
							}
						}
					}
				}
			}
			None
		}

		// If the type is a Result<T,..> return T
		fn result_inner(ty: &Type) -> Option<&Type> {
			if let Type::Path(type_path) = ty {
				if let Some(seg) = type_path.path.segments.last() {
					if seg.ident == "Result" {
						if let syn::PathArguments::AngleBracketed(args) =
							&seg.arguments
						{
							if let Some(syn::GenericArgument::Type(inner_ty)) =
								args.args.first()
							{
								return Some(inner_ty);
							}
						}
					}
				}
			}
			None
		}

		// Recursively unwrap output extractors and Results
		fn unwrap_type(ty: &Type) -> Type {
			// Json<T>
			if let Some(inner) = output_extractor_inner(ty) {
				return inner.clone();
			} else if let Some(inner) = result_inner(ty) {
				return unwrap_type(inner);
			} else {
				parse_quote! { () }
			}
		}

		match &func.item_fn.sig.output {
			syn::ReturnType::Default => parse_quote! { () },
			syn::ReturnType::Type(_, ty) => unwrap_type(ty),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_router::prelude::*;
	use proc_macro2::TokenStream;
	use quote::ToTokens;
	use sweet::prelude::*;
	use syn::parse_quote;

	#[test]
	fn parse_inputs() {
		fn assert(inputs: &str, expected: Option<(&str, &str)>) {
			let inputs: TokenStream = syn::parse_str(&inputs).unwrap();
			FuncTokensToServerActions::parse_inputs(
				&FuncTokens::simple_with_func("/add", syn::parse_quote! {
					fn post(#inputs){}
				}),
			)
			.xmap(|idents| {
				idents.map(|(a, b)| {
					(
						a.to_token_stream().to_string(),
						b.to_token_stream().to_string(),
					)
				})
			})
			.xmap(expect)
			.to_be(expected.map(|(a, b)| (a.to_string(), b.to_string())));
		}
		assert("", None);
		assert("foo: Bar", None);
		assert("foo: Json<u32>", Some(("foo : u32", "foo")));
		assert("Json(foo): Json<u32>", Some(("foo : u32", "foo")));
		assert("foo: Json<(u32)>", Some(("foo : (u32)", "foo")));
		assert("foo: Json<(u32,u32)>", Some(("foo : (u32 , u32)", "foo")));
		assert(
			"Json((foo,bar)): Json<(u32,u32)>",
			Some(("foo : u32 , bar : u32", "(foo , bar)")),
		);
	}
	#[test]
	fn parse_output() {
		fn assert(output: &str, expected: &str) {
			let output: TokenStream = syn::parse_str(output).unwrap();
			let func_tokens =
				FuncTokens::simple_with_func("/add", parse_quote! {
					fn post() -> #output {}
				});
			let ty = FuncTokensToServerActions::parse_output(&func_tokens);
			expect(ty.to_token_stream().to_string())
				.to_be(expected.to_string());
		}
		// No output
		let func_tokens = FuncTokens::simple_with_func("/foo", parse_quote! {
			fn post() {}
		});
		let ty = FuncTokensToServerActions::parse_output(&func_tokens);
		expect(ty.to_token_stream().to_string()).to_be("()");

		assert("Bar", "()");
		assert("Result<Foo, Bar>", "()");
		assert("Json<u32>", "u32");
		assert("Result<Json<u64>>", "u64");
		assert("Result<Json<i32>, Bar>", "i32");
		assert("Json<Result<u32 , i32>>", "Result < u32 , i32 >");
		assert("Result<Json<Result<u32 , u32>>>", "Result < u32 , u32 >");
	}




	#[test]
	fn get() {
		fn assert(
			func: syn::ItemFn,
			expected: syn::ItemFn,
		) -> (String, String) {
			let received = FuncTokens::simple_with_func("/add", func)
				.xpipe(FuncTokensToServerActions)
				.to_token_stream()
				.to_string();
			(received, expected.to_token_stream().to_string())
		}
		assert(parse_quote! {
			fn get() {
				1 + 1
			}
		},parse_quote! {
			pub async fn get() -> Result<(), ServerActionError> {
				CallServerAction::request_no_data(RouteInfo::new("/add", HttpMethod::Get)).await
			}
		}).xmap(|(received, expected)| {
			expect(received).to_be(expected);
		});

		assert(parse_quote! {
			fn get(JsonQuery((a,b)):JsonQuery<(i32,i64)>)->Result<Json<Result<u32>>> {
				1 + 1
			}
		},parse_quote! {
			pub async fn get(a:i32,b:i64) -> Result<Result<u32>, ServerActionError> {
				CallServerAction::request(RouteInfo::new("/add", HttpMethod::Get),(a,b)).await
			}
		}).xmap(|(received, expected)| {
			expect(received).to_be(expected);
		});
	}
}
