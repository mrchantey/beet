use quote::format_ident;
use quote::quote;
use syn::Block;
use syn::FnArg;
use syn::Ident;
use syn::ItemFn;
use syn::Pat;
use syn::ReturnType;
use syn::Type;
use syn::parse_quote;

pub struct ServerActionBuilder {
	item_fn: ItemFn,
	route_path: String,
}

impl ServerActionBuilder {
	pub fn new(item_fn: ItemFn, route_path: String) -> Self {
		Self {
			item_fn,
			route_path,
		}
	}

	pub fn build(&self) -> (syn::ItemFn, syn::ItemFn) {
		let fn_name = &self.item_fn.sig.ident;
		let return_type = &self.item_fn.sig.output;
		let is_async = self.item_fn.sig.asyncness.is_some();
		let method_name = fn_name.to_string().to_lowercase();

		// Determine the HTTP method based on function name
		// Classify methods into bodyless and methods with body
		let is_bodyless = matches!(
			method_name.as_str(),
			"get" | "head" | "options" | "connect" | "trace" | "delete"
		);

		let http_method: Ident = match method_name.as_str() {
			"get" => parse_quote!(get),
			"post" => parse_quote!(post),
			"put" => parse_quote!(put),
			"delete" => parse_quote!(delete),
			"patch" => parse_quote!(patch),
			"head" => parse_quote!(head),
			"options" => parse_quote!(options),
			"connect" => parse_quote!(connect),
			"trace" => parse_quote!(trace),
			_ => parse_quote!(post), // Default fallback to POST for unknown methods
		};

		// Extract the function body
		let function_body = &self.item_fn.block;

		// Extract parameters
		let (args_tuple_pat, args_type, client_params, server_extractors) =
			self.extract_parameters();

		// Build client version
		let client_out = self.build_client_version(
			fn_name,
			&http_method,
			&client_params,
			return_type,
		);

		// Build server version - use is_bodyless instead of is_get
		let server_out = self.build_server_version(
			fn_name,
			is_async,
			is_bodyless,
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
	) -> (Option<Pat>, Option<Type>, Vec<syn::FnArg>, Vec<syn::FnArg>) {
		let mut args_tuple_pat = None;
		let mut args_type = None;
		let mut client_params = Vec::new();
		let mut server_extractors = Vec::new();

		for param in &self.item_fn.sig.inputs {
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
		fn_name: &syn::Ident,
		http_method: &syn::Ident,
		client_params: &[syn::FnArg],
		return_type: &ReturnType,
	) -> syn::ItemFn {
		let route_path = &self.route_path;
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
				CallServerAction::#http_method(#route_path, (#(#param_names),*)).await
			}
		}
	}

	fn build_server_version(
		&self,
		fn_name: &syn::Ident,
		is_async: bool,
		is_bodyless: bool,
		args_tuple_pat: Option<Pat>,
		args_type: Option<Type>,
		function_body: &Block,
		server_extractors: &[syn::FnArg],
		return_type: &ReturnType,
	) -> syn::ItemFn {
		let extractor_type: syn::TypePath = if is_bodyless {
			parse_quote!(JsonQuery)
		} else {
			parse_quote!(Json)
		};

		let return_inner_type = match return_type {
			ReturnType::Type(_, ty) => ty.clone(),
			_ => parse_quote!(()),
		};

		let args_param: syn::FnArg = if let Some(ty) = args_type {
			if let Some(pat) = args_tuple_pat {
				parse_quote! { #extractor_type(#pat): #ty }
			} else {
				parse_quote! { #extractor_type(args): #ty }
			}
		} else {
			if is_bodyless {
				// For bodyless requests with no parameters, use JsonQuery<()>
				parse_quote! { #extractor_type(args): #extractor_type < () > }
			} else {
				parse_quote! { #extractor_type(args): () }
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
	use crate::prelude::*;
	use quote::ToTokens;
	use quote::quote;
	use sweet::prelude::*;
	use syn::parse_quote;

	#[test]
	fn test_basic_get_function() {
		// Input function
		let item_fn: syn::ItemFn = parse_quote! {
			fn get(args: (i32, i32), some_extractor: SomeAxumExtractor) -> i32 {
				args.0 + args.1
			}
		};

		let builder = ServerActionBuilder::new(item_fn, "/add".to_string());
		let (client_fn, server_fn) = builder.build();

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

		let builder = ServerActionBuilder::new(item_fn, "/add".to_string());
		let (client_fn, server_fn) = builder.build();

		// Check client function has post method
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::post).to_token_stream().to_string(),
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

		let builder = ServerActionBuilder::new(item_fn, "/add".to_string());
		let (client_fn, _) = builder.build();

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

		let builder = ServerActionBuilder::new(item_fn, "/add".to_string());
		let (client_fn, _) = builder.build();

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

		let builder = ServerActionBuilder::new(item_fn, "/add".to_string());
		let (_, server_fn) = builder.build();

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

		let builder = ServerActionBuilder::new(item_fn, "/add".to_string());
		let (_, server_fn) = builder.build();

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

		let builder = ServerActionBuilder::new(item_fn, "/complex".to_string());
		let (client_fn, server_fn) = builder.build();

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

		let builder = ServerActionBuilder::new(item_fn, "/add".to_string());
		let (_, server_fn) = builder.build();

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

		let builder = ServerActionBuilder::new(item_fn, "/greet".to_string());
		let (client_fn, server_fn) = builder.build();

		// Client function should have zero parameters since there's no args tuple
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::get("/greet", ()))
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

		let builder = ServerActionBuilder::new(item_fn, "/add".to_string());
		let (client_fn, server_fn) = builder.build();

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

		let builder = ServerActionBuilder::new(item_fn, "/hello".to_string());
		let (client_fn, server_fn) = builder.build();

		// Client function should have no parameters
		let client_params = client_fn.sig.inputs.len();
		expect(client_params).to_be(0);

		// Verify call has empty tuple
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::get("/hello", ()))
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

		let builder = ServerActionBuilder::new(item_fn, "/log".to_string());
		let (client_fn, server_fn) = builder.build();

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

		let builder = ServerActionBuilder::new(item_fn, "/add".to_string());
		let (client_fn, server_fn) = builder.build();

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

		let builder = ServerActionBuilder::new(item_fn, "/update".to_string());
		let (client_fn, server_fn) = builder.build();

		// Check client function has PUT method
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::put).to_token_stream().to_string(),
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

		let builder = ServerActionBuilder::new(item_fn, "/delete".to_string());
		let (client_fn, server_fn) = builder.build();

		// Check client function has DELETE method
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::delete)
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

		let builder = ServerActionBuilder::new(item_fn, "/patch".to_string());
		let (client_fn, server_fn) = builder.build();

		// Check client function has PATCH method
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::patch)
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

		let builder = ServerActionBuilder::new(item_fn, "/options".to_string());
		let (client_fn, server_fn) = builder.build();

		// Check client function has OPTIONS method
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::options)
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

		let builder = ServerActionBuilder::new(item_fn, "/head".to_string());
		let (client_fn, server_fn) = builder.build();

		// Check client function has HEAD method
		let client_str = quote!(#client_fn).to_token_stream().to_string();
		expect(client_str).to_contain(
			&quote!(CallServerAction::head).to_token_stream().to_string(),
		);

		// Check server function uses JsonQuery extractor for HEAD (bodyless)
		let first_param = server_fn.sig.inputs.first().unwrap();
		let param_str = quote!(#first_param).to_token_stream().to_string();
		expect(&param_str)
			.to_contain(&quote!(JsonQuery).to_token_stream().to_string());
	}
}
