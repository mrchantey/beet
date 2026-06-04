//! Emits the typed `routes::` module for compile-time-checked links.
//!
//! Given the route paths of every pages collection, generates a module tree of
//! path functions so links can be written as `routes::docs::index()` and fail
//! to compile if a route moves or its dynamic params change.

use beet_core::prelude::*;
use beet_net::prelude::*;
use heck::ToSnakeCase;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use syn::Ident;
use syn::Item;
use syn::ItemFn;

/// A node in the route tree keyed by path segment.
#[derive(Default)]
struct TreeNode {
	/// Sanitized segment name, used as the module/function identifier.
	ident: String,
	/// The full route path of a route at this exact node, if any.
	route: Option<SmolPath>,
	/// Child nodes by segment.
	children: Vec<TreeNode>,
}

impl TreeNode {
	/// Inserts a route path into the tree, creating nodes as needed.
	fn insert(&mut self, path: &SmolPath) {
		let mut node = self;
		for seg in path.segments() {
			let ident = PathPatternSegment::new(seg).name().to_snake_case();
			let idx = node
				.children
				.iter()
				.position(|child| child.ident == ident)
				.unwrap_or_else(|| {
					node.children.push(TreeNode { ident, ..default() });
					node.children.len() - 1
				});
			node = &mut node.children[idx];
		}
		node.route = Some(path.clone());
	}
}

/// Builds the `routes` module item from a list of route paths.
pub(crate) fn emit_route_tree(route_paths: &[SmolPath]) -> Result<Item> {
	let mut root = TreeNode {
		ident: "routes".into(),
		..default()
	};
	for path in route_paths {
		root.insert(path);
	}
	mod_tree(&root, true)
}

/// Recursively builds the module/function items for a tree node.
fn mod_tree(node: &TreeNode, is_root: bool) -> Result<Item> {
	let own_fn = node
		.route
		.as_ref()
		.map(|path| path_fn(node, path))
		.transpose()?;

	if is_root || !node.children.is_empty() {
		let ident = Ident::new(&node.ident, Span::call_site());
		let children = node
			.children
			.iter()
			.map(|child| mod_tree(child, false))
			.collect::<Result<Vec<_>>>()?;
		Ok(syn::parse_quote! {
			#[allow(unused, missing_docs)]
			pub mod #ident {
				use super::*;
				#own_fn
				#(#children)*
			}
		})
	} else {
		Ok(own_fn
			.map(|func| func.into())
			.unwrap_or(Item::Verbatim(TokenStream::default())))
	}
}

/// Builds the typed path function for a route at the given path.
///
/// Nodes with children expose their own route as `index`, leaf nodes use the
/// last path segment. Static paths return `&'static str`; dynamic paths take a
/// `&str` per dynamic segment and return a `String`.
fn path_fn(node: &TreeNode, path: &SmolPath) -> Result<ItemFn> {
	let ident = if node.children.is_empty() {
		Ident::new(&node.ident, Span::call_site())
	} else {
		Ident::new("index", Span::call_site())
	};
	let pattern = PathPattern::new(path)?;

	if pattern.is_static() {
		let path = path.with_leading_slash();
		Ok(syn::parse_quote! {
			pub fn #ident() -> &'static str { #path }
		})
	} else {
		let dyn_idents = pattern
			.iter()
			.filter(|seg| !seg.is_static())
			.map(|seg| Ident::new(seg.name(), Span::call_site()))
			.collect::<Vec<_>>();
		let format_str = format!(
			"/{}",
			pattern
				.iter()
				.map(|seg| if seg.is_static() {
					seg.name().to_string()
				} else {
					"{}".to_string()
				})
				.collect::<Vec<_>>()
				.join("/")
		);
		Ok(syn::parse_quote! {
			pub fn #ident(#(#dyn_idents: &str),*) -> String {
				format!(#format_str, #(#dyn_idents),*)
			}
		})
	}
}
