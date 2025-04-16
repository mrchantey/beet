use crate::prelude::*;
use quote::format_ident;
use quote::quote;
use sweet::prelude::*;
use syn::Block;
use syn::FnArg;
use syn::ItemFn;
use syn::Pat;
use syn::ReturnType;
use syn::Type;
use syn::parse_quote;
use syn::punctuated::Punctuated;

/// Maps the [`FuncTokens::item_fn`] to a pair of
/// [`ItemFn`]s, one for the client and one for the server.
pub struct FuncTokensToServerActions;

impl Pipeline<FuncTokens, Option<(ItemFn, ItemFn)>>
	for FuncTokensToServerActions
{
	fn apply(self, func_tokens: FuncTokens) -> Option<(ItemFn, ItemFn)> {
		let Some(item_fn) = func_tokens.item_fn else {
			return None;
		};
		Some(self.build(&func_tokens.route_info, &item_fn))
	}
}

impl FuncTokensToServerActions {
	pub fn build(
		&self,
		route_info: &RouteInfo,
		item_fn: &ItemFn,
	) -> (syn::ItemFn, syn::ItemFn) {
		let fn_name = &item_fn.sig.ident;
		let return_type = &item_fn.sig.output;
		let is_async = item_fn.sig.asyncness.is_some();

		// Extract the function body
		let function_body = &item_fn.block;

		// Extract parameters
		let (args_tuple_pat, args_type, client_params, server_extractors) =
			self.extract_parameters(&item_fn.sig.inputs);

		// Build client version using method name directly
		let client_out = self.build_client_version(
			route_info,
			fn_name,
			&client_params,
			return_type,
		);

		// Build server version using is_bodyless to determine extractor type
		let server_out = self.build_server_version(
			route_info,
			fn_name,
			is_async,
			args_tuple_pat,
			args_type,
			function_body,
			&server_extractors,
			return_type,
		);

		(client_out, server_out)
	}

	fn extract_parameters(
		&self,
		inputs: &Punctuated<FnArg, syn::token::Comma>,
	) -> (Option<Pat>, Option<Type>, Vec<FnArg>, Vec<FnArg>) {
		let mut args_tuple_pat = None;
		let mut args_type = None;
		let mut client_params = Vec::new();
		let mut server_extractors = Vec::new();

		for param in inputs {
			match param {
				FnArg::Receiver(_) => {
					// Skip self parameters
					continue;
				}
				FnArg::Typed(pat_type) => {
					let pat = &*pat_type.pat;
					let ty = &*pat_type.ty;

					// Check if this is the arguments tuple
					if let Pat::Tuple(tuple) = pat {
						args_tuple_pat = Some(Pat::Tuple(tuple.clone()));
						args_type = Some(ty.clone());

						// Extract tuple elements for client parameters
						let mut index = 0;
						for elem in &tuple.elems {
							if let Pat::Ident(ident) = elem {
								// Named tuple element
								let param_name = &ident.ident;
								let param_type =
									self.extract_tuple_element_type(ty, index);
								client_params.push(
									parse_quote! { #param_name: #param_type },
								);
							} else {
								// Unnamed tuple element
								let param_name = format_ident!("args{}", index);
								let param_type =
									self.extract_tuple_element_type(ty, index);
								client_params.push(
									parse_quote! { #param_name: #param_type },
								);
							}
							index += 1;
						}
					} else if let Pat::Ident(ident) = pat {
						// Check if this might be an args parameter
						if ident.ident.to_string().starts_with("args") {
							args_tuple_pat = Some(pat.clone());
							args_type = Some(ty.clone());

							// Handle tuple type for unnamed args
							if let Type::Tuple(tuple_type) = &*ty {
								for (index, elem) in
									tuple_type.elems.iter().enumerate()
								{
									let param_name =
										format_ident!("args{}", index);
									client_params.push(
										parse_quote! { #param_name: #elem },
									);
								}
							}
						} else {
							// This is likely an extractor
							server_extractors.push(parse_quote! { #pat: #ty });
						}
					}
				}
			}
		}

		(args_tuple_pat, args_type, client_params, server_extractors)
	}

	fn extract_tuple_element_type(&self, ty: &Type, index: usize) -> Type {
		if let Type::Tuple(tuple_ty) = ty {
			if index < tuple_ty.elems.len() {
				return tuple_ty.elems[index].clone();
			}
		}
		parse_quote! { () } // Fallback
	}

	fn build_client_version(
		&self,
		route_info: &RouteInfo,
		fn_name: &syn::Ident,
		client_params: &[FnArg],
		return_type: &ReturnType,
	) -> syn::ItemFn {
		let return_inner_type = match return_type {
			ReturnType::Type(_, ty) => ty.clone(),
			_ => parse_quote!(()),
		};

		// Build parameter collection
		let param_names: Vec<_> = client_params
			.iter()
			.enumerate()
			.map(|(i, _)| format_ident!("args{}", i))
			.collect();

		parse_quote! {
			#[cfg(feature="client")]
			async fn #fn_name(#(#client_params),*) -> Result<#return_inner_type, ServerActionError> {
				CallServerAction::request(#route_info, (#(#param_names),*)).await
			}
		}
	}

	fn build_server_version(
		&self,
		route_info: &RouteInfo,
		fn_name: &syn::Ident,
		is_async: bool,
		args_tuple_pat: Option<Pat>,
		args_type: Option<Type>,
		function_body: &Block,
		server_extractors: &[FnArg],
		return_type: &ReturnType,
	) -> syn::ItemFn {
		let method_has_body = route_info.method.has_body();
		let extractor_type: syn::TypePath = if method_has_body {
			parse_quote!(Json)
		} else {
			parse_quote!(JsonQuery)
		};

		let return_inner_type = match return_type {
			ReturnType::Type(_, ty) => ty.clone(),
			_ => parse_quote!(()),
		};

		let args_param: FnArg = if let Some(ty) = args_type {
			if let Some(pat) = args_tuple_pat {
				parse_quote! { #extractor_type(#pat): #ty }
			} else {
				parse_quote! { #extractor_type(args): #ty }
			}
		} else {
			if method_has_body {
				// is this valid?
				parse_quote! { #extractor_type(args): () }
			} else {
				// For bodyless requests with no parameters, use JsonQuery<()>
				parse_quote! { #extractor_type(args): #extractor_type < () > }
			}
		};

		let function_execution = if is_async {
			quote! {
				Json({
					#function_body.await
				})
			}
		} else {
			quote! {
				Json({
					#function_body
				})
			}
		};

		parse_quote! {
			#[cfg(not(feature="client"))]
			async fn #fn_name(#args_param, #(#server_extractors),*) -> Json<#return_inner_type> {
				#function_execution
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::str::FromStr;

	use crate::prelude::*;
	use quote::ToTokens;
	use quote::quote;
	use sweet::prelude::*;
	use syn::parse_quote;

	fn build(func: syn::ItemFn, path: &str) -> (syn::ItemFn, syn::ItemFn) {
		let mut func_tokens = FuncTokens::simple(path, syn::parse_quote!({}));
		func_tokens.route_info.method =
			HttpMethod::from_str(&func.sig.ident.to_string()).unwrap();
		func_tokens.item_fn = Some(func);
		func_tokens.xpipe(FuncTokensToServerActions).unwrap()
	}

	#[test]
	fn test_basic_get_function() {
		// Input function
		let item_fn: syn::ItemFn = parse_quote! {
			fn get(args: (i32, i32), some_extractor: SomeAxumExtractor) -> i32 {
				args.0 + args.1
			}
		};

		let (client_fn, server_fn) = build(item_fn, "/add");

		// Check client function
		expect(client_fn.sig.ident.to_string()).to_be("get");
		expect(
			client_fn
				.attrs
				.iter()
				.any(|attr| attr.path().is_ident("cfg")),
		)
		.to_be(true);
		expect(client_fn.sig.asyncness).to_be_some();

		// Verify parameters
		expect(client_fn.sig.inputs.len()).to_be(2);

		// Check server function
		expect(server_fn.sig.ident.to_string()).to_be("get");
		expect(
			server_fn
				.attrs
				.iter()
				.any(|attr| attr.path().is_ident("cfg")),
		)
		.to_be(true);
		expect(server_fn.sig.asyncness).to_be_some();

		// Verify extractor is JsonQuery for GET
		let first_param = server_fn.sig.inputs.first().unwrap();
		let param_str = quote!(#first_param).to_token_stream().to_string();
		expect(param_str)
			.to_contain(&quote!(JsonQuery).to_token_stream().to_string());
	}

	#[test]
	fn test_basic_post_function() {
		// Input function
		let item_fn: syn::ItemFn = parse_quote! {
			fn post(args: (i32, i32), some_extractor: SomeAxumExtractor) -> i32 {
				args.0 + args.1
			}
		};

		let (client_fn, server_fn) = build(item_fn, "/add");

		// Check client function has post method
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::request(
				RouteInfo::new("/add", HttpMethod::Post),
				(args0, args1)
			))
			.to_token_stream()
			.to_string(),
		);

		// Check server function uses Json extractor instead of JsonQuery
		let first_param = server_fn.sig.inputs.first().unwrap();
		let param_str = quote!(#first_param).to_token_stream().to_string();
		expect(&param_str)
			.to_contain(&quote!(Json).to_token_stream().to_string());
		expect(param_str)
			.not()
			.to_contain(&quote!(JsonQuery).to_token_stream().to_string());
	}

	#[test]
	fn test_named_tuple_parameters() {
		// Input function with named tuple parameters
		let item_fn: syn::ItemFn = parse_quote! {
			fn get((a, b): (i32, i32)) -> i32 {
				a + b
			}
		};

		let (client_fn, _) = build(item_fn, "/add");

		// Verify client function parameters are named correctly
		let params_str = quote!(#client_fn).to_token_stream().to_string();
		expect(&params_str)
			.to_contain(&quote!(a : i32).to_token_stream().to_string());
		expect(&params_str)
			.to_contain(&quote!(b : i32).to_token_stream().to_string());
	}

	#[test]
	fn test_unnamed_tuple_parameters() {
		// Input function with unnamed tuple
		let item_fn: syn::ItemFn = parse_quote! {
			fn get(args: (i32, i32)) -> i32 {
				args.0 + args.1
			}
		};

		let (client_fn, _) = build(item_fn, "/add");

		// Verify client function parameters use generic args names
		let params_str = quote!(#client_fn).to_token_stream().to_string();
		expect(&params_str)
			.to_contain(&quote!(args0 : i32).to_token_stream().to_string());
		expect(&params_str)
			.to_contain(&quote!(args1 : i32).to_token_stream().to_string());
	}

	#[test]
	fn test_async_function() {
		// Input async function
		let item_fn: syn::ItemFn = parse_quote! {
			async fn get(args: (i32, i32)) -> i32 {
				fetch_some_data().await + args.0 + args.1
			}
		};

		let (_, server_fn) = build(item_fn, "/add");

		// Verify server function properly awaits the body
		let server_str = quote!(#server_fn).to_token_stream().to_string();
		expect(server_str)
			.to_contain(&quote!(. await).to_token_stream().to_string());
	}

	#[test]
	fn test_non_async_function() {
		// Input non-async function
		let item_fn: syn::ItemFn = parse_quote! {
			fn get(args: (i32, i32)) -> i32 {
				args.0 + args.1
			}
		};

		let (_, server_fn) = build(item_fn, "/add");

		// Verify server function doesn't await the body
		let server_str = quote!(#server_fn).to_token_stream().to_string();

		// The function itself is async but doesn't await the body
		let function_body = &server_fn.block;
		expect(server_fn.sig.asyncness).to_be_some();
		expect(server_str).not().to_contain(
			&quote!(#function_body.await).to_token_stream().to_string(),
		);
	}

	#[test]
	fn test_return_type_handling() {
		// Test with complex return type
		let item_fn: syn::ItemFn = parse_quote! {
			fn get(args: (i32, i32)) -> Result<Vec<String>, Error> {
				Ok(vec![format!("Sum: {}", args.0 + args.1)])
			}
		};

		let (client_fn, server_fn) = build(item_fn, "/complex");

		// Check client return type
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(Result<Result<Vec<String>, Error>, ServerActionError>)
				.to_token_stream()
				.to_string(),
		);

		// Check server return type - use the expected type without spaces
		let server_fn_str = quote!(#server_fn).to_token_stream().to_string();
		// Remove whitespace for more reliable comparison
		let server_fn_no_space = server_fn_str.replace(" ", "");
		expect(server_fn_no_space).to_contain("Json<Result<Vec<String>,Error>");
	}

	#[test]
	fn test_multiple_extractors() {
		// Test with multiple extractors
		let item_fn: syn::ItemFn = parse_quote! {
			fn get(args: (i32, i32), state: State<AppState>, headers: Headers) -> i32 {
				args.0 + args.1
			}
		};

		let (_, server_fn) = build(item_fn, "/add");

		// Check that all extractors are preserved in server function
		let server_str = quote!(#server_fn).to_token_stream().to_string();
		expect(&server_str).to_contain(
			&quote!(state : State < AppState >)
				.to_token_stream()
				.to_string(),
		);
		expect(&server_str).to_contain(
			&quote!(headers : Headers).to_token_stream().to_string(),
		);
	}

	#[test]
	fn test_different_extractor_types() {
		// Test with different extractor types besides tuples
		let item_fn: syn::ItemFn = parse_quote! {
			fn get(query: Query<Params>, auth: Auth) -> String {
				format!("Hello, {}", auth.name)
			}
		};

		let (client_fn, server_fn) = build(item_fn, "/greet");

		// Client function should have zero parameters since there's no args tuple
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::request(
				RouteInfo::new("/greet", HttpMethod::Get),
				()
			))
			.to_token_stream()
			.to_string(),
		);

		// Server function should preserve the extractors
		let server_str = quote!(#server_fn).to_token_stream().to_string();
		expect(&server_str).to_contain(
			&quote!(JsonQuery(args) : JsonQuery < () >)
				.to_token_stream()
				.to_string(),
		);
		expect(&server_str).to_contain(
			&quote!(query : Query < Params >)
				.to_token_stream()
				.to_string(),
		);
		expect(&server_str)
			.to_contain(&quote!(auth : Auth).to_token_stream().to_string());
	}

	#[test]
	fn test_case_insensitive_method_names() {
		// Test with capitalized method names
		let item_fn: syn::ItemFn = parse_quote! {
			fn GET(args: (i32, i32)) -> i32 {
				args.0 + args.1
			}
		};

		let (client_fn, server_fn) = build(item_fn, "/add");

		// Check that method name case is preserved but HTTP method is correct
		expect(client_fn.sig.ident.to_string()).to_be("GET");

		// Verify JsonQuery is used (GET method)
		let server_str = quote!(#server_fn).to_token_stream().to_string();
		expect(server_str)
			.to_contain(&quote!(JsonQuery).to_token_stream().to_string());
	}

	#[test]
	fn test_no_parameters() {
		// Test with no parameters
		let item_fn: syn::ItemFn = parse_quote! {
				fn get() -> String {
						"Hello World".to_string()
				}
		};

		let (client_fn, server_fn) = build(item_fn, "/hello");

		// Client function should have no parameters
		let client_params = client_fn.sig.inputs.len();
		expect(client_params).to_be(0);

		// Verify call has empty tuple
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::request(
				RouteInfo::new("/hello", HttpMethod::Get),
				()
			))
			.to_token_stream()
			.to_string(),
		);

		// Server function should have JsonQuery with empty tuple
		let server_str = quote!(#server_fn).to_token_stream().to_string();
		expect(server_str).to_contain(
			&quote!(JsonQuery (args) : JsonQuery < () >)
				.to_token_stream()
				.to_string(),
		);
	}

	#[test]
	fn test_unit_return_type() {
		// Test with unit return type
		let item_fn: syn::ItemFn = parse_quote! {
				fn post(args: (i32, i32)) {
						println!("Sum: {}", args.0 + args.1);
				}
		};

		let (client_fn, server_fn) = build(item_fn, "/log");

		// Check return types
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(Result < () , ServerActionError >)
				.to_token_stream()
				.to_string(),
		);

		let server_str = quote!(#server_fn).to_token_stream().to_string();
		expect(server_str)
			.to_contain(&quote!(Json<()>).to_token_stream().to_string());
	}

	#[test]
	fn test_self_parameter() {
		// Test with self parameter which should be ignored
		let item_fn: syn::ItemFn = parse_quote! {
				fn get(&self, args: (i32, i32)) -> i32 {
						args.0 + args.1
				}
		};

		let (client_fn, server_fn) = build(item_fn, "/add");

		// Verify self parameter is removed from client function
		let client_params = client_fn.sig.inputs.len();
		expect(client_params).to_be(2); // Just the two args, no self

		// Verify self is not in server function
		let server_str = quote!(#server_fn).to_token_stream().to_string();
		expect(server_str)
			.not()
			.to_contain(&quote!(&self).to_token_stream().to_string());
	}

	#[test]
	fn test_put_function() {
		// Input function
		let item_fn: syn::ItemFn = parse_quote! {
			fn put(args: (i32, i32)) -> i32 {
				args.0 + args.1
			}
		};

		let (client_fn, server_fn) = build(item_fn, "/update");

		// Check client function has PUT method
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::request(
				RouteInfo::new("/update", HttpMethod::Put),
				(args0, args1)
			))
			.to_token_stream()
			.to_string(),
		);

		// Check server function uses Json extractor for PUT (has body)
		let first_param = server_fn.sig.inputs.first().unwrap();
		let param_str = quote!(#first_param).to_token_stream().to_string();
		expect(&param_str)
			.to_contain(&quote!(Json).to_token_stream().to_string());
		expect(param_str)
			.not()
			.to_contain(&quote!(JsonQuery).to_token_stream().to_string());
	}

	#[test]
	fn test_delete_function() {
		// Input function
		let item_fn: syn::ItemFn = parse_quote! {
			fn delete(args: (i32, i32)) -> i32 {
				args.0 + args.1
			}
		};

		let (client_fn, server_fn) = build(item_fn, "/delete");

		// Check client function has DELETE method
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::request(
				RouteInfo::new("/delete", HttpMethod::Delete),
				(args0, args1)
			))
			.to_token_stream()
			.to_string(),
		);

		// DELETE currently treated as a bodyless method
		let first_param = server_fn.sig.inputs.first().unwrap();
		let param_str = quote!(#first_param).to_token_stream().to_string();
		expect(&param_str)
			.to_contain(&quote!(JsonQuery).to_token_stream().to_string());
	}

	#[test]
	fn test_patch_function() {
		// Input function
		let item_fn: syn::ItemFn = parse_quote! {
			fn patch(args: (i32, i32)) -> i32 {
				args.0 + args.1
			}
		};

		let (client_fn, server_fn) = build(item_fn, "/patch");

		// Check client function has PATCH method
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::request(
				RouteInfo::new("/patch", HttpMethod::Patch),
				(args0, args1)
			))
			.to_token_stream()
			.to_string(),
		);

		// Check server function uses Json extractor for PATCH (has body)
		let first_param = server_fn.sig.inputs.first().unwrap();
		let param_str = quote!(#first_param).to_token_stream().to_string();
		expect(&param_str)
			.to_contain(&quote!(Json).to_token_stream().to_string());
		expect(param_str)
			.not()
			.to_contain(&quote!(JsonQuery).to_token_stream().to_string());
	}

	#[test]
	fn test_options_function() {
		// Input function
		let item_fn: syn::ItemFn = parse_quote! {
			fn options() -> String {
				"Options supported".to_string()
			}
		};

		let (client_fn, server_fn) = build(item_fn, "/options");

		// Check client function has OPTIONS method
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::request(
				RouteInfo::new("/options", HttpMethod::Options),
				()
			))
			.to_token_stream()
			.to_string(),
		);

		// Check server function uses JsonQuery extractor for OPTIONS (bodyless)
		let first_param = server_fn.sig.inputs.first().unwrap();
		let param_str = quote!(#first_param).to_token_stream().to_string();
		expect(&param_str)
			.to_contain(&quote!(JsonQuery).to_token_stream().to_string());
	}

	#[test]
	fn test_head_function() {
		// Input function
		let item_fn: syn::ItemFn = parse_quote! {
			fn head(args: (i32, i32)) -> i32 {
				args.0 + args.1
			}
		};

		let (client_fn, server_fn) = build(item_fn, "/head");

		// Check client function has HEAD method
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::request(
				RouteInfo::new("/head", HttpMethod::Head),
				(args0, args1)
			))
			.to_token_stream()
			.to_string(),
		);

		// Check server function uses JsonQuery extractor for HEAD (bodyless)
		let first_param = server_fn.sig.inputs.first().unwrap();
		let param_str = quote!(#first_param).to_token_stream().to_string();
		expect(&param_str)
			.to_contain(&quote!(JsonQuery).to_token_stream().to_string());
	}
}
