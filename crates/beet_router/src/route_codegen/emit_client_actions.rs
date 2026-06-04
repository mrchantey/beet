//! Emits client-side callers for server actions.
//!
//! For each server-action handler this generates an `async fn` that builds a
//! [`Request`](beet_net::prelude::Request) to the action's path, sends it via
//! the configured server URL, and deserializes the typed response. The callers
//! are organized into a module tree mirroring the route paths.

use crate::prelude::*;
use crate::route_codegen::syn_utils::action_output_ty;
use crate::route_codegen::syn_utils::inner_generic;
use beet_core::prelude::*;
use beet_net::prelude::*;
use heck::ToSnakeCase;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;
use syn::Item;
use syn::ItemFn;

/// How a client caller transports its single input argument.
enum ClientInput {
	/// No input argument.
	None,
	/// `Json<T>` extractor: send `T` as a JSON body.
	Json(syn::Type),
	/// `QueryParams<T>` extractor: send `T` as a URL-encoded query string.
	Query(syn::Type),
}

/// A node in the client-action module tree, keyed by path segment.
#[derive(Default)]
struct ActionNode {
	ident: String,
	methods: Vec<(SmolPath, RouteMethod)>,
	children: Vec<ActionNode>,
}

impl ActionNode {
	fn insert(&mut self, path: &SmolPath, method: RouteMethod) {
		let mut node = self;
		for seg in path.segments() {
			let ident = PathPatternSegment::new(seg).name().to_snake_case();
			let idx = node
				.children
				.iter()
				.position(|child| child.ident == ident)
				.unwrap_or_else(|| {
					node.children.push(ActionNode { ident, ..default() });
					node.children.len() - 1
				});
			node = &mut node.children[idx];
		}
		node.methods.push((path.clone(), method));
	}
}

/// Builds the top-level client-action module items from a list of route files.
pub(crate) fn emit_client_actions(files: &[RouteFile]) -> Result<Vec<Item>> {
	let mut root = ActionNode::default();
	for file in files {
		for method in file.methods() {
			root.insert(&file.route_path, method.clone());
		}
	}
	root.children.iter().map(client_mod).collect()
}

/// Builds the module item for a single client-action tree node.
fn client_mod(node: &ActionNode) -> Result<Item> {
	let ident = Ident::new(&node.ident, Span::call_site());
	let funcs = node
		.methods
		.iter()
		.map(|(path, method)| client_fn(path, method))
		.collect::<Result<Vec<_>>>()?;
	let children = node
		.children
		.iter()
		.map(client_mod)
		.collect::<Result<Vec<_>>>()?;
	Ok(syn::parse_quote! {
		#[allow(missing_docs)]
		pub mod #ident {
			#[allow(unused_imports)]
			use super::*;
			#(#funcs)*
			#(#children)*
		}
	})
}

/// Builds the client caller function for a single server-action handler.
fn client_fn(path: &SmolPath, method: &RouteMethod) -> Result<ItemFn> {
	let fn_ident =
		Ident::new(&method.method.to_string_lowercase(), Span::call_site());
	let http = method.method.self_token_stream();
	let route = path.with_leading_slash();
	let out_ty = action_output_ty(&method.item);
	let docs = doc_attrs(&method.item);

	let (args, build): (TokenStream, TokenStream) =
		match client_input(&method.item) {
			ClientInput::None => (quote! {}, quote! {}),
			ClientInput::Json(ty) => {
				(quote! { input: #ty }, quote! { .with_json_body(&input)? })
			}
			ClientInput::Query(ty) => (
				quote! { input: #ty },
				quote! { .with_query_string(&QueryParams(input).encode()?) },
			),
		};

	Ok(syn::parse_quote! {
		#(#docs)*
		#[allow(unused)]
		pub async fn #fn_ident(#args) -> Result<#out_ty> {
			server_action_request(#http, #route)
				.with_accept(MediaType::Json)
				#build
				.send()
				.await?
				.into_result()
				.await?
				.json()
				.await
		}
	})
}

/// Classifies a handler's input extractor for the client caller.
fn client_input(item: &ItemFn) -> ClientInput {
	let Some(syn::FnArg::Typed(pat_type)) = item.sig.inputs.first() else {
		return ClientInput::None;
	};
	// unwrap the ActionContext<X> wrapper to find the extractor X
	let Some(extractor) = inner_generic(&pat_type.ty, "ActionContext") else {
		return ClientInput::None;
	};
	if let Some(inner) = inner_generic(&extractor, "Json") {
		ClientInput::Json(inner)
	} else if let Some(inner) = inner_generic(&extractor, "QueryParams") {
		ClientInput::Query(inner)
	} else {
		ClientInput::None
	}
}

/// Collects `#[doc]` attributes from a handler for the client caller.
fn doc_attrs(item: &ItemFn) -> Vec<syn::Attribute> {
	item.attrs
		.iter()
		.filter(|attr| attr.path().is_ident("doc"))
		.cloned()
		.collect()
}
